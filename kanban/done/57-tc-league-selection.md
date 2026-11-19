# BC `team_creation` — Rattachement à une ligue

**Priorité : haute**
**Dépend de :** `54-ref-league-selector-widget.md`
**Contexte :** `team_creation` — action coach

## Objectif

Permettre au coach de rattacher son équipe à une ligue pendant la phase de finalisation. Le choix est persisté dans le draft et inclus dans `TeamSubmitted`. L'UI est déléguée au widget `références` (carte 54) via HTMX.

---

## Conception

### Modèle domaine

`LeagueId` est un newtype dupliqué dans `team_creation` — pas partagé avec `références`. Partager un type entre BCs crée un couplage de compilation : une évolution de `références::LeagueId` impacterait `team_creation`. Le `String` sous-jacent assure l'interopérabilité sans couplage structurel.

```rust
// team_creation/domain/model/
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeagueId(pub String);
```

### Événement domaine

```rust
pub struct LeagueSet {
    pub team_id:   TeamId,
    pub league_id: LeagueId,
}
```

### Commande et use case

```rust
pub struct SetLeagueCommand {
    pub team_id:   TeamId,
    pub league_id: LeagueId,
}
```

```rust
pub enum SetLeagueError {
    TeamNotFound,
    Repository(RepositoryError),
}

pub async fn execute(
    cmd:       SetLeagueCommand,
    team_repo: &dyn ITeamCreationRepository,
) -> Result<(), SetLeagueError>
```

Logique :
1. Charger le draft
2. Appender `LeagueSet` + mettre à jour la projection
3. Persister
4. Retourner le widget sélecteur mis à jour (état fermé avec la nouvelle ligue)

### Route et handler

```
POST /team-creation/{draft_id}/league
Body : { "league_id": "WOODLAND_LEAGUE" }
```

**Réponse succès** — le handler retourne `HX-Trigger` avec la nouvelle ligue. Le container dans la page hôte écoute l'événement et se recharge depuis `références` :

```rust
Response::builder()
    .header("HX-Trigger", serde_json::json!({
        "leagueSelected": { "league_id": cmd.league_id.0 }
    }).to_string())
    .body(Body::empty())
    .unwrap()
```

Le container dans la page hôte :

```html
<div id="league-selector-container"
     hx-get="/références/leagues/selector"
     hx-trigger="load, leagueSelected from:body"
     hx-vals="js:{
       selected:   (event.detail?.league_id ?? '{{ draft.league_id }}'),
       on_select:  '/team-creation/{{ draft_id }}/league'
     }"
     hx-target="this"
     hx-swap="outerHTML">
</div>
```

### Chargement du widget dans la page hôte

```html
<div id="league-selector-container"
     hx-get="/références/leagues/selector
             ?selected={{ draft.league_id }}
             &on_select=/team-creation/{{ draft_id }}/league"
     hx-trigger="load"
     hx-target="this"
     hx-swap="outerHTML">
</div>
```

### Impact sur `TeamSubmitted`

L'event `TeamSubmitted` (carte 59) inclut désormais `league_id`. La validation de soumission doit vérifier qu'une ligue a été sélectionnée avant de permettre la soumission.

```rust
// Validation domaine ajoutée dans validate_for_submission() — obligatoire
if self.league_id.is_none() {
    errors.push(DomainError::LeagueNotSelected);
}
```

---

## Points à préciser


---

## Checklist

- [ ] `LeagueId` newtype dans `team_creation/domain/model/`
- [ ] `DomainError::LeagueNotSelected`
- [ ] `LeagueSet` event
- [ ] `SetLeagueCommand` + use case
- [ ] Route `SET_LEAGUE` dans `routes.rs` + `router.rs`
- [ ] Handler POST : persiste + retourne widget mis à jour (option A ou B)
- [ ] Validation de soumission mise à jour (`validate_for_submission`) — carte 59
- [ ] Container `#league-selector-container` dans le template de la page de finalisation
