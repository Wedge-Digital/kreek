# Team detail — Phases 2 à 7 : conception

Feature à périmètre restreint sur une page unique (`teams-team-detail.html`),
sans nouveau widget — traitée en un seul document.

---

## Phase 2 — Architecture front

Aucun nouveau widget. Le bandeau d'état est un bloc conditionnel **inline**
dans le template existant `teams-team-detail.html`, calculé depuis le même
`TeamDetailVm` déjà chargé par le handler `team_detail()` (même précédent que
le badge de statut actuel, ou que `.lock-banner` dans `player-detail.html`).
Positionné entre l'en-tête (`.team-header`) et les onglets (`.tabs`), comme
dans la maquette.

Pas de communication par événements DOM (pas de widgets tiers à notifier) :
les 3 actions de mutation déclenchent un `HX-Refresh: true` qui recharge toute
la page (le changement de phase impacte aussi le badge d'en-tête).

| Élément | Type | Trigger | Réponse |
|---|---|---|---|
| Bouton "Imprimer en PDF" | client-only | `onclick` | `window.print()`, pas de requête |
| Lien "Reprendre le rapport →" | navigation | `hx-get` | swap `#app-content` (même pattern que le lien "← Mes équipes") |
| Bouton "Évolutions terminées" | mutation | `hx-post` | `HX-Refresh: true` |
| Bouton "Terminer les achats" | mutation | `hx-post` | `HX-Refresh: true` |
| Bouton "Valider les renvois" | mutation | `hx-post` | `HX-Refresh: true` |

---

## Phase 3 — Architecture back

Widgets → BC : aucun nouveau widget, tout reste dans `teams::io::web::team_detail`.

### Fichiers

```
src/app/teams/
├── domain/team.rs                                   (modifié)
├── domain/error.rs                                  (inchangé)
├── use_cases/
│   ├── validate_improvement_phase_use_case.rs       (nouveau)
│   ├── validate_recruitment_phase_use_case.rs        (nouveau)
│   ├── validate_dismissals_phase_use_case.rs         (nouveau)
│   └── commands.rs                                   (modifié — 3 commandes)
├── io/
│   ├── app_events/match_report_published_listener.rs (nouveau)
│   ├── web/
│   │   ├── team_detail.rs                            (modifié — VM banner + 3 handlers)
│   │   └── templates/teams-team-detail.html           (modifié — bloc bandeau)
│   └── repository/team_repository.rs                 (inchangé — pas de nouvelle colonne projection)
├── context.rs                                        (modifié — câblage du nouveau listener)
├── routes.rs                                          (modifié — 3 routes)
└── router.rs                                          (modifié — 3 routes)
```

Aucun port inter-BC nécessaire : le lien "reprendre le rapport" utilise
`AppRoutes::default().match_report.edit_match_report(space_id, match_report_id)`
(déjà accessible via `AppRoutes`, cf. règle CLAUDE.md « Accès aux routes »).
Le `match_report_id` est porté par l'agrégat `Team` lui-même (champ ajouté en
phase 6), pas par une lecture cross-BC.

Aucun domain service nécessaire (pas de DTO de port à transformer).

---

## Phase 4 — Contrats de données

### Commandes (entrée — value objects, pas de primitives nues)

```rust
// src/app/teams/use_cases/commands.rs
pub struct ValidateImprovementPhaseCommand { pub team_id: TeamId }
pub struct ValidateRecruitmentPhaseCommand { pub team_id: TeamId }
pub struct ValidateDismissalsPhaseCommand  { pub team_id: TeamId }
```
Émises par : les 3 handlers HTTP (`io/web/team_detail.rs`), à partir du `team_id` du path, validé via `TeamId::try_new()`.
Consommées par : les 3 use cases correspondants.

### VM de sortie (lecture — primitives acceptées)

```rust
// io/web/team_detail.rs
pub enum BannerCtaVm {
    Print,                                    // bouton "Imprimer en PDF"
    Navigate  { label: String, href: String },        // "Reprendre le rapport →"
    Mutate    { label: String, post_url: String, outline: bool }, // actions de validation
}

pub struct BannerVm {
    pub css_variant: String,   // "pending" | "ready" | "phase"
    pub icon: String,          // emoji
    pub title: String,         // partie <strong>
    pub detail: String,        // reste du texte
    pub ctas: Vec<BannerCtaVm>,
}
```
Émis par : `BannerVm::from_domain(team, space_id, app_routes)` — constructeur co-localisé (pur domaine, cf. convention VM).
Consommé par : `TeamDetailTemplate` / `teams-team-detail.html`.

`TeamDetailVm` gagne un champ `pub banner: Option<BannerVm>` (`None` pour les phases non couvertes — cf. règle transverse).

Pas de nouveau DTO de port.

---

## Phase 5 — Use cases

Un fichier par mutation, même forme que `use_cases/dismiss_team.rs` (find_by_id → méthode domaine → append).

```rust
// validate_improvement_phase_use_case.rs
pub enum ValidateImprovementPhaseError { TeamNotFound, Domain(DomainError), Repository(RepositoryError) }

pub async fn execute(
    cmd: ValidateImprovementPhaseCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), ValidateImprovementPhaseError> {
    let team = team_repo.find_by_id(&cmd.team_id.to_string()).await
        .map_err(ValidateImprovementPhaseError::Repository)?
        .ok_or(ValidateImprovementPhaseError::TeamNotFound)?;
    let event = team.validate_improvement_phase().map_err(ValidateImprovementPhaseError::Domain)?;
    team_repo.append(&cmd.team_id.to_string(), &event, team.version).await
        .map_err(ValidateImprovementPhaseError::Repository)?;
    Ok(())
}
```

Idem pour `validate_recruitment_phase_use_case.rs` (appelle `team.validate_recruitment_phase()`)
et `validate_dismissals_phase_use_case.rs` (appelle `team.validate_dismissals_phase()`).

Aucune logique métier dans les use cases — ils délèguent entièrement aux
méthodes domaine déjà existantes. Erreurs applicatives : réutilisent le
`DomainError` existant (pas de nouveau variant nécessaire).

---

## Phase 6 — Domaine

### Récapitulatif des règles métier (validé par l'utilisateur)

1. Bandeau "en attente d'inscription" — informatif, aucune action.
2. Bandeau "prête à jouer" — bouton impression, client-only.
3. Bandeau "rapport en cours" — lien de reprise vers `current_match_report_id`.
4. Bandeau "phase d'amélioration" — bouton "Évolutions terminées" → `PlayerImprovement → Recruitment`.
5. Bandeau "phase de recrutement" — bouton "Terminer les achats" → `Recruitment → Dismissals`. Bouton "Recruter" omis.
6. Bandeau "phase de renvois" — bouton "Valider les renvois" → `Dismissals → ReadyToPlay` **(modifié, bypass TemporaryRetirement)**. Bouton "Gérer les renvois" omis.
7. Phases non couvertes (`TemporaryRetirement`, `OffSeason`, `Dismissed`, `Rejected`) → pas de bandeau.
8. Aucune transition possible en dehors de celles listées — gardes déjà en place (`expect_phase`, matching sur `participation_status`). Seule échappatoire : `override_phase()` (admin, hors périmètre).
9. Listener `MatchReportPublished` : dérive `MatchResult` des scores par équipe, ignore silencieusement (log warn) si équipe introuvable ou hors phase `ReadyToPlay`.
10. `current_match_report_id` : peuplé sur `MatchReportingStarted`, vidé sur `PostMatchSequenceStarted`.
11. Agrégat `Team` confirmé event-sourcé (`team_event_store` rejoué via `hydrate()`).

### Modifications sur l'agrégat `Team` (`domain/team.rs`)

```rust
pub struct Team {
    // ...champs existants...
    pub current_match_report_id: Option<MatchReportId>,  // nouveau
}
```

```rust
// apply()
TeamDomainEvent::MatchReportingStarted { match_report_id } => {
    self.game_phase = Some(GamePhase::MatchReporting);
    self.current_match_report_id = Some(*match_report_id);   // nouveau
}
TeamDomainEvent::PostMatchSequenceStarted { .. } => {
    // ...inchangé...
    self.current_match_report_id = None;                     // nouveau
}
TeamDomainEvent::DismissalsPhaseValidated => {
    self.game_phase = Some(GamePhase::ReadyToPlay);           // modifié (était TemporaryRetirement)
}
```

Aucun nouveau `DomainError`. Aucune nouvelle méthode de commande domaine — les
4 méthodes `validate_*_phase()` existent déjà et sont réutilisées telles
quelles (seul le comportement de `apply()` sur l'event résultant change pour
`DismissalsPhaseValidated`).

### Tests unitaires prévus

- Modifier `phase_sequence_advances_correctly` (`team.rs` tests) : `Dismissals → ReadyToPlay` au lieu de `→ TemporaryRetirement`.
- Nouveau test : `MatchReportingStarted` peuple `current_match_report_id`.
- Nouveau test : `PostMatchSequenceStarted` vide `current_match_report_id`.
- Nouveau test : `validate_dismissals_phase()` hors phase `Dismissals` → `Err(WrongGamePhase)` (déjà couvert par le pattern `expect_phase`, à dupliquer pour ce cas précis si absent).
- Nouveau test (module `match_report_published_listener`, ou test d'intégration) : dérivation correcte de `MatchResult` depuis les scores (home > away → home Win/away Loss, égalité → Draw des deux côtés).

---

## Phase 7 — Effets de bord

### Persistance

- Aucune nouvelle colonne sur `team_proj` — le champ `current_match_report_id`
  ne vit que sur l'agrégat event-sourcé (le handler lit l'agrégat via
  `find_by_id`, pas la projection).
- `update_projection_in_tx()` : aucun changement requis pour les 3 nouveaux
  events de validation de phase (le `game_phase` textuel n'est pas mis à jour
  en base pour ces phases actuellement — cohérent avec l'existant qui ne
  couvre que `TeamCreated`/`TeamEnrolled`/`MatchReportingStarted`/`TeamDismissed`/`TeamEnrollmentRejected`).
  *(Note : `team_proj.game_phase` restera donc désynchronisé pour Recruitment/Dismissals/PlayerImprovement — déjà le cas aujourd'hui, hors périmètre de cette carte puisque rien ne lit `team_proj.game_phase` pour ces phases.)*

### Événements

- Domain events réutilisés : `PlayerImprovementPhaseValidated`, `RecruitmentPhaseValidated`, `DismissalsPhaseValidated`, `PostMatchSequenceStarted` (tous déjà définis).
- Nouveau listener `match_report_published_listener::init(app_event_bus, team_repo)`, câblé dans `teams::context::init_listeners`, souscrivant à `MatchReportAppEvent::MatchReportPublished`.

### Handlers HTTP

```rust
// io/web/team_detail.rs (ou fichier dédié)
pub async fn post_validate_improvement_phase(Path((space_id, team_id)), State(state)) -> impl IntoResponse
pub async fn post_validate_recruitment_phase(Path((space_id, team_id)), State(state)) -> impl IntoResponse
pub async fn post_validate_dismissals_phase(Path((space_id, team_id)), State(state)) -> impl IntoResponse
```
Chacun : construit la commande → appelle le use case → `HX-Refresh: true` en succès, fragment d'erreur HTMX sinon (`AppError`).

### Templates

- `teams-team-detail.html` : ajout du bloc `state-banner` (une branche par `css_variant`/CTA), entre `.team-header` et `.tabs`.
- CSS : classes `.state-banner*` ajoutées à `assets/static/css/pages/app-team-detail.css` (reprises de la maquette, pas de style inline).

### Tests E2E prévus

- Équipe `PendingEnrollment` → bandeau informatif visible, aucune action.
- Équipe `ReadyToPlay` → bouton impression présent (pas de vérification de l'impression réelle).
- Équipe `MatchReporting` → clic sur "Reprendre le rapport" navigue vers la bonne étape du rapport.
- Équipe `PlayerImprovement` → clic "Évolutions terminées" → page rechargée, badge "Phase de recrutement".
- Équipe `Recruitment` → clic "Terminer les achats" → badge "Phase de renvois".
- Équipe `Dismissals` → clic "Valider les renvois" → badge "Prête à jouer".
