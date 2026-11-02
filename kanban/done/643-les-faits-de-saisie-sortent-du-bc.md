# Les faits de saisie sortent du BC

**Épic :** E23 — Le temps réel : connexions, présence et notifications · **Ordre :** 1 (en parallèle de la 642) · **Dépend de :** rien
**Priorité :** normale
**Conception :** `docs/specs/saisie-mobile/E23-cadrage.md` (§ 1 « Les faits de l'épic, face au code », § 3, § 7, § 11.7, 11.10, 11.11, 11.12)

## Objectif

Ce que fait un coach pendant la saisie devient visible hors de `match_report` :
le démarrage, chaque action (MVP compris), son retrait, la sélection des coups
de pouce validée avec ce qui a été pris, leur annulation. Chaque fait sort en
app event, avec son auteur nommé, par le seul publisher du BC. Personne ne
l'écoute encore : la 645 et la 646 s'y branchent.

## Ce que le code fait

Chemins relatifs à `src/app/match_report/`.

- **Quatre faits sur six ne sont pas émis** : `record_action_use_case.rs:132`,
  `delete_action_use_case.rs:56`, `record_mvp_use_case.rs:85`,
  `draw_mvp_use_case.rs:52` appendent sans `emettre()` ;
  `record_inducements_use_case.rs:452` appende et ne passe le bus qu'à
  `init_temp_players` (`:455-465`). Le démarrage
  (`create_match_report_use_case.rs:158-167`, `update_match_selection_use_case.rs:138-143`)
  et l'annulation des coups de pouce (`cancel_inducements_use_case.rs:96-100`)
  le sont.
- Les auteurs sont dans les événements : `ActionRecorded.recorded_by`,
  `ActionDeleted.deleted_by`, `InducementsRecorded.recorded_by`,
  `InducementsCancelled.cancelled_by`, `SelectionConfirmed.confirmed_by`
  (`domain/events.rs:36-158`). `MvpDrawn` n'en porte pas ; l'`ActionRecorded`
  appendé avec lui, si.
- Le publisher aiguille sans joker ; ces faits sont dans son bras groupé
  (`io/app_events/app_event_publisher.rs:150-167`). Ses dépendances : `repo`,
  `competition_data`, `team_data` (`context.rs:32-49`) — ni `coach_data`, ni
  `reference_catalog`, que le contexte possède pourtant (`context.rs:21`, `:28`).
- `MatchReportConfirmed` porte `space_id` et les deux équipes, pas d'auteur
  (`shared_kernel/app_events/match_report_app_events.rs:8-29`).
- **Aucun listener d'un autre BC ne fait de `match` exhaustif** sur
  `MatchReportAppEvent` (`teams/io/app_events/journeymen_fielded_listener.rs:77`,
  `players/io/app_events/journeyman_departure_listener.rs:85-87`) : de nouveaux
  variants ne cassent rien.
- Les libellés de coups de pouce se résolvent déjà par
  `IReferenceCatalogPort::find_inducement_labels` (`ports.rs:288`,
  `use_cases/inducements_recap_service.rs:142`).

## Ce qu'il faut faire

- **Émettre** après l'append, dans les cinq use cases ci-dessus, comme
  `cancel_inducements` (`emettre(bus, event.to_enveloppe(mr_id))` pour chaque
  événement appendé) ; rien n'est émis sur un conflit. Les contrôleurs leur
  passent `state.event_bus` (`record_action_controller.rs:104`, `:192`,
  `mvp_step_controller.rs:163`, `mvp_draw_controller.rs:60`,
  `inducements_controller.rs:365`).
- **Les app events** (§ 3), non persistés, tous porteurs de `space_id`,
  `home_team_id`, `away_team_id` :
  - `MatchReportConfirmed` **enrichi** de `confirmed_by`, `confirmed_by_name` ;
  - `MatchActionRecorded` : `team_side`, `team_name`, `turn`,
    `action: ActionTypePayload`, `player_display_name`, `recorded_by`,
    `recorded_by_name` ;
  - `MatchActionDeleted` : `team_side`, `deleted_by` ;
  - `MatchInducementsRecorded` : `team_id`, `team_name`, `recorded_by`,
    `recorded_by_name`, `items: Vec<{ label, qty, kind }>`, libellés résolus par
    `IReferenceCatalogPort` (vedettes comprises) et, pour un mercenaire, par le
    poste du roster, quantités prises dans `purchases` ; liste vide pour une
    sélection vide. `kind: Inducement | StarPlayer | Mercenary { skilled }`
    (décision du 2026-11-02) : sans lui, un mercenaire ne s'annoncerait que par
    son poste, et la 645 devrait décoder l'identifiant `MERCO:…` ;
  - `MatchInducementsCancelled` : `cancelled_by`, `cancelled_by_name`.
  `event_type()` et `to_enveloppe()` les nomment (`match_report_app_events.rs:200-240`).
  Tous les `*_name` sont des `Option<String>` (décision du 2026-11-02) : un nom
  introuvable n'est pas inventé, c'est au canal de dire comment l'annoncer. Le
  camp est un `TeamSidePayload { Home, Away }`.
- **Le publisher** : un bras par fait ; la portée se lit sur la projection
  (`find_access_scope`, `find_space_id`, `domain/match_report_repository_port.rs:71-85`),
  pas par la relecture de l'agrégat ; les noms par `ICoachDataPort` et
  `ITeamDataPort::find_team_info`. `MvpDrawn` reste dans le bras groupé
  (décision 10) ; les autres faits aussi.
- **L'injection** : `coach_data` et `reference_catalog` passés au publisher
  (`context.rs:32-49`, appel dans `main.rs`).

## Fichiers touchés

`use_cases/record_action_use_case.rs`, `delete_action_use_case.rs`,
`record_mvp_use_case.rs`, `draw_mvp_use_case.rs`, `record_inducements_use_case.rs` ;
`io/web/record_action_controller.rs`, `mvp_step_controller.rs`,
`mvp_draw_controller.rs`, `inducements_controller.rs` ;
`io/app_events/app_event_publisher.rs`, `context.rs` ;
`src/app/shared_kernel/app_events/match_report_app_events.rs` ; `src/main.rs`.

## Tests

- **Unitaires** : chaque use case émet ses événements après l'append, et
  n'émet rien sur un conflit (patron de `cancel_inducements_use_case.rs:155-215`) ;
  le tirage émet l'`ActionRecorded` puis `MvpDrawn`. Le publisher, sur
  doublures : chaque bras produit son app event avec auteur et noms ; la
  sélection de coups de pouce porte ses libellés et quantités, vide comprise ;
  `MvpDrawn` ne produit rien.
- **E2E** (`tests/e2e/test_match_report_outgoing_facts.py`) : une action saisie
  au navigateur, puis la table `event_log` lue par `query_db` porte la ligne
  `ActionRecorded` du rapport — la preuve, sans canal, que le fait a quitté
  l'append. Idem pour la validation des coups de pouce.
- `tests/impact-map.toml` mis à jour.

## Hors périmètre

- Qui reçoit quoi, et le texte du toast : 645.
- « Camp validé » : E22.
- Les autres faits (`FanFactorRecorded`, `PostMatchRecorded`…) : ils restent
  dans le BC.

## Conséquences sur les données de production

**La table `event_log` reçoit davantage de lignes** : `event_log_feeder`
(`src/common/event_listener/event_log_feeder.rs:7-24`) y écrit chaque domain
event émis, et les actions, retraits, MVP et validations de coups de pouce le
deviennent — quelques dizaines de lignes par match. La table n'a aucun lecteur
en production. L'event store, les projections et l'existant ne changent pas ;
les app events ne sont pas persistés.

## Terminé quand

- [ ] Une action saisie produit une ligne `ActionRecorded` dans `event_log` et une ligne `app event émis event=MatchActionRecorded` au journal
- [ ] La validation des coups de pouce d'un camp produit `MatchInducementsRecorded` avec ses libellés et quantités
- [ ] Le démarrage d'un match produit `MatchReportConfirmed` portant le nom du démarreur
- [ ] Un conflit d'écriture n'émet rien
- [ ] Les listeners de `teams`, `competitions`, `players`, `ranking` passent leurs tests sans modification de leur code. Seul change un test : celui de `competitions/io/app_events/match_report_confirmed_listener.rs` construit `MatchReportConfirmed` champ par champ et reçoit les deux champs ajoutés (`confirmed_by`, `confirmed_by_name`) ; le listener, lui, lit l'événement avec `..` et n'est pas touché
- [ ] `make lint`, `make check-arch`, `make test`, e2e impactés
