# Step 1 — Sélection du match : Use cases

## Agrégat event-sourcé

L'agrégat `MatchReport` est **event-sourcé** : le repository persiste les domain events dans un event store, et l'agrégat est reconstitué par rejeu des événements (rehydratation via `apply()`). Même pattern que le BC `teams`.

## Use case 1 : `CreateMatchReportUseCase`

**Appelé par :** POST `/match-report/new` (formulaire vierge) ET listener `PairingCreated`

```rust
pub struct CreateMatchReportUseCase {
    match_report_repo: Arc<dyn IMatchReportRepository>,
}

pub enum CreateMatchReportError {
    SameTeam,
    Unauthorized,
    Repository(String),
}
```

**Orchestration :**

1. Valider que `home_team_id != away_team_id` → sinon `SameTeam`
2. Créer l'agrégat `MatchReport` via sa méthode factory → produit un `MatchReportCreated` domain event
3. Appender l'événement dans l'event store (version 0)
4. Retourner le `match_report_id`

## Use case 2 : `UpdateMatchSelectionUseCase`

**Appelé par :** POST `/match-report/{id}` (rapport pré-existant, mode édition)

```rust
pub struct UpdateMatchSelectionUseCase {
    match_report_repo: Arc<dyn IMatchReportRepository>,
}

pub enum UpdateMatchSelectionError {
    NotFound,
    SameTeam,
    TeamNotAvailable(String),
    Unauthorized,
    Repository(String),
}
```

**Orchestration :**

1. Charger l'agrégat depuis l'event store (rehydratation par rejeu) → doit être un `MatchReportState::Draft`
2. Si la sélection a changé → appel méthode domaine `update_selection()` → valide `home != away`, produit un `SelectionUpdated` domain event → appender
3. **Vérifier via `ITeamDataPort` que les deux équipes sont en `ReadyToPlay`** → sinon `TeamNotAvailable`
4. Appel méthode domaine `confirm_selection()` → produit un `SelectionConfirmed` domain event → appender
5. Retourner le `match_report_id`

Note : la vérification `ReadyToPlay` (étape 3) se fait dans le use case, pas dans l'agrégat — c'est une vérification applicative qui dépend d'un port externe. Plusieurs rapports en phase Draft peuvent coexister pour la même équipe ; seul le `SelectionConfirmed` verrouille.

## Listener `PairingCreated`

Appelle `CreateMatchReportUseCase` avec la commande construite depuis le payload de l'app event. Même use case que le formulaire vierge, pas de use case séparé.

## Pas de use case pour les GET

Les endpoints GET (page, fragments cascade) sont de la lecture pure : ils interrogent les ports et construisent les VMs directement dans le handler.
