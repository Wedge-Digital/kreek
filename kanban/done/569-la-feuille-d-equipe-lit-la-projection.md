# La feuille d'équipe lit la projection

**Priorité : moyenne — dette, aucun symptôme visible**
**Épic :** aucune
**Dépend de :** 568 (même tableau, mêmes fichiers — à faire après)
**Fichiers :**
`src/app/players/io/web/widgets/player_table_widget.rs`,
`migrations/<horodatage>_players_proj_spp_remaining.sql` *(nouveau)*,
`src/app/players/io/repository/player_repository.rs`,
`src/app/players/io/repository/projection_repository.rs`, `src/app/players/ports.rs`,
`src/infrastructure/data_migrations/m007_solde_spp.rs` *(nouveau)*,
`src/infrastructure/data_migrations/mod.rs`,
`tests/e2e/test_feuille_caracteristiques.py` *(nouveau)*, `tests/impact-map.toml`

## L'objectif

Le tableau des joueurs de la feuille d'équipe se lit **entièrement** dans
`players_proj`. `resolve_team_derived` disparaît : afficher une feuille ne
rejoue plus aucun événement.

## Ce qui l'a fait naître

Une ligne du tableau vient de la projection — nom, numéro, poste, compétences,
valeur, statut — sauf deux valeurs : les caractéristiques et le solde de SPP.
Pour elles, `resolve_team_derived` recharge le flux d'événements de chaque
joueur de l'équipe et hydrate les agrégats, **à chaque affichage**. En CQRS, une
lecture interroge la projection ; elle ne rejoue pas l'event store.

Le raccourci est en partie périmé : depuis la carte 303, `players_proj` porte
`ma_delta` … `av_delta`, tenus dans la transaction de l'événement. Le widget des
journaliers et `squad_adapter` de `teams` les lisent déjà ; la feuille d'équipe
recalcule encore ses caractéristiques depuis les agrégats. Seul le solde de SPP
n'existe aujourd'hui que dans l'agrégat (`Player::spp_remaining`).

## Le changement

**Les caractéristiques** : base du poste (catalogue `references`, déjà lu pour
les compétences de base) plus les deltas de la projection.
`resolve_stats_from_deltas` rejoint `resolve_stats` dans
`player_stats_service.rs` et en réutilise les primitives — pas une seconde
copie du calcul. Personne ne compose aujourd'hui base et deltas depuis la
projection : `squad_adapter` ne fait que **transporter** les deltas vers
`teams`.

**Le plancher à zéro.** `resolve_stats` sature à zéro **après chaque**
ajustement ; une somme de deltas ne sature qu'une fois. Les deux ne divergent
que si une caractéristique a traversé zéro en route — impossible en jeu. Un test
unitaire pose l'écart en clair.

**Le solde de SPP** entre dans la projection :
- migration SQL : `players_proj.spp_remaining INTEGER NOT NULL DEFAULT 0` ;
- recalculé depuis `Player::spp_remaining()` dans la transaction de chaque
  événement qui le touche — gains de match, achat de compétence, augmentation
  de caractéristique, leurs annulations, customisations — sur le modèle de
  `recompute_persistent_injuries` ;
- **les événements déclencheurs sont un `match` sans joker** sur
  `PlayerDomainEvent` : chaque variant est nommé et classé. Un événement ajouté
  demain ne compile pas tant qu'on n'a pas décidé s'il touche le solde — une
  liste oubliée afficherait un solde faux, sans un bruit ;
- migration de données `m007` pour les joueurs existants.

Puis `resolve_team_derived` et `PlayerDerived` sont **supprimés**, après avoir
listé leurs consommateurs (règle 4).

## Les tests e2e ne bougent pas — c'est la preuve

La carte change **d'où** vient chaque valeur, pas **ce qui** s'affiche. Les
tests e2e qui lisent une ligne de joueur sur la feuille d'équipe doivent donc
passer **sans aucune modification** avant et après : c'est ce qui prouve que la
projection donne les mêmes chiffres que les agrégats. Un test qu'il faudrait
retoucher pour passer signale un écart, pas un test à mettre à jour.

Ce qui existe et ne doit pas bouger :
- `test_player_spp_spending.py::test_la_colonne_spp_de_la_liste_est_le_solde_pas_le_cumul`
  — la colonne SPP égale la réserve de la fiche joueur, avant et après un
  achat ;
- `test_player_availability_after_injury.py` — ligne d'un absent, sous-total,
  alignement du pied ;
- `test_feuille_blessures_persistantes.py` (carte 568).

**Ce qui manque, et que la carte écrit d'abord.** Aucun test e2e ne lit les
**caractéristiques** d'une ligne de la feuille d'équipe —
`test_stat_increase_updates_stat_and_reserve` les lit sur la fiche joueur.
`test_feuille_caracteristiques.py` est donc écrit **avant toute modification du
code**, vérifié vert sur le code actuel, **commité seul** — l'historique montre
que le témoin précède le changement — puis laissé intact :

- une augmentation achetée en SPP : la caractéristique de la ligne monte d'un
  cran, et le solde SPP de la ligne baisse ;
- une séquelle subie en match : la caractéristique de la ligne baisse d'un cran ;
- le rapport dépublié : la séquelle disparaît de la ligne ;
- une customisation de caractéristique par un commissaire : la ligne la reflète,
  et son retrait la retire.

## Ce que la carte ne couvre pas

- La fiche joueur (`player_detail`), qui hydrate un seul agrégat : une écriture
  y est en cours, et c'est le côté commande.
- Les autres lectures d'agrégats hors de la feuille d'équipe.

## Tests

Unitaires : la projection du solde SPP sur chaque événement qui le touche, et sa
remise à jour après annulation ; `m007` ; la composition base + deltas.

E2E : ceux de la section ci-dessus — les existants inchangés, le nouveau écrit
en premier.

## Terminé quand

`resolve_team_derived` n'existe plus, la feuille d'équipe ne lit aucun flux
d'événements, et les tests e2e de la feuille passent sans qu'une ligne en ait
été modifiée depuis le début de la carte.
