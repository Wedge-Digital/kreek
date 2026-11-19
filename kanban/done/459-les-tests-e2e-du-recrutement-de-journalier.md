# Les tests E2E du recrutement de journalier

**Épic :** E15 — Recruter un journalier
**Ordre :** 5 · **Dépend de :** 458
**Conception :** `docs/specs/embaucher-un-journalier/ecran-de-recrutement/07-integration.md`

## Objectif

Prouver dans un navigateur une chaîne qui traverse **trois BCs et deux bus
d'événements**, et qu'aucun test unitaire ne voit d'un bout à l'autre.

Fichier : `tests/e2e/test_journeyman_recruitment.py`.

## Les scénarios

| Test | Ce qu'il prouve |
|---|---|
| `test_le_panneau_est_absent_sans_journalier` | le cas le plus fréquent |
| `test_un_journalier_apparait_apres_un_match` | `match_report → teams → players` — **il porte la dette de la 455** |
| **`test_le_journalier_recrute_reste_dans_l_effectif`** | **le test qui compte** |
| `test_le_journalier_non_recrute_disparait` | la décision 13, bout en bout |
| `test_le_prix_se_decompose_avec_une_amelioration` | « 65 + 20 » à l'écran |
| `test_le_meme_journalier_ne_s_ajoute_pas_deux_fois` | la règle propre |
| `test_seize_dont_journaliers_autorise_le_recrutement` | le cas qui donne son sens au plafond |

## Le préalable : un effectif incomplet

**La suite e2e n'exerce aujourd'hui aucun journalier**, et ne le peut pas :
`collect_journeymen` n'en crée que si l'équipe a moins de onze joueurs
disponibles, or toutes les équipes de `build_full_competition` ont un effectif
complet. Vérifié après une exécution complète — zéro ligne `Journeyman` en base.

Aucun des sept scénarios ci-dessous n'existe sans ce préalable. Il faut donc
d'abord une équipe à effectif incomplet dans le jeu de données : soit une
fixture qui renvoie des joueurs avant le match, soit une équipe construite avec
moins de onze.

C'est le premier travail de cette carte, avant tout scénario.

## La dette héritée de la carte 455

Deux tests que la 455 n'a pas pu écrire lui reviennent :
`un_journalier_aligne_devient_un_joueur` et
`les_actions_du_rapport_pointent_le_joueur_reel`. Une carte de socle ne peut
pas prouver ce qu'elle rend seulement possible — la chaîne traverse trois BCs
et deux bus, et n'est observable qu'en navigateur.

`test_un_journalier_apparait_apres_un_match` couvre le premier. Le second
demande une assertion de plus : que l'action enregistrée dans le rapport porte
**le même identifiant** que le joueur créé dans `players`. C'est ce qui prouve
que les deux mondes désignent la même entité, et non deux copies.

## Celui qui vaut le prix de la suite

**`test_le_journalier_recrute_reste_dans_l_effectif`.**

Il traverse tout : la création à l'ouverture du rapport, le match, la
publication, le recrutement au panier, la validation de phase — et vérifie qu'il
est **toujours là** quand les autres sont partis.

C'est le seul qui prouve que l'ordre du lot d'événements tient : le basculement
en `Active` et le passage en `Dismissals` sont dans le même lot, dans cet ordre.
Si quelqu'un déplace un jour le ménage avant la validation, ce test échoue —
alors que le code compilerait parfaitement et perdrait un joueur qu'on vient de
payer.

## La non-régression, qui compte autant

**`tests/e2e/test_recruitment_phase.py` doit rester vert sans une
modification.** Ses huit cas ne concernent aucun journalier : ils mesurent donc
que le recrutement ordinaire fonctionne exactement comme avant.

Si l'un d'eux doit être adapté pour passer, c'est le signe que la carte 457 a
changé un comportement qu'elle ne devait pas toucher — le plafond, le panier ou
la trésorerie.

## Le piège de la fenêtre non câblée

Le catalogue et le panier arrivent par `hx-get` et se rechargent sur
`basketChanged`. Tout clic sur du contenu fraîchement injecté passe par
`cliquer_quand_cable` (`tests/e2e/htmx_helpers.py`).

**Pas de `sleep`.** Une durée fixe n'a aucune marge sur une machine chargée, et
c'est exactement là que la suite échouait.

## Ce que les tests ne couvrent pas

- **L'annulation d'un rapport** : elle demande de défaire un rapport en cours,
  ce que la carte 456 couvre unitairement.
- **Le garde-fou de `players`** — un recrutement sur un joueur déjà perdu : il
  demande de provoquer une course que le navigateur ne sait pas créer. Carte
  456.
- **Les quatre requêtes SQL** de la carte 454 : leurs tests d'intégration valent
  mieux qu'un parcours d'écran.

## Checklist

- [x] Le préalable : une équipe à effectif incomplet, par une blessure en match
- [x] Sept scénarios
- [x] `cliquer_quand_cable_locator` sur le contenu injecté
- [x] Aucun `sleep` — on attend le **fait**, jamais une durée
- [x] `test_recruitment_phase.py` vert **sans modification**
- [x] L'entrée dans `tests/impact-map.toml` (axe 8)
- [x] `make e2e` — 363 passés, 7 ignorés

## Ce qui a été fait

### Le préalable, levé par le jeu réel

Une équipe ne peut pas naître incomplète : `MIN_PLAYERS_FOR_SUBMISSION = 11`
l'interdit à la soumission. Le fixture joue donc **deux matchs** — le premier
blesse un titulaire, le second n'a plus que dix alignables et appelle un
journalier. `_record_injury`, éprouvé par
`test_player_availability_after_injury`, fait la première moitié.

C'est aussi ce qui explique que la suite n'ait jamais exercé un journalier
avant cette carte, et que quatre cartes aient été livrées sans qu'un seul
n'existe.

### Deux maillons manquants, que rien d'autre ne voyait

**C'était la raison d'être de cette carte, et elle l'a remplie deux fois.**

| Carte | Ce qui manquait | Effet |
|---|---|---|
| `455` | `init_temp_players_use_case` n'émettait pas sur le bus interne | bras de publisher mort — **aucun journalier jamais créé** |
| `457` | `JourneymanRecruited` n'avait pas de bras dans `to_app_event` | **le coach payait son journalier et le perdait** à la clôture |

Le second est le plus instructif. Le joker `_ => None` du publisher l'avalait
en silence — exactement ce contre quoi son propre commentaire met en garde. Et
**deux commentaires affirmaient que ce maillon existait**, dont un qui nommait
un listener inexistant : la chaîne avait été décrite comme complète sans avoir
jamais été parcourue.

### Une case vide dans le découpage de l'épic

L'épic dit *« le recrutement ne fait que basculer son `membership` en
`Active` »*. La `456` fait disparaître, la `457` tient le domaine de `teams`, la
`458` l'écran — **aucune ne portait la bascule**. Elle est tombée entre trois
cartes, et seul le parcours complet pouvait le montrer.

Le correctif est rattaché à la `457`, qui possède `JourneymanRecruited` et
aurait dû le faire sortir de son BC.

### Ce que les tests ne couvrent pas, et pourquoi

**Le plafond à seize dont des journaliers** n'a pas de scénario de navigateur :
il demanderait de faire recruter cinq joueurs à la main par Playwright pour
éprouver une règle que
`seize_dont_trois_journaliers_autorisent_le_recrutement` tient déjà
unitairement. Le montage aurait coûté plus que ce qu'il prouve.

**Le doublon au panier** non plus, pour la même raison : `add_journeyman` rend
`JourneymanAlreadyInBasket`, et le panneau retire le journalier de la liste dès
qu'il y entre — il n'y a pas de second bouton à cliquer.

**La décomposition du prix** est vérifiée sur le cas nu — « aucune » — parce que
le parcours ne produit pas de journalier amélioré : le rapport ne lui donne
aucune action. Le cas décomposé est tenu par
`le_prix_se_decompose_au_dela_du_tarif`, sur le view model.

## Ce que l'épic laisse ouvert

**L'axe 12 de `check-arch` ne voit ni l'un ni l'autre de ces deux trous.** Il
vérifie qu'une émission passe par `emettre()` ou `publier()` — jamais qu'un
événement destiné à sortir du BC est bien émis, ni qu'un bras de publisher est
atteignable.

C'est le même piège que l'épic E11 a documenté trois fois : du code qui a l'air
branché et ne l'est pas. Il mérite sa propre carte.
