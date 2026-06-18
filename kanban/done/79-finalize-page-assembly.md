# BC `team_creation` — Template finalize-team comme page d'assemblage

**Priorité : haute**
**Dépend de :** `76`, `77`, `78`
**Contexte :** BC `team_creation` — page hôte finalize-team

## Objectif

Réécrire `finalize-team.html` en page d'assemblage pure (pattern build-team). Supprimer les ~170 lignes d'Alpine JS (`finalizePage()`, `skillPicker()`). Chaque section est un widget ou un fragment inline mis à jour par événements DOM.

---

## Conception

### 7 sections

| Section | BC | Rendu | Événement |
|---|---|---|---|
| A — Header équipe | team_creation | Inline (logo, nom, budget) | — |
| B — League selector | références | `hx-get` + `hx-trigger="load"` | — |
| C — Liste joueurs | team_creation | Inline, `playerSelected` au clic | Émet `playerSelected` |
| D — Skill header | team_creation | `hx-trigger="playerSelected, skillsUpdated"` | Écoute |
| E — Skill picker | références | `hx-trigger="playerSelected, skillsUpdated"` | Écoute |
| F — SPP summary | team_creation | Inline, mis à jour par OOB (via spend/cancel) | — |
| G — Submit button | team_creation | Inline, `hx-post` | — |

### Événement `playerSelected`

Dispatché par un clic sur une ligne joueur dans la section C :

```html
<div class="player-row"
     onclick="htmx.trigger(document.body, 'playerSelected', {
       player_id: '{{ player.id }}',
       roster_line_id: '{{ player.roster_line_id }}',
       spp: {{ spp_pool }},
       acquired: '{{ player.acquired_csv }}',
       on_acquire: '{{ team_routes.spend_spp(space_id, team_id, player.id) }}',
       on_cancel: '{{ team_routes.spend_spp(space_id, team_id, player.id) }}'
     })">
```

### `FinalizeTeamTemplate` — état final

```rust
pub struct FinalizeTeamTemplate {
    pub web_routes: WebRoutes,
    pub team_routes: Routes,
    pub ref_routes: RefRoutes,
    pub space_id: String,
    pub team_id: String,
    pub logo_url: Option<String>,
    pub team_name: String,
    pub roster_name: String,
    pub treasury: u32,
    pub spp_pool: u8,
    pub players: Vec<FinalizePlayerVm>,
    pub spp_log: Vec<SppLogEntryVm>,
}
```

### Handler GET simplifié

Le handler ne passe plus `data_json` — les données sont structurées dans le template Askama via des VMs typés.

### JS résiduel

Comme pour build-team, un script minimal pour stocker le `player_id` sélectionné dans `body.dataset` (nécessaire pour le rechargement post-skillsUpdated).

---

## Situation finale

- `finalize-team.html` est une page d'assemblage, quasi zéro JS
- Le méga Alpine `finalizePage()` (~120 lignes) est supprimé
- Le méga Alpine `skillPicker()` (~45 lignes) est supprimé (déjà dans le widget BC references)
- Communication par événements DOM : `playerSelected`, `skillsUpdated`, `leagueSelected`
- Le handler GET ne passe plus de `data_json`

---

## Checklist

- [ ] Créer les VMs `FinalizePlayerVm`, `SppLogEntryVm`
- [ ] Réécrire `FinalizeTeamTemplate` avec les champs typés
- [ ] Réécrire le handler GET pour passer les VMs au lieu de `data_json`
- [ ] Réécrire `finalize-team.html` : sections A-G avec `hx-get`/`hx-trigger`
- [ ] Section C : dispatch `playerSelected` au clic
- [ ] Section D : `hx-trigger="playerSelected, skillsUpdated"`
- [ ] Section E : `hx-trigger="playerSelected, skillsUpdated"`
- [ ] Section B : `hx-trigger="load"`
- [ ] Section F : SPP summary inline
- [ ] Section G : submit button avec `hx-post`
- [ ] Supprimer `finalizePage()` et `skillPicker()` du template
- [ ] Supprimer `FinalizeData`, `PlayerJson`, `PricingJson` de `finalize_team.rs`
- [ ] Script résiduel pour `body.dataset.playerId`
