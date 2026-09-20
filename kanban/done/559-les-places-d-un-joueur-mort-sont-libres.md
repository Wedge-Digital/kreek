# Les places d'un joueur mort sont libres

**Priorité : haute — un coach ne peut plus renuméroter ni réordonner son effectif après une mort**
**Épic :** aucune — une correction, livrable d'un bloc
**Dépend de :** rien
**Fichiers :**
`src/app/players/domain/player.rs`,
`src/app/players/use_cases/update_roster_use_case.rs`,
`src/app/players/io/repository/projection_repository.rs`, `src/app/players/ports.rs`,
`tests/e2e/test_effectif_apres_une_mort.py` *(nouveau)*, `tests/impact-map.toml`

## L'objectif

Le numéro et le rang d'un joueur mort sont libres : un coach peut les donner à
un vivant depuis « Modifier l'effectif », et le numéro automatique d'un
journalier peut les reprendre.

## Ce qui l'a fait naître

Deux refus signalés par les coachs, « deux joueurs portent le même numéro » et
« deux joueurs occupent le même rang », alors que l'écran ne montrait aucun
doublon. Le second joueur était **mort**.

Une mort est un statut de participation, `Dead`, pas un départ : le joueur
reste membre de l'effectif, avec son numéro et son rang. L'écran d'édition ne le
montre pas — il lit `find_alive_by_team_id` — mais le use case `update_roster`
recharge l'effectif depuis l'event store et ne filtrait que sur l'appartenance.
Le mort y figurait, et `ensure_no_duplicates` comptait ses deux valeurs comme
prises. Rang et numéro sont deux symptômes du même filtre manquant.

La numérotation automatique des journaliers avait le même défaut :
`jerseys_by_team_id` comptait le maillot du mort, et `premier_libre` sautait
son numéro.

## Le changement

**Le domaine dit qui tient une place.** `Player::occupe_une_place` répond vrai
pour un vivant, blessé compris, et faux pour un mort. C'est le vocabulaire de
`teams`, où `SquadPresence::occupe_une_place` exclut déjà un perdu du plafond de
seize et du quota de son poste ; il vaut ici pour ce que `players` possède, les
maillots et l'ordre d'affichage.

**Le use case l'applique.** Les joueurs qui ne tiennent plus leur place ne
versent ni leur numéro ni leur rang dans les ensembles d'unicité. Le reste ne
change pas : un renvoi reste un départ, un journalier reste exclu de l'édition,
un blessé tient toujours sa place.

**La requête des maillots pris l'applique en SQL.** `participation_status <>
'Dead'`, à côté du filtre d'appartenance qui existait déjà.

## Ce que la carte ne couvre pas

Les quotas de recrutement : ils excluaient déjà les morts, par `teams`.

## Tests

Unitaires : le prédicat sur un vivant, un blessé et un mort ; le use case qui
accepte de reprendre le numéro d'un mort, son rang, et refuse toujours celui
d'un vivant ; le dépôt dont la liste des maillots pris libère celui du mort.
Les trois tests de reprise échouent sans la correction.

E2E, `test_effectif_apres_une_mort.py` : un match publié où un joueur meurt,
puis une modification d'effectif qui donne son numéro à un coéquipier,
acceptée ; la même modification visant le numéro d'un vivant, refusée.

## Terminé quand

Après la mort d'un joueur, « Modifier l'effectif » accepte de donner son
numéro et son rang à un coéquipier, et un journalier reçoit ce numéro s'il est
le premier libre.
