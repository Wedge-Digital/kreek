# BC `players` — Page de détail joueur (handler + template)

**Priorité : haute**
**Dépend de :** `156-players-stats-resolution-service.md`, `161-players-domain-match-concluded.md`, `162-players-persistence-match-concluded.md`, `165-players-match-history-service.md`
**Contexte :** `players/io/web` — nouveau handler + template, câblage de la maquette `assets/rawpages/html/app-player-detail-readonly.html`

## Objectif

Câbler la fiche joueur en lecture seule sur données réelles. Périmètre :
en-tête, résumé de carrière, portefeuille SPP, compétences, historique de
matchs, journal des évolutions. **Hors scope** : mode Customisation (JS de la
maquette non porté — feature à part), page de dépense de SPP (bouton présent,
non câblé — feature à part).

---

## Conception

### VMs — `player_detail_controller.rs`

```rust
pub struct PlayerDetailVm {
    pub player_id: String, pub team_id: String, pub team_name: String,
    pub name: String, pub jersey: Option<i16>, pub position_name: String,
    pub ma: u8, pub st: u8, pub ag: u8, pub pa: u8, pub av: u8,
    pub base_skills: Vec<String>, pub acquired_skills: Vec<String>,
    pub value_formatted: String,
    pub spp_earned: u32, pub spp_spent: u32, pub spp_reserve: u32, pub spp_percent: u8,
    pub matches_played: u16,
    pub career_touchdowns: u16, pub career_passes: u16, pub career_interceptions: u16,
    pub career_casualties: u16, pub career_mvps: u16,
    pub can_customise: bool,
    pub match_history: Vec<MatchHistoryCardVm>,
    pub evolution_log: Vec<EvolutionLogRowVm>,
}
pub struct MatchHistoryCardVm {
    pub opponent_name: String, pub round_label: String,
    pub result_label: String, pub result_css: String,
    pub team_score: u8, pub opponent_score: u8,
    pub actions: Vec<MatchActionLineVm>, pub subtotal_spp: u32,
}
pub struct MatchActionLineVm { pub icon: String, pub label: String, pub spp: Option<u32> }
pub struct EvolutionLogRowVm { pub label: String, pub mode_label: String, pub cost: String, pub value: String, pub origin: String }
```

### Icônes/libellés par type d'action (`MatchHistoryActionKind` → `MatchActionLineVm`)

| Kind | Icône | Libellé |
|---|---|---|
| Touchdown | 🏈 | Touchdown |
| Pass | 🎯 | Passe réussie |
| Interception | 🛡️ | Interception |
| Casualty | 🩸 | Sortie infligée |
| Mvp | ⭐ | MVP |
| Foul | 🟨 | Faute |
| Injury | 🤕 | Blessure |

Résultat de match (`result_label`/`result_css`) dérivé par comparaison des scores, même règle que `build_team_banner` (`match_report/io/web/builders.rs`) : Victoire/vert, Défaite/rouge, Nul/gris.

### Handler

```rust
pub async fn player_detail_controller(
    Path((space_id, player_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let player = state.players.repository.find_by_id(&PlayerId(player_id)).await?
        .ok_or(StatusCode::NOT_FOUND)?;
    let events = state.players.repository.find_events_by_id(&player.id).await?;
    let team = state.teams.team_repository.find_by_id(&player.team_id.0).await?
        .ok_or(StatusCode::NOT_FOUND)?;

    let can_customise = check_admin_rights(&state, &auth_session, &space_id, &team).await;
    let stats = player_stats_service::resolve_stats(&player, state.references.repository.as_ref());
    let match_history = match_history_service::build_match_history(&events);

    // ... assemblage PlayerDetailVm, cf. Phase 4 (spec)
}

async fn check_admin_rights(state: &AppState, auth: &AuthSession, space_id: &str, team: &Team) -> bool {
    // même pattern que competitions/io/web/admin/admin_page.rs :
    // SpaceProfile::SpaceAdmin via state.spaces.space_repository.find_member_profile
    // OU coach_id/coach_name dans team.competition_id → competition_repository.find_base_info().admin_ids/admin_names
}
```

`spp_spent` = somme de `acquired_skills[].spp_cost` ; `spp_reserve` = `spp_earned - spp_spent` ; `spp_percent` = `spp_spent * 100 / spp_earned` (0 si `spp_earned == 0`).

`evolution_log` : une ligne par `acquired_skills[]`, `origin` toujours `"Compétence initiale bonus"` pour l'instant (seule source existante, cf. discussion — la dépense de SPP en saison est une feature à part qui introduira une origine différente).

### Template `player-detail.html`

Adaptée de `assets/rawpages/html/app-player-detail-readonly.html`, **copiée telle quelle puis adaptée** (règle CLAUDE.md sur les déplacements de code) :
- Suppression de tout le JS Alpine (`x-data="playerDetail()"`, mode `view`/`customise`, panneau de customisation, tabs, formulaires) — page 100% rendue serveur, aucun état client
- Suppression du bloc "Roster" dans les métadonnées d'en-tête
- `spp-budget` : deux nombres distincts (gagnés / dépensés), plus barre + réserve
- Bouton "✏️ Customiser" : `{% if vm.can_customise %}` — visible mais mène vers une page pas encore construite (comme aujourd'hui avec la page de debug)
- Bouton "▶ Activer la dépense de SPP" : toujours visible, cible non câblée
- Historique de matchs : boucle sur `vm.match_history`
- Journal des évolutions : boucle sur `vm.evolution_log`, colonne "Obtenue" = `origin`

---

## Checklist

- [ ] VMs (`PlayerDetailVm` et sous-structures)
- [ ] `check_admin_rights()` (pattern `admin_page.rs`)
- [ ] Handler `player_detail_controller`
- [ ] Template `player-detail.html` (copié-adapté de la maquette, JS de customisation retiré)
- [ ] Test unitaire : calcul spp_spent/spp_reserve/spp_percent, mapping icônes/libellés par type d'action
