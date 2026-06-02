# BC `team_creation` — Page de finalisation et soumission

**Priorité : haute**
**Dépend de :** `54`, `55`, `56`, `57`, `58`
**Contexte :** `team_creation` — page hôte

## Objectif

Assembler les 7 cartes de la page de finalisation dans un template `team_creation`. Câbler l'événement `playerSelected` qui synchronise les deux panneaux droits (carte D depuis `team_creation`, carte E depuis `références`) sans couplage entre BCs. Mettre à jour `TeamSubmitted` pour inclure les nouvelles données collectées durant la finalisation.

---

## Conception

### Route de la page

```
GET /team-creation/{draft_id}/finalize
```

Charge l'agrégat draft complet et rend la page avec les données initiales. Les deux widgets `références` (cartes B et E) sont chargés via `hx-trigger="load"`.

### Composition des 7 cartes

| Carte | BC | Rendu |
|---|---|---|
| A — Header équipe | `team_creation` | Inline (template Askama) |
| B — Sélecteur de ligue | `références` | `hx-get` au chargement |
| C — Liste joueurs | `team_creation` | Inline |
| D — Header joueur | `team_creation` | Fragment, chargé sur `playerSelected` |
| E — Catalogue compétences | `références` | Fragment, chargé sur `playerSelected` |
| F — Synthèse SPP | `team_creation` | Inline, mis à jour par OOB swaps (carte 58) |
| G — Barre de soumission | `team_creation` | Inline |

### Événement `playerSelected`

Dispatché par le clic sur une ligne joueur dans le panneau gauche (carte C). Porte toutes les données nécessaires aux deux cartes cibles :

```javascript
// Inline dans le template, sur chaque player-row
htmx.trigger(document.body, 'playerSelected', {
    player_id:      '{{ player.id }}',
    roster_line_id: '{{ player.roster_line_id }}',
    spp:            {{ player.spp_remaining }},
    acquired:       '{{ player.acquired_skill_ids_csv }}',
    on_acquire:     '{{ routes.spend_spp(draft_id, player.id) }}',
    on_cancel:      '{{ routes.cancel_spp_base(draft_id, player.id) }}'
});
```

Carte D écoute `playerSelected` et se charge depuis `team_creation` :

```html
<div id="skill-header"
     hx-get="{{ routes.skill_header(draft_id) }}"
     hx-trigger="playerSelected from:body"
     hx-vals="js:{player_id: event.detail.player_id}"
     hx-target="this"
     hx-swap="outerHTML">
  <!-- État vide initial -->
  <div class="skill-panel-empty">
    <div class="skill-panel-empty-icon">⬆</div>
    Sélectionnez un joueur pour gérer ses compétences
  </div>
</div>
```

Carte E écoute le même événement et se charge depuis `références`. Les deux triggers (`playerSelected` et `skillsUpdated`) portent les mêmes clés dans `event.detail` — le container n'a aucun état à mémoriser :

```html
<div id="skill-picker-container"
     hx-get="/références/roster-lines/skill-picker"
     hx-trigger="playerSelected from:body, skillsUpdated from:body"
     hx-vals="js:{
       roster_line_id: event.detail.roster_line_id,
       spp:            event.detail.spp,
       acquired:       event.detail.acquired,
       on_acquire:     event.detail.on_acquire,
       on_cancel:      event.detail.on_cancel
     }"
     hx-target="this"
     hx-swap="outerHTML">
</div>
```

Le handler `spend_creation_spp` (carte 58) inclut tous ces champs dans le payload `skillsUpdated` puisqu'il connaît le `player_id` de la requête et peut reconstruire les URLs de callback :

```rust
.header("HX-Trigger", serde_json::json!({
    "skillsUpdated": {
        "roster_line_id": player.roster_line_id,
        "spp":            team.spp_pool.0,
        "acquired":       player.acquired_skill_ids_csv(),
        "on_acquire":     routes.spend_spp(draft_id, player.id),
        "on_cancel":      routes.cancel_spp_base(draft_id, player.id)
    }
}).to_string())

### Route du header joueur (carte D)

```
GET /team-creation/{draft_id}/players/skill-header?player_id={id}
```

Rend le fragment : numéro + nom éditables (inputs auto-save), position · roster, compétences existantes en tags, compteur SPP restants.

### Mise à jour de `TeamSubmitted`

L'event `TeamSubmitted` existant est enrichi avec les données collectées pendant la finalisation :

```rust
pub struct TeamSubmittedEvent {
    // champs existants conservés
    pub team_id:      TeamId,
    pub team_name:    String,
    pub space_id:     String,
    pub roster_id:    RosterId,
    pub coach_id:     CoachId,
    pub total_cost:   Kpo,

    // nouveaux champs
    pub league_id:    LeagueId,              // carte 57
    pub players:      Vec<PlayerSnapshot>,   // enrichi avec cartes 56 + 58
}

pub struct PlayerSnapshot {
    pub player_id:       PlayerId,
    pub roster_line_id:  RosterLineId,
    pub name:            PlayerName,         // carte 56 (peut être vide)
    pub jersey:          JerseyNumber,       // carte 56
    pub acquired_skills: Vec<AcquiredSkill>, // carte 58
}

pub struct AcquiredSkill {
    pub skill_id: SkillId,
    pub mode:     AcquisitionMode,
    pub spp_cost: SppAmount,
}
```

### Validation de soumission mise à jour

```rust
pub fn validate_for_submission(&self) -> Result<(), Vec<DomainError>> {
    let mut errors = Vec::new();
    if self.players.len() < MIN_PLAYERS_FOR_SUBMISSION {
        errors.push(DomainError::InsufficientPlayerCount);
    }
    if self.league_id.is_none() {
        errors.push(DomainError::LeagueNotSelected);
    }
    // Nom de joueur absent : silencieux — donnée cosmétique, pas de contrainte domaine
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
```

### Barre de soumission (carte G)

```html
<div class="submit-bar">
  <button class="btn-submit"
          hx-post="{{ routes.submit_team(draft_id) }}"
          hx-target="#submit-errors"
          hx-swap="innerHTML">
    Soumettre l'équipe →
  </button>
  <div id="submit-errors"></div>
</div>
```

| `DomainError` | Message FR |
|---|---|
| `InsufficientPlayerCount` | Vous devez engager au moins 11 joueurs pour soumettre votre équipe. |
| `LeagueNotSelected` | Veuillez sélectionner une ligue avant de soumettre. |

---

## Points à préciser


---

## Checklist

- [ ] Route `GET /team-creation/{draft_id}/finalize` + handler + template `finalize.html`
- [ ] Route `GET /team-creation/{draft_id}/players/skill-header` + handler + template `skill-header-fragment.html`
- [ ] Dispatch `playerSelected` câblé sur chaque `.player-row` (JS inline dans le template)
- [ ] Container carte D (`#skill-header`) avec `hx-trigger="playerSelected from:body"`
- [ ] Container carte E (`#skill-picker-container`) avec `hx-trigger="playerSelected from:body, skillsUpdated from:body"`
- [ ] Container carte B (`#league-selector-container`) avec `hx-trigger="load"`
- [ ] `PlayerSnapshot` + `AcquiredSkill` dans le domaine `team_creation`
- [ ] `TeamSubmittedEvent` enrichi : `league_id` + `players` avec nom/jersey/compétences
- [ ] `validate_for_submission()` mis à jour : `LeagueNotSelected`
- [ ] Barre de soumission câblée avec gestion d'erreurs domaine
- [ ] Sélection visuelle du joueur actif dans le panneau gauche (classe `selected`, géré en JS pur — pas de session serveur)
