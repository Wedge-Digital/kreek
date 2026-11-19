# Un joueur indisponible se voit dans la liste

**Priorité : moyenne** — la donnée existe, l'écran la tait
**Dépend de :** rien · **Suite de la carte 488** · **Sans épic**
**Demandée par :** l'utilisateur · **Maquette validée** avant écriture

## Le constat

Le tableau des joueurs de la feuille d'équipe affiche tout le monde de la même
manière. Un joueur blessé qui manquera le prochain match s'y lit exactement comme
un joueur disponible.

La carte 488 l'avait déjà noté en passant — *« aucun n'affiche de statut »* — et
s'en est tenue à faire disparaître les morts de la liste. Les blessés, eux, sont
restés indistinguables.

## Ce que ça ne coûte pas

**Aucune donnée nouvelle.** `participation_status` vit déjà dans `players_proj`,
le dépôt le lit, et `PlayerProjection` le porte (`ports.rs:50`). Il s'arrête au
view model. Pas de migration, pas d'événement, pas de listener.

## Le rendu, validé sur maquette

`assets/rawpages/html/app-joueur-indisponible.html`

**La ligne entière est barrée** — maillot, nom, poste, mots-clefs, les cinq
caractéristiques, SPP et valeur. Le texte passe en gris : la ligne recule sans
devenir illisible.

**Les pastilles de compétences ne sont pas barrées, elles pâlissent.** Ce ne sont
pas du texte courant mais des étiquettes à fond arrondi ; une barre les traverse
de bout en bout et se confond avec leur forme. Elles perdent leur couleur de
catégorie et reculent avec la ligne.

**Un repère à droite du nom dit pourquoi.** Un barré nu n'explique rien : un
coach qui ouvre la feuille sans contexte ne sait pas s'il s'agit d'une blessure,
d'une suspension ou d'un défaut d'affichage.

## Les statuts couverts

Le domaine en connaît quatre (`players/domain/match_impact.rs:33`).

| Statut | Dans la liste | Traitement |
|---|---|---|
| `Available` | oui | rien |
| `MissingNextGame` | oui | barré · « Manque le prochain match » |
| `Retired` | oui | barré · « A pris sa retraite » |
| `Dead` | **non** | exclu par `find_alive_by_team_id` (carte 488) |

**`Retired` est traité bien qu'il ne soit jamais posé.** Aucun code du domaine ne
l'atteint, et la base en compte zéro. Mais `squad_adapter.rs:100` le range déjà
parmi les indisponibles : ne pas le barrer ferait dire à l'écran l'inverse de ce
que `teams` calcule. Une ligne de correspondance coûte moins que cette
divergence.

## Trois choses que la maquette a tranchées

**En mode édition, le barré s'efface des champs.** La table bascule en saisie :
maillot et nom deviennent des `<input>`. Un champ barré se lit mal et suggère que
la valeur est condamnée, alors qu'on est en train de la modifier. Le barré ne
porte que sur les cellules en lecture ; le repère reste.

**Un `enum`, pas un booléen.** Le gabarit n'a pas à connaître les quatre statuts
du domaine ni à décider lesquels comptent comme une absence — c'est une règle
métier. Mais un `bool` ne distinguerait pas les deux repères. L'énumération dit à
la vue exactement ce qu'elle doit savoir.

**Le mobile.** La table défile déjà dans son conteneur et resserre ses cellules
sous 768 px ; elle ne masque aucune colonne. Le repère élargit la colonne du nom,
donc le défilement horizontal. À mesurer, et à réduire à sa seule icône si le
coût est réel.

## Ce que la carte ne fait pas

**Elle ne touche pas aux autres écrans.** Le sélecteur de joueurs d'un rapport de
match et les comptes par poste de `match_report` montrent les mêmes joueurs sans
dire leur statut ; c'est le même manque, ce n'est pas le même écran, et la carte
488 les avait déjà laissés de côté.

**Elle ne change aucune règle de jeu.** Un joueur barré reste sélectionnable,
comptabilisé et modifiable exactement comme avant : la carte est un affichage.

## Le mobile : ce qui a été mesuré, et ce que ça ne corrige pas

La demande était de **valider** la compatibilité mobile. La mesure a dit autre
chose que ce que j'attendais.

**Le libellé du repère ne coûte rien au défilement.** À 390 px, la table défile
de 331 px avec ou sans lui, et la colonne du nom fait 78 px dans les deux cas.
Mon masquage n'évitait donc pas l'élargissement que je lui prêtais.

**Ce qu'il évite vraiment**, c'est un repère de 160 px là où 24 suffisent. Parce
que la cellule du nom **débordait déjà** : à 390 px, le nom occupe 158 px dans
une cellule de 78 — un dépassement de 88 px, **identique sur une ligne barrée et
sur une ligne saine**. Le repère s'ajoute derrière un contenu qui débordait avant
lui.

**C'est donc un défaut mobile préexistant de la table, pas de cette carte.** Le
masquage limite l'ajout ; il ne répare pas le fond. Ça mérite sa propre carte :
douze colonnes dans un `overflow-x: auto`, et une cellule dont le contenu sort
de ses bornes.

## Un test qui ne prouvait rien, corrigé

La première version du test mobile asseyait « la page ne déborde pas
latéralement ». **Creux** : la table vit dans un `overflow-x: auto`, donc
l'assertion est vraie que le libellé soit affiché ou non — elle passait aussi
bien avec le défaut qu'avec la correction. La falsification l'a montré : le test
échouait sur le masquage, jamais sur le débordement.

Il mesure désormais la largeur du repère — 24 px attendus, 160 avec le libellé —
un chiffre qui distingue réellement les deux états.

## Hors carte : un test qui bloquait `make e2e`

`test_renommer_une_competition_depuis_les_parametres` cliquait sur un panneau
injecté par htmx **sans attendre son câblage**. Le clic se perd, aucune requête
ne part, et l'assertion intermédiaire passe quand même — l'`input` contient
encore ce que `fill` y a mis. C'est le rechargement qui découvre que rien n'a été
enregistré, et son message accuse alors le mauvais coupable.

Le test voisin du **même fichier** posait déjà cette attente sur le **même
sélecteur**, trente-six lignes plus bas. La correction n'avait pas été
généralisée.

Une ligne a été ajoutée pour débloquer la suite. **Le sujet reste entier** : 59
`page.click` sur 60 n'attendent pas le câblage, et rien ne distingue ceux qui
cliquent sur du contenu injecté de ceux qui cliquent après une navigation
complète.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `chaque_statut_donne_son_absence` | la correspondance des quatre statuts, unitaire — `Dead` compris, qui rend `None` |
| `un_statut_inconnu_ne_barre_rien` | l'échec ouvert : un statut non reconnu laisse la liste lisible |
| `les_deux_absences_ne_se_disent_pas_pareil` | libellés et icônes distincts — la raison d'être de l'`enum` |
| `test_le_joueur_indisponible_est_barre_dans_la_liste` | le **style calculé**, e2e : la classe seule passerait sans qu'aucune règle ne l'atteigne (carte 487) |
| `test_les_autres_joueurs_ne_sont_pas_barres` | le barré ne déborde pas sur l'effectif sain |
| `test_le_repere_se_reduit_a_son_icone_en_mobile` | 24 px sous 768, 160 avec le libellé |

## Checklist

- [x] `Absence` sur `PlayerRowVm`, alimenté depuis `participation_status`
- [x] Le gabarit pose la classe et rend le repère
- [x] CSS : barré, gris, pastilles pâlies à 0,75, repère — scopé `players-widget`
- [x] Le mode édition n'est pas barré
- [x] Mesure mobile — le repère tombe de 160 à 24 px ; le débordement de la cellule préexiste
- [x] Six tests (3 unitaires dans `player_table_widget.rs`, 3 e2e dans
      `test_player_availability_after_injury.py`), chacun falsifié
- [x] `make lint`, `make test` (1629), `make check-arch` (17 axes), `make e2e` (**351**, 0 échec)
