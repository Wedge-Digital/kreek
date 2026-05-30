# Validation finale et soumission de l'équipe

**Priorité : haute**
**Dépend de :** `25-cart-realtime.md`
**Contexte :** `team_creation` (émetteur) → `competitions` (récepteur via event bus)

## Objectif

Câbler le bouton "Valider →" du panier : validation domaine de l'équipe complète, émission d'un event `TeamSubmitted`, persistance de l'état final, redirection vers "mes équipes" avec toast de confirmation.

---

## État de l'existant

| Élément | Fichier | Remarque |
|---|---|---|
| Use case stub | `use_cases/register_new_team.rs` | Entièrement commenté, ne couvre pas la soumission finale |
| Events existants | `domain_event.rs` + `domain/events/` | `DraftTeamCreated`, `RulesetSelected`, `RosterSelected` — `TeamSubmitted` absent |
| Erreurs domaine | `domain/error.rs` | Pas de `InsufficientPlayerCount` ni de règle de soumission |
| `RosterSelectedTeam` | `domain/team_roster_selected.rs` | Méthodes purchase/remove complètes ; pas de `submit()` ni de validation globale |
| Port de persistance | `ports.rs` → `ITeamRosterRepository` | Créé en carte 21 |
| Route "mes équipes" | `routes.rs` → `my_teams()` | Cible de la redirection post-soumission |

---

## Conception

### Nouvelle validation domaine

`RosterSelectedTeam` ne valide aujourd'hui que les achats un par un. La soumission nécessite une validation de l'état global.

```rust
// domain/error.rs — nouveau variant
DomainError::InsufficientPlayerCount,  // < 11 joueurs engagés
```

```rust
// domain/team_roster_selected.rs
pub const MIN_PLAYERS_FOR_SUBMISSION: usize = 11;

pub fn validate_for_submission(&self) -> Result<(), Vec<DomainError>> {
    let mut errors = Vec::new();
    if self.hired_players.len() < MIN_PLAYERS_FOR_SUBMISSION {
        errors.push(DomainError::InsufficientPlayerCount);
    }
    // Extensible : règles compétition, budget min, etc.
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
```

Le budget est déjà garanti par les achats individuels — pas de double vérification nécessaire.

### Nouveau domain event

```rust
// domain/events/team_submitted_event.rs
#[derive(Debug, Serialize, Deserialize)]
pub struct TeamSubmittedEvent {
    pub team_id:        String,
    pub team_name:      String,
    pub space_id:       String,
    pub competition_id: String,
    pub season_id:      String,
    pub roster_id:      String,
    pub roster_name:    String,
    pub coach_id:       String,
    pub total_cost:     u32,
}

impl DomainEvent for TeamSubmittedEvent {
    fn event_type() -> &'static str { "TeamSubmittedEvent" }
    fn version()    -> &'static str { "1.0" }
    fn schema()     -> &'static str { "/schemas/team_creation" }
}
```

À ajouter dans `domain_event.rs` → `TeamCreationEvent::TeamSubmitted { … }`.

### Commande et use case

```rust
// use_cases/commands.rs
pub struct SubmitTeamCommand {
    pub team_id:  TeamId,
    pub space_id: String,
}
```

```rust
// use_cases/submit_team.rs
pub enum SubmitTeamError {
    TeamNotFound,
    Domain(Vec<DomainError>),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd:        SubmitTeamCommand,
    team_repo:  &dyn ITeamRosterRepository,
    event_bus:  &EventBus,
) -> Result<(), SubmitTeamError>
```

Logique :
1. `team_repo.find_by_id(&cmd.team_id)` → `TeamNotFound`
2. `team.validate_for_submission()` → `SubmitTeamError::Domain` si erreurs
3. Marquer l'équipe comme soumise — deux options :
   - **Option A** (recommandée) : ajouter un champ `submitted_at: Option<DateTime>` dans la table `team_roster_selections` et le mettre à jour
   - **Option B** : changer le statut dans `team_drafts` pour éviter une double-écriture
4. `team_repo.save(...)` avec le statut mis à jour
5. Publier `TeamSubmittedEvent` sur l'event bus (contexte compétition l'écoute pour inscrire l'équipe)

### Route et handler

```
POST /app/{space_id}/team/{team_id}/submit
```

Pas de body — toute la donnée est déjà persistée.

**Réponse en cas de succès** :

```rust
Response::builder()
    .header("HX-Redirect", team_routes.my_teams(&space_id))
    .header("HX-Trigger", r#"{"showToast":"Équipe soumise avec succès !"}"#)
    .body(Body::empty())
    .unwrap()
```

**Réponse en cas d'erreur domaine** (HTTP 422) :

Fragment injecté dans `#submit-error` sous le bouton :

```html
<div id="submit-error" class="submit-error-panel">
  {% for err in errors %}
  <div class="submit-error-line">{{ err }}</div>
  {% endfor %}
</div>
```

| `DomainError` | Message FR |
|---|---|
| `InsufficientPlayerCount` | Vous devez engager au moins 11 joueurs pour soumettre votre équipe. |

### Modifications de `build-team.html`

```html
<!-- Bouton Valider dans build-cart-fragment.html (carte 25) -->
<button class="btn btn-primary"
        hx-post="{{ team_routes.submit_team(space_id, team_id) }}"
        hx-target="#submit-error"
        hx-swap="innerHTML"
        {% if cart.total == 0 %}disabled{% endif %}>
  Valider →
</button>
<div id="submit-error"></div>
```

### Mise à jour du statut dans "mes équipes"

Après soumission, la page "mes équipes" doit afficher l'équipe avec le statut mis à jour. Le `TeamCardVm.status` passe de `"draft"` à `"active"` (ou `"submitted"`), et `status_label` de `"Brouillon"` à `"Inscrite"`.

Le handler `my_teams` lit le statut depuis le repository — il faut exposer un champ `status` dans `ITeamRosterRepository::find_by_coach_and_space` ou maintenir la cohérence dans `team_drafts`.

---

## Checklist

- [ ] `DomainError::InsufficientPlayerCount` dans `domain/error.rs` + message FR
- [ ] `MIN_PLAYERS_FOR_SUBMISSION` + `validate_for_submission()` sur `RosterSelectedTeam`
- [ ] `TeamSubmittedEvent` dans `domain/events/team_submitted_event.rs` + enregistrement dans `domain_event.rs`
- [ ] `SubmitTeamCommand` dans `commands.rs`
- [ ] Use case `submit_team.rs` : validation → statut → event bus
- [ ] Champ `submitted_at` dans la migration `team_roster_selections` (option A)
- [ ] Route `SUBMIT_TEAM` dans `routes.rs` + `router.rs`
- [ ] Handler POST `submit_team` : 200 `HX-Redirect` + `HX-Trigger` toast / 422 fragment erreur
- [ ] Bouton "Valider →" câblé dans `build-cart-fragment.html` (carte 25)
- [ ] `my_teams` handler : affiche statut `"active"` / `"Inscrite"` pour les équipes soumises
- [ ] Listener `TeamSubmittedEvent` dans le contexte `competitions` (ticket séparé)