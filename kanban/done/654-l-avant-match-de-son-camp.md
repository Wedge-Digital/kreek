# L'avant-match de son camp

**Épic :** E22 — Le mode « Chacun son équipe » · **Ordre :** 3 · **Dépend de :** 652, 653
**Conception :** `docs/specs/saisie-mobile/E22-cadrage.md` (R1, R4, § 3, § 5, § 12.6, § 12.7)
**Maquette :** `assets/rawpages/html/app-mobile-duo-pre-match.html` (B1)

## Le constat

- `FanFactorRecorded` porte les **deux** jets (`src/app/match_report/domain/events.rs:49-57`),
  et le formulaire de `step2` les exige tous les deux
  (`io/web/pre_match_controller.rs:167-171`) : en parallèle, le dernier qui
  enregistre écrase le jet de l'autre.
- Les valeurs d'équipe **ne sont pas figées** : chaque enregistrement relit et
  réappende `TeamValuesRecorded` (`use_cases/record_fan_factor_use_case.rs:101-122`).
- **Le fan factor n'est pas émis sur le bus** : le use case n'a pas de bus
  (`record_fan_factor_use_case.rs:51-68`). Aucun app event, donc aucun
  `MatchChanged` : la page de l'adversaire ne peut pas le voir arriver.
- **`pre-match.html` ne déclare pas sa présence** : seuls `actions-step.html:7`,
  `mvp-step.html:7` et `live-match.html:12` le font, et le signal ne va qu'aux
  présents (`src/web/realtime/registry/presence.rs:118-132`).

## Ce qui change

- **`TeamFanFactorRecorded { team_side: TeamSide, fan_roll: D3Roll,
  dedicated_fans: DedicatedFans, recorded_by: CoachId }`**, nommé dans
  `type_name`. `MatchReportPreMatch::record_team_fan_factor` pose le jet et les
  fans du seul camp. Rejeu en `PreMatch` et en `ReadyToPublish`, comme
  `FanFactorRecorded` (`domain/match_report_state.rs:119-135`, `331-347`).
- **Gel des valeurs d'équipe** (R4) : le premier `TeamFanFactorRecorded` est
  appendé avec `TeamValuesRecorded` (les deux équipes) ; le second ne relit
  rien. La règle est dans l'agrégat : « des valeurs d'équipe sont-elles déjà
  enregistrées ? ».
- **Le use case** prend le camp du coach (jamais du formulaire), un bus, et
  **émet** par `emettre`. En mode A, `FanFactorRecorded` reste tel quel.
- **L'app event `MatchFanFactorRecorded`** (`space_id`, équipes, `team_side`,
  `recorded_by`) : le publisher le produit ; `notice_of` le range sans toast
  (§ 12.7) ; le signal part de lui-même.
- **`step2` en mode B** : son jet en saisie ; celui de l'autre en lecture seule
  (« ✓ London Rats · 1 + 2 = 3, lancé par Morgane », ou « en attente ») dans un
  widget rechargé sur `matchChanged` ; la page déclare sa présence
  (`data-activity="write"`). L'admin choisit le camp qu'il saisit (§ 12.6).
- La suite : sa page de coups de pouce, ou le match si le palier n'en a pas.
- Projection : `TeamFanFactorRecorded` n'écrit rien, comme `FanFactorRecorded`
  (`match_report_repository.rs:119`).
- **Le réessai sur conflit** (carte 653, cadrage § 12.2) : devenu une commande
  d'un seul camp, le fan factor entre dans la liste blanche de
  `use_cases/conflict_retry.rs` — deux tours, `lost_the_race` puis `settle`,
  avec les valeurs d'équipe relues **dans** chaque essai. La 653 l'en avait
  écarté parce que `record_fan_factor` écrasait les deux camps sans refuser ;
  le réessai suppose que le domaine refuse un second jet du même camp.

## Fichiers touchés

`domain/events.rs`, `domain/match_report_pre_match.rs`,
`domain/match_report_state.rs`, `use_cases/record_fan_factor_use_case.rs`,
`io/web/pre_match_controller.rs`, `io/web/team_pre_match_controller.rs`,
`io/web/templates/pre-match.html`, le widget « avant-match adverse »
(`io/web/widgets/opponent_fan_factor_widget.rs`), `io/app_events/app_event_publisher.rs`,
`io/repository/match_report_repository.rs`,
`src/app/shared_kernel/app_events/match_report_app_events.rs`,
`src/web/realtime/toast.rs` ; `tests/impact-map.toml`.

## Règles et invariants

- Chacun son jet ; écrire le jet de l'autre est impossible par construction
  (le camp vient de la session).
- Les valeurs d'équipe se figent au premier fan factor du mode B, pour les deux
  équipes (R4) ; le mode A les relit comme aujourd'hui.
- Le rejeu de tout flux existant est inchangé.

## Tests

- **Unitaires** : rejeu d'un flux mode B (un fan factor par camp, une seule
  `TeamValuesRecorded`) et d'un flux mode A ; le second fan factor ne relit pas
  les valeurs d'équipe ; le use case émet `TeamFanFactorRecorded` ; le
  publisher en fait `MatchFanFactorRecorded`, sans toast.
- **E2E** : deux contextes ; chacun son jet ; le jet du domicile apparaît chez
  le visiteur sans rechargement ; un POST de `step2` par le visiteur ne touche
  pas le jet du domicile.

## Réalisation — précisions et écarts (2026-11-12)

Six décisions prises avec l'utilisateur avant le code :

1. **Le camp est dans la route** : `GET/POST /step2/{side}`, rangée dans
   `routes_de_camp()` et donc gardée par `garde_camp` — 403 sur un POST dans le
   camp de l'autre. En mode B, `GET /step2` redirige (303) vers
   `/step2/{side}` du camp de l'utilisateur (`arrival_side`). Le formulaire ne
   porte que `fan_roll`.
2. **Un second jet du même camp est refusé** (`FanFactorAlreadyRecorded`, 422).
   La correction d'un jet n'est pas couverte (cf. « Points ouverts »).
3. **L'admin** (§ 12.6) arrive sur le camp qu'il coache, au domicile s'il
   n'en coache aucun ; un lien « Saisir pour <autre équipe> » lui ouvre l'autre.
4. **Les valeurs d'équipe figées s'affichent depuis l'agrégat**
   (`FrozenTeamValuesVm`), plus depuis la projection de `teams`, dès qu'elles
   sont enregistrées.
5. **La présence** (`data-activity="write"`) n'est déclarée qu'en mode B.
6. **Le mode A n'émet rien de nouveau** : `FanFactorRecorded` ne sort toujours
   pas du BC.

Ce que la carte ne disait pas :

- `record_fan_factor` (les deux jets) est **refusé par le domaine en mode B**
  (`TeamEntryFanFactorBySide`), et `record_team_fan_factor` en mode A
  (`NotTeamEntry`) : un `POST /step2` aux deux jets dans un match en mode B
  rend 422 sans rien écrire.
- Le widget de l'adversaire a sa route, `GET /step2/{side}/fan-factor`, dans
  `routes_de_suivi()` (lecture, `garde_suivi`) ; il rend 204 hors d'un rapport
  en cours.
- Un jet lancé ne se ressaisit plus : la page montre le jet, son total, et un
  lien « Continuer » vers la suite.
- `RecordFanFactorOutcome::RedirectToInducements` porte `team_id` (au lieu de
  `topdog_team_id`) : le favori en mode A, l'équipe du coach en mode B.
- Les e2e qui démarraient un match en mode B passaient le fan factor par les
  deux jets d'un coup : `match_report_helpers.ensure_team_entry_inducements`
  les remplace par un jet par camp (`test_match_report_concurrent_writes`), et
  `test_match_report_entry_mode` suit la redirection de `/step2`.

## Points ouverts

- **Correction d'un jet** en « Chacun son équipe » : un second jet du même
  camp est refusé, et rien ne permet aujourd'hui de corriger un jet mal saisi.
  Hors de cette carte.

## Conséquences sur les données de production

Un événement **ajouté**, propre au mode B. `FanFactorRecorded`, à deux camps,
reste celui du mode A et se rejoue tel quel ; aucun compte rendu existant n'est
en mode B. `event_log` reçoit une ligne par fan factor de mode B.

## Terminé quand

Dans un match en mode B, chaque coach saisit son seul jet, voit celui de
l'autre arriver sans recharger, et les valeurs d'équipe ne changent plus après
le premier jet.
