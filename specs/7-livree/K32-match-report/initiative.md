---
id: K32
title: "Match Report — Spec index"
conception: docs/specs/match-report/
---

# K32 — Match Report — Spec index

Initiative créée à la migration vers le kanban de la spec factory (2026-11-19), à partir de la conception de la fonctionnalité `docs/specs/match-report/` (workflow feature). Ses cartes y sont listées dans les `08-cards.md`.

Conception : [`docs/specs/match-report/`](../../../docs/specs/match-report/)

## Tickets

- `100-mr-step2-record-fan-factor-use-case` — MR-STEP2-04 — Use case record_fan_factor (livrée à la migration)
- `101-mr-step2-team-match-context-json` — MR-STEP2-05 — Endpoint JSON team-match-context (BC Teams) (livrée à la migration)
- `102-mr-step2-page-get` — MR-STEP2-06 — Page step 2 : handler GET + template (livrée à la migration)
- `105-mr-step2-inducements-domain-vo-events` — BC match_report — Domain : value objects + events inducements (livrée à la migration)
- `106-mr-step2-inducements-domain-methods` — BC match_report — Domain : méthodes agrégat MatchReportPreMatch (livrée à la migration)
- `107-mr-step2-inducements-migration-projection` — BC match_report — Migration SQL + projection repository (livrée à la migration)
- `108-mr-step2-inducements-ports-adapters` — BC match_report — Ports extension + infrastructure adapters (livrée à la migration)
- `109-mr-step2-inducements-uc-fan-factor` — BC match_report — Use case record_fan_factor (extension TV + routing) (livrée à la migration)
- `110-mr-step2-inducements-uc-record` — BC match_report — Use case record_inducements (nouveau) (livrée à la migration)
- `111-ref-inducement-selector-widget` — BC references — Widget inducement-selector (livrée à la migration)
- `112-mr-step2-inducements-page` — BC match_report — Page inducements GET + POST (livrée à la migration)
- `113-mr-step2-inducements-e2e` — BC match_report — E2E tests step2-inducements (livrée à la migration)
- `114-mr-step3-4-domain-vo-events` — BC match_report — Domain : value objects + events step3-4 (livrée à la migration)
- `115-mr-step3-4-domain-aggregate` — BC match_report — Domain : champs agrégat, méthodes + rehydratation (livrée à la migration)
- `116-mr-step3-4-migration-projection` — BC match_report — Migration SQL + projection repository step3-4 (livrée à la migration)
- `117-mr-step3-4-ports-adapters` — BC match_report — Ports + adapters step3-4 (livrée à la migration)
- `118-mr-step3-4-uc-init-temp-players` — BC match_report — Use case init_temp_players + extension record_inducements (livrée à la migration)
- `119-mr-step3-4-uc-record-delete-action` — BC match_report — Use cases record_action + delete_action (livrée à la migration)
- `120-players-match-player-selector-widget` — BC players — Widget match-player-selector (livrée à la migration)
- `121-mr-step3-4-page-turn-temp-widgets` — BC match_report — Page hôte step3/step4 + turn-selector + temp-player-selector (livrée à la migration)
- `122-mr-step3-4-action-panel-log-controller` — BC match_report — Action-panel + action-log + record_action_controller (livrée à la migration)
- `123-mr-step3-4-e2e` — BC match_report — E2E tests step3-4-actions (livrée à la migration)
- `124-mr-step2-merco-domaine` — BC match_report — Domaine mercenaires (BR4) (livrée à la migration)
- `125-mr-step2-merco-ports-adapters` — BC match_report — Ports + Adapters mercenaires (livrée à la migration)
- `126-mr-step2-merco-use-case` — BC match_report — Extension use case + collect_mercs (livrée à la migration)
- `127-mr-step2-merco-widget-get` — BC match_report — Widget mercenary-selector (GET) (livrée à la migration)
- `128-mr-step2-merco-controller-html` — BC match_report — Controller + HTML inducements (extension mercenaires) (livrée à la migration)
- `129-mr-step2-merco-e2e` — BC match_report — E2E tests step2 mercenaires (livrée à la migration)
- `139-mr-step5-fan-factor-option-b` — MR-STEP5-01 — Enrichissement fan factor (Option B) (livrée à la migration)
- `140-mr-step5-domaine` — MR-STEP5-02 — Domaine step5 (livrée à la migration)
- `141-mr-step5-use-case` — MR-STEP5-03 — Use case `record_post_match` (livrée à la migration)
- `142-mr-step5-handler-template` — MR-STEP5-04 — Handler + template step5 (livrée à la migration)
- `143-mr-step5-e2e-tests` — MR-STEP5-05 — Tests E2E step5 (livrée à la migration)
- `144-mr-recap-domaine` — MR-RECAP-01 — Domaine MatchReportPublished (livrée à la migration)
- `145-mr-recap-spp-calculator-stub` — MR-RECAP-02 — Mini BC spp_calculator (stub) (livrée à la migration)
- `146-mr-recap-ports-adapters` — MR-RECAP-03 — Ports & adapters (livrée à la migration)
- `147-mr-recap-appevent-wiring` — MR-RECAP-04 — Bus interne + publisher + AppEvent (livrée à la migration)
- `148-mr-recap-use-case` — MR-RECAP-05 — Use case publish_match_report_use_case (livrée à la migration)
- `149-mr-recap-handler-template` — MR-RECAP-06 — Handler + template + VMs (livrée à la migration)
- `87-mr-shared-kernel-vos` — BC match_report — shared_kernel : VOs + MatchReporting (livrée à la migration)
- `88-mr-domain-draft-events` — BC match_report — Domaine : MatchReportDraft + events + rehydrate (livrée à la migration)
- `89-mr-event-store-repository` — BC match_report — Event store + repository (livrée à la migration)
- `90-mr-ports-adapters` — BC match_report — Ports + adapters (livrée à la migration)
- `91-mr-bc-wiring` — BC match_report — BC wiring (context + router) (livrée à la migration)
- `92-mr-get-handlers-templates` — BC match_report — GET handlers + templates step1 (livrée à la migration)
- `93-mr-use-cases-post-handlers` — BC match_report — Use cases + POST handlers step1 (livrée à la migration)
- `94-mr-app-event-pairing-created` — BC match_report — App event PairingCreated (livrée à la migration)
- `95-mr-app-event-confirmed` — BC match_report — App event MatchReportConfirmed + listener teams (livrée à la migration)
- `97-mr-step2-d3roll` — MR-STEP2-01 — Value object D3Roll (livrée à la migration)
- `98-mr-step2-fan-factor-event` — MR-STEP2-02 — Événement FanFactorRecorded + rehydratation (livrée à la migration)
- `99-mr-step2-record-fan-factor-method` — MR-STEP2-03 — Méthode agrégat PreMatch::record_fan_factor (livrée à la migration)
