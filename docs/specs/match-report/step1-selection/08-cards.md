# Step 1 — Sélection du match : Cartes kanban

## Ordre d'implémentation

| # | Carte | Dépend de | Résumé |
|---|-------|-----------|--------|
| 87 | shared_kernel — VOs + MatchReporting | — | Ajouter MatchReportId, RoundId, PairingId + variant MatchReporting dans GamePhase |
| 88 | domain — MatchReportDraft + events + rehydrate | 87 | Agrégat, domain events, VOs locaux, erreurs, rehydrate(), tests unitaires |
| 89 | event store — migration + repository | 88 | Migration SQL, trait IMatchReportRepository, implémentation, projection |
| 90 | ports + adapters | 87 | ICompetitionDataPort, ITeamDataPort, adapters dans infrastructure/match_report/ |
| 91 | BC wiring — context + router | 89, 90 | MatchReportContext, router, routes, intégration main.rs |
| 92 | GET handlers — page + fragments cascade | 91 | Handlers GET, templates Askama, TomSelect searchable |
| 93 | use cases + POST handlers | 91 | CreateMatchReportUseCase, UpdateMatchSelectionUseCase, POST handlers |
| 94 | app event PairingCreated | 93 | Émission depuis BC competitions, listener dans BC match_report |
| 95 | app event MatchReportConfirmed | 93, 87 | Émission depuis BC match_report, listener dans BC teams (→ MatchReporting) |
| 96 | E2E tests step1 | 92, 93 | Tests Playwright : création, pré-remplissage, cascade, erreurs, reprise |
