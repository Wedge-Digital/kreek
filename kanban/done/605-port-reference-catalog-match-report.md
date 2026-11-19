# `match_report` lit le catalogue de références

**Priorité : moyenne**
**Épic :** E16 — Imprimer la feuille d'équipe · PR 2
**Dépend de :** 603, 604
**Spec :** `04-dtos.md` (« DTO des ports »), `07-integration.md` (« Persistance »)
**Fichiers :** `src/app/match_report/ports.rs`, `src/infrastructure/match_report/reference_catalog_adapter.rs`, `src/infrastructure/match_report/player_data_adapter.rs`, `src/infrastructure/match_report/ref_team_data_adapter.rs`, `src/app/match_report/context.rs`, `src/main.rs`

## Objectif

La feuille de match et la page des coups de pouce ont besoin de ce que
`match_report` ne sait pas lire aujourd'hui : libellés et descriptions, fiches
de star players, barème SPP d'un roster, rappels du corpus, joueurs
alignables, saison d'inscription.

## Conception

- Nouveau port **synchrone** `IReferenceCatalogPort` :
  `find_inducement_labels`, `find_star_players(uids, roster_id)` (filtre de la
  603), `find_spp_scale(roster_id)`, `find_match_reminders()` (fichier de la
  604). DTO propres à `match_report`.
- `IPlayerDataPort::list_alignable_players(team_id)` → `AlignablePlayerDto { jersey, name, is_journeyman }`.
- `ITeamDataPort::find_team_season(team_id)`.
- Adapters dans `src/infrastructure/match_report/`, injection dans le contexte
  par `main.rs` ; `match_report` n'importe pas `references` (axe 3).

## Checklist

- [x] Port, DTO, adapters, injection
- [x] Tests unitaires des adapters : skills résolus en noms · star player filtré par roster · barème du roster · rappels absents → vides
- [x] Doublures de test du port pour les services des 607 et 608
- [x] `make check-arch`, `make lint`, `make test`
