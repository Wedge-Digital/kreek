# MR-RECAP-06 — Handler + template + VMs

## Objectif

Implémenter la page récap : handler GET (composition de lecture pure), handler POST
(publication), template Askama, view models, routes et enregistrement dans le router.

## Dépendances

144 — état domaine `Published` disponible.
146 — ports/adapters disponibles pour la composition GET.
148 — `publish_match_report_use_case` disponible pour le POST.

## Conception

Voir `docs/specs/match-report/recap/02-front.md`, `03-back.md`, `04-dtos.md`, `07-integration.md`.

## Fichiers impactés

- `src/app/match_report/io/web/recap_controller.rs` (nouveau)
- `src/app/match_report/io/web/templates/recap.html` (nouveau)
- `src/app/match_report/io/web/builders.rs`
- `src/app/match_report/io/web/view_models.rs`
- `src/app/match_report/routes.rs`
- `src/app/match_report/router.rs`
- `src/app/match_report/io/web/mod.rs`
- `assets/static/css/pages/match-report-recap.css` (nouveau)

## Checklist

### Routes
- [ ] Constantes `MATCH_REPORT_RECAP` / `MATCH_REPORT_RECAP_PUBLISH` dans `routes.rs`
- [ ] Méthodes `recap(...)` / `recap_publish(...)` dans `impl Routes`

### View models — pur domaine (`view_models.rs`)
- [ ] `MatchResultVm::from_domain` (score, summary)
- [ ] `GainsFanVm::from_domain` (gains, fan_mod — delta uniquement)
- [ ] `TimelineEventVm::all_from_domain` (toutes les actions, `injury_label` jamais renseigné pour `Sortie`)
- [ ] `MvpRowVm::all_from_domain`
- [ ] `InjuryRowVm::all_from_domain` (uniquement `Blesse{injury}`)

### View models — dépendant d'un port (`builders.rs`)
- [ ] `build_team_banner` (×2) — `TeamBannerVm` via `ITeamDataPort::find_team_info` + score domaine
- [ ] `build_round_context_vm` — `RoundContextVm` via `ICompetitionDataPort::find_round_context`, `None` si échec
- [ ] `build_performance_rows` — `PerformanceRowVm` via `ISppCalculatorPort::calculate_match_spp`
- [ ] `build_submitted_by` — `Option<String>` via `ICoachDataPort::find_coach_name(created_by)`

### Handler GET `get_recap`
- [ ] `RecapTemplate` (champs définis dans `04-dtos.md`)
- [ ] `Draft`/`PreMatch` → 404 ; `Cancelled` → 410 ; `ReadyToPublish`/`Published` → rendu
- [ ] `is_published` calculé depuis l'état courant
- [ ] URLs construites (`publish_url`, `back_to_step5_url`, `competition_url` via `AppRoutes`, `home_team_detail_url` via `AppRoutes`)

### Handler POST `post_publish`
- [ ] Construction de `PublishMatchReportCommand` depuis `auth_session.user.id`
- [ ] Appel `publish_match_report_use_case::execute`
- [ ] Mapping erreurs → HTTP (404/409/410/500)
- [ ] Redirect vers `GET /recap` en cas de succès

### Template `recap.html`
- [ ] Extends `app-layout.html`
- [ ] Hero : contexte (masqué si `round_context` absent) + scoreboard + stats strip (TDs, Blessures — pas de KO)
- [ ] Compte-rendu (`summary_title`/`summary_body`, byline `submitted_by` si présent)
- [ ] Chronologie (`timeline` + `mvps` en fin de liste)
- [ ] Sidebar : Joueurs du match (`mvps`), Performances SPP (`performances`), Gains & Fan Factor (`gains_fan`), Bilan sanitaire (`injuries`)
- [ ] CTA bas de page : 2 variantes selon `is_published` (Publier/Modifier étape 5 vs Retour compétition/Voir fiche équipe)
- [ ] Pas de style inline — classes CSS dans `match-report-recap.css`

### Router
- [ ] Routes `MATCH_REPORT_RECAP` (GET) et `MATCH_REPORT_RECAP_PUBLISH` (POST) enregistrées
- [ ] Module `recap_controller` déclaré dans `io/web/mod.rs`

### Build & vérification
- [ ] Compiler sans erreur (`cargo build`)
- [ ] Rendu visuel vérifié en dev (correspondance maquette `app-match-summary.html`, ajustée des décisions actées)
