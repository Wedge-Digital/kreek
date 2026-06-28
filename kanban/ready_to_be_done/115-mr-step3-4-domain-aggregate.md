# BC match_report — Domain : champs agrégat, méthodes + rehydratation

**Priorité : haute**
**Dépend de :** 114
**Contexte :** match_report step3-4-actions — couche domaine

## Objectif

Étendre l'agrégat `MatchReportPreMatch` avec les champs, méthodes et branches de rehydratation pour les actions de match.

## Conception

Cf. `docs/specs/match-report/step3-4-actions/06-domaine.md`

### Nouveaux champs (`match_report_pre_match.rs`)

```rust
pub home_temp_players: Vec<TempPlayer>,
pub away_temp_players: Vec<TempPlayer>,
pub home_actions:      Vec<MatchAction>,
pub away_actions:      Vec<MatchAction>,
```

Valeurs initiales dans `from_draft` : `Vec::new()` pour les quatre.

### Nouvelles méthodes

- `init_temp_players(&self, team_id: &TeamId, players: Vec<TempPlayer>) -> (Self, MatchReportDomainEvent)`
  → met à jour home ou away selon team_id, émet `TempPlayersInitialized`
- `reset_temp_players(&self, team_id: &TeamId) -> (Self, MatchReportDomainEvent)`
  → vide home ou away, émet `TempPlayersReset`
- `record_action(&self, team_side, turn, player, action, player_display_name, action_id) -> (Self, MatchReportDomainEvent)`
  → push dans home ou away actions, émet `ActionRecorded` — aucune validation métier
- `delete_action(&self, action_id: &ActionId, deleted_by: CoachId) -> Result<(Self, MatchReportDomainEvent), DomainError>`
  → cherche dans home puis away, retire l'entrée, émet `ActionDeleted` ; Err si introuvable

### Méthodes de lecture

- `temp_players_for(&self, side: TeamSide) -> &[TempPlayer]`
- `star_player_uids_for(&self, team_id: &TeamId) -> Vec<InducementId>`
  → filtre les events `StarPlayerEngaged` rejoués dans l'agrégat (nécessite un champ `star_engagements: Vec<(TeamId, InducementId)>` alimenté à la rehydratation de `StarPlayerEngaged`)
- `purchases_for(&self, team_id: &TeamId) -> &[InducementPurchase]`
- `actions_for(&self, side: TeamSide) -> &[MatchAction]`

### Rehydratation (`match_report_state.rs`)

4 nouveaux bras dans `rehydrate()` — cf. `docs/specs/match-report/step3-4-actions/07-integration.md`.

## Checklist

- [ ] Champs `home_temp_players`, `away_temp_players`, `home_actions`, `away_actions` initialisés dans `from_draft`
- [ ] `init_temp_players` + `reset_temp_players`
- [ ] `record_action` (sans validation)
- [ ] `delete_action` → `Err(ActionNotFound)` si absent
- [ ] Méthodes de lecture : `temp_players_for`, `star_player_uids_for`, `purchases_for`, `actions_for`
- [ ] Bras rehydratation pour les 4 nouveaux events
- [ ] Tests unitaires : `record_action_pushes_to_home_actions`, `record_action_pushes_to_away_actions`, `record_two_actions_same_player_same_turn`, `record_two_mvp_same_team`, `delete_action_removes_entry`, `delete_action_fails_when_not_found`, `init_temp_players_sets_list`, `reset_temp_players_clears_list`, `star_player_uids_for_returns_engaged_uids`, `purchases_for_returns_team_inducements`, `actions_for_returns_correct_side`
