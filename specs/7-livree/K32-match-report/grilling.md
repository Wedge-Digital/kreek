# K32 — Grilling

> Reconstitué à la migration (2026-11-19), à partir de `docs/specs/match-report/`. Le dépôt ne garde
> pas de trace d'un grilling : les besoins ci-dessous reprennent l'objectif et les cartes, à relire.

## Questions et réponses

Aucune trace dans le dépôt.

## Besoins

- **K32-B1** — MR-STEP2-04 — Use case record_fan_factor (ticket `100-mr-step2-record-fan-factor-use-case`)
- **K32-B2** — MR-STEP2-05 — Endpoint JSON team-match-context (BC Teams) (ticket `101-mr-step2-team-match-context-json`)
- **K32-B3** — MR-STEP2-06 — Page step 2 : handler GET + template (ticket `102-mr-step2-page-get`)
- **K32-B4** — BC match_report — Domain : value objects + events inducements (ticket `105-mr-step2-inducements-domain-vo-events`)
- **K32-B5** — BC match_report — Domain : méthodes agrégat MatchReportPreMatch (ticket `106-mr-step2-inducements-domain-methods`)
- **K32-B6** — BC match_report — Migration SQL + projection repository (ticket `107-mr-step2-inducements-migration-projection`)
- **K32-B7** — BC match_report — Ports extension + infrastructure adapters (ticket `108-mr-step2-inducements-ports-adapters`)
- **K32-B8** — BC match_report — Use case record_fan_factor (extension TV + routing) (ticket `109-mr-step2-inducements-uc-fan-factor`)
- **K32-B9** — BC match_report — Use case record_inducements (nouveau) (ticket `110-mr-step2-inducements-uc-record`)
- **K32-B10** — BC references — Widget inducement-selector (ticket `111-ref-inducement-selector-widget`)
- **K32-B11** — BC match_report — Page inducements GET + POST (ticket `112-mr-step2-inducements-page`)
- **K32-B12** — BC match_report — E2E tests step2-inducements (ticket `113-mr-step2-inducements-e2e`)
- **K32-B13** — BC match_report — Domain : value objects + events step3-4 (ticket `114-mr-step3-4-domain-vo-events`)
- **K32-B14** — BC match_report — Domain : champs agrégat, méthodes + rehydratation (ticket `115-mr-step3-4-domain-aggregate`)
- **K32-B15** — BC match_report — Migration SQL + projection repository step3-4 (ticket `116-mr-step3-4-migration-projection`)
- **K32-B16** — BC match_report — Ports + adapters step3-4 (ticket `117-mr-step3-4-ports-adapters`)
- **K32-B17** — BC match_report — Use case init_temp_players + extension record_inducements (ticket `118-mr-step3-4-uc-init-temp-players`)
- **K32-B18** — BC match_report — Use cases record_action + delete_action (ticket `119-mr-step3-4-uc-record-delete-action`)
- **K32-B19** — BC players — Widget match-player-selector (ticket `120-players-match-player-selector-widget`)
- **K32-B20** — BC match_report — Page hôte step3/step4 + turn-selector + temp-player-selector (ticket `121-mr-step3-4-page-turn-temp-widgets`)
- **K32-B21** — BC match_report — Action-panel + action-log + record_action_controller (ticket `122-mr-step3-4-action-panel-log-controller`)
- **K32-B22** — BC match_report — E2E tests step3-4-actions (ticket `123-mr-step3-4-e2e`)
- **K32-B23** — BC match_report — Domaine mercenaires (BR4) (ticket `124-mr-step2-merco-domaine`)
- **K32-B24** — BC match_report — Ports + Adapters mercenaires (ticket `125-mr-step2-merco-ports-adapters`)
- **K32-B25** — BC match_report — Extension use case + collect_mercs (ticket `126-mr-step2-merco-use-case`)
- **K32-B26** — BC match_report — Widget mercenary-selector (GET) (ticket `127-mr-step2-merco-widget-get`)
- **K32-B27** — BC match_report — Controller + HTML inducements (extension mercenaires) (ticket `128-mr-step2-merco-controller-html`)
- **K32-B28** — BC match_report — E2E tests step2 mercenaires (ticket `129-mr-step2-merco-e2e`)
- **K32-B29** — MR-STEP5-01 — Enrichissement fan factor (Option B) (ticket `139-mr-step5-fan-factor-option-b`)
- **K32-B30** — MR-STEP5-02 — Domaine step5 (ticket `140-mr-step5-domaine`)
- **K32-B31** — MR-STEP5-03 — Use case `record_post_match` (ticket `141-mr-step5-use-case`)
- **K32-B32** — MR-STEP5-04 — Handler + template step5 (ticket `142-mr-step5-handler-template`)
- **K32-B33** — MR-STEP5-05 — Tests E2E step5 (ticket `143-mr-step5-e2e-tests`)
- **K32-B34** — MR-RECAP-01 — Domaine MatchReportPublished (ticket `144-mr-recap-domaine`)
- **K32-B35** — MR-RECAP-02 — Mini BC spp_calculator (stub) (ticket `145-mr-recap-spp-calculator-stub`)
- **K32-B36** — MR-RECAP-03 — Ports & adapters (ticket `146-mr-recap-ports-adapters`)
- **K32-B37** — MR-RECAP-04 — Bus interne + publisher + AppEvent (ticket `147-mr-recap-appevent-wiring`)
- **K32-B38** — MR-RECAP-05 — Use case publish_match_report_use_case (ticket `148-mr-recap-use-case`)
- **K32-B39** — MR-RECAP-06 — Handler + template + VMs (ticket `149-mr-recap-handler-template`)
- **K32-B40** — BC match_report — shared_kernel : VOs + MatchReporting (ticket `87-mr-shared-kernel-vos`)
- **K32-B41** — BC match_report — Domaine : MatchReportDraft + events + rehydrate (ticket `88-mr-domain-draft-events`)
- **K32-B42** — BC match_report — Event store + repository (ticket `89-mr-event-store-repository`)
- **K32-B43** — BC match_report — Ports + adapters (ticket `90-mr-ports-adapters`)
- **K32-B44** — BC match_report — BC wiring (context + router) (ticket `91-mr-bc-wiring`)
- **K32-B45** — BC match_report — GET handlers + templates step1 (ticket `92-mr-get-handlers-templates`)
- **K32-B46** — BC match_report — Use cases + POST handlers step1 (ticket `93-mr-use-cases-post-handlers`)
- **K32-B47** — BC match_report — App event PairingCreated (ticket `94-mr-app-event-pairing-created`)
- **K32-B48** — BC match_report — App event MatchReportConfirmed + listener teams (ticket `95-mr-app-event-confirmed`)
- **K32-B49** — MR-STEP2-01 — Value object D3Roll (ticket `97-mr-step2-d3roll`)
- **K32-B50** — MR-STEP2-02 — Événement FanFactorRecorded + rehydratation (ticket `98-mr-step2-fan-factor-event`)
- **K32-B51** — MR-STEP2-03 — Méthode agrégat PreMatch::record_fan_factor (ticket `99-mr-step2-record-fan-factor-method`)

## Hors périmètre

Non tracé dans le dépôt.
