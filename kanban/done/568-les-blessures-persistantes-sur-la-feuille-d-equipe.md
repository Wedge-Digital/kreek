# Les blessures persistantes sur la feuille d'équipe

**Priorité : moyenne**
**Épic :** aucune — livrable d'un bloc
**Dépend de :** rien
**Maquette :** `assets/rawpages/html/app-team-sheet-persistent-injuries.html`
**Fichiers :**
`migrations/<horodatage>_players_proj_persistent_injuries.sql` *(nouveau)*,
`src/app/players/io/repository/player_repository.rs`,
`src/app/players/io/repository/projection_repository.rs`, `src/app/players/ports.rs`,
`src/infrastructure/data_migrations/m006_blessures_persistantes.rs` *(nouveau)*,
`src/infrastructure/data_migrations/mod.rs`,
`src/app/players/io/web/widgets/player_table_widget.rs`,
`src/app/players/io/web/templates/player-table-fragment.html`,
`assets/static/css/widgets/players-widget.css`,
`tests/e2e/test_feuille_blessures_persistantes.py` *(nouveau)*,
`tests/e2e/test_player_availability_after_injury.py`, `tests/impact-map.toml`

## L'objectif

Une colonne « BP », juste avant SPP dans le tableau des joueurs, donne le nombre
de **blessures persistantes** de chaque joueur. Une blessure persistante naît
d'une blessure sérieuse, et d'elle seule. Tiret quand il n'en a aucune.

## Ce qui l'a fait naître

Le domaine compte ces blessures — `career_persistent_injuries`, incrémenté sur
`BlessureSerieuse` et sur elle seule (`player.rs:556`), décrémenté quand le
rapport est dépublié (`player.rs:871`). Mais le compteur ne vit que dans
l'agrégat : seule la page de debug l'affiche, et le coach ne le voit nulle part.

## Le changement

**Le compteur entre dans la projection.** Le tableau lit sa ligne dans
`players_proj` ; le compteur doit y être, et non être relu dans l'agrégat — une
lecture ne rejoue pas l'event store (la dette existante est la carte 569).

- Migration SQL : `players_proj.persistent_injuries SMALLINT NOT NULL DEFAULT 0`.
- **Recalculé depuis l'agrégat, dans la transaction de l'événement**, sur le
  modèle exact de `recompute_stat_deltas` : sur `InjurySustained` et sur
  `MatchImpactReverted`. Recalculer plutôt qu'incrémenter rend la colonne
  insensible aux rejeux et aux corrections — une dépublication la fait
  redescendre sans qu'on ait à savoir de combien.
- **Migration de données `m006`** pour les joueurs existants : rejoue chaque
  joueur et pose son compteur, dans la transaction de sa marque. Une migration
  SQL ne peut pas rejouer des événements.
- `PlayerProjection` (`ports.rs`) et les deux lectures de
  `projection_repository.rs` portent le champ.

**L'affichage.**

- `PlayerRowVm.persistent_injuries: u16`, lu dans la projection.
- Gabarit : en-tête `BP` (`title="Blessures persistantes"`) avant SPP ; tiret
  pour zéro, le nombre sinon ; tiret au pied, comme SPP — une somme de
  blessures entre joueurs ne dirait rien. Le commentaire qui compte les
  colonnes passe de douze à treize.
- CSS : largeur d'une caractéristique ; rouge et gras si non nul. Pour un
  absent, `.player-bp` rejoint la liste des cellules barrées (carte 489) : le
  compteur suit le reste de la ligne.
- La page « Modifier l'effectif » rend le même tableau
  (`roster_edition_controller.rs`) : la colonne y apparaît, en lecture seule
  comme SPP.

## Ce que la carte ne couvre pas

- Les séquelles : déjà comptées dans les caractéristiques affichées.
- La guérison d'une blessure persistante : non modélisée.
- Le mobile : la colonne n'apparaît qu'en faisant défiler le tableau, comme SPP
  et Valeur. Limite acceptée.
- Le passage des caractéristiques et des SPP du tableau sur la projection :
  carte 569.

## Tests

Unitaires :
- la projection : une blessure sérieuse pose 1, un « amoché » laisse 0, une
  séquelle laisse 0, `MatchImpactReverted` ramène à 0 ;
- `m006` pose le compteur d'un joueur blessé avant la migration ;
- le rendu : tiret pour 0, le nombre sinon ; la cellule d'un absent est barrée.

E2E, `test_feuille_blessures_persistantes.py` : un match publié où un joueur
subit une blessure sérieuse et un autre un « amoché » ; la feuille montre BP = 1
pour le premier, un tiret pour le second ; le rapport dépublié, le premier
revient au tiret. Le tiret du pied est aligné sous la colonne BP.

`test_player_availability_after_injury.py` repère SPP et Valeur par
`th:nth-child(12)` et `(13)`, et le tiret du pied par `tfoot .player-foot-dash` :
la colonne décale les deux en-têtes et ajoute un second tiret. Les sélecteurs
passent par les classes de colonne.

## Terminé quand

Un joueur qui a subi une blessure sérieuse montre BP = 1 sur la feuille
d'équipe, un « amoché » n'y laisse rien, et la dépublication du rapport efface
le compteur — sans que la feuille relise un seul événement pour cette colonne.
