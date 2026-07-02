# Step 5 — Domaine

## Règles métier validées

1. Le score est déduit du journal d'actions : comptage des `Touchdown` par side — pas de saisie manuelle
2. Les sorties = comptage des `Sortie` uniquement (pas `Blesse { .. }`) par side
3. La suggestion de gain : `(fans_home + fans_away) / 2 × 10 000 + nb_tds × 10 000` (division entière)
4. `suggest_gains()` est une méthode d'agrégat pure — les `dedicated_fans` sont stockés dans l'agrégat au moment de `FanFactorRecorded` (Option B)
5. Le gain saisi doit être > 0
6. Le fan factor modifier est dans \[-2 ; +2\] inclus
7. Le résumé (titre + corps) est optionnel — aucune validation domaine
8. La soumission n'est possible que depuis `PreMatch` ou `ReadyToPublish`
9. La soumission transite vers `ReadyToPublish` via `PostMatchRecorded`
10. La re-soumission écrase les données précédentes

---

## Nouveaux value objects (`domain/value_objects.rs`)

```rust
#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct MatchGain(u32);

#[nutype(
    validate(greater_or_equal = -2, less_or_equal = 2),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct FanFactorMod(i8);
```

---

## Impact sur le flow step 2 (Option B)

### `domain/events.rs` — enrichissement de `FanFactorRecorded`

Deux champs ajoutés avec `#[serde(default)]` pour la compatibilité avec les événements déjà persistés
(les anciens événements désérialisent `dedicated_fans = 0` — acceptable, la suggestion reste approximative
pour les anciens rapports mais les valeurs réellement enregistrées ne sont pas affectées).

```rust
FanFactorRecorded {
    home_fan_roll: D3Roll,
    away_fan_roll: D3Roll,
    #[serde(default)]
    home_dedicated_fans: u32,
    #[serde(default)]
    away_dedicated_fans: u32,
    recorded_by: CoachId,
}
```

### `domain/match_report_pre_match.rs` — nouveaux champs

```rust
pub home_dedicated_fans: u32,
pub away_dedicated_fans: u32,
```

Alimentés lors de la réhydration de `FanFactorRecorded`.

### `ports.rs` — enrichissement de `TeamInfoDto`

```rust
pub struct TeamInfoDto {
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
    pub roster_id: String,
    pub logo_url: Option<String>,
    pub dedicated_fans: u32,    // nouveau
}
```

### `use_cases/record_fan_factor_use_case.rs` — impact

La commande `RecordFanFactorCommand` reçoit deux nouveaux champs :

```rust
pub home_dedicated_fans: u32,
pub away_dedicated_fans: u32,
```

Le use case les récupère via `ITeamDataPort::find_team_info` (déjà appelé) et les passe à
`MatchReportPreMatch::record_fan_factor(...)`.

La signature de `record_fan_factor` sur l'agrégat est enrichie en conséquence.

---

## Nouvelles méthodes sur `MatchReportPreMatch`

```rust
pub fn compute_score(&self) -> (u8, u8) {
    let home = self.home_actions.iter()
        .filter(|a| matches!(a.action, MatchActionType::Touchdown))
        .count() as u8;
    let away = self.away_actions.iter()
        .filter(|a| matches!(a.action, MatchActionType::Touchdown))
        .count() as u8;
    (home, away)
}

pub fn compute_cas(&self) -> (u8, u8) {
    let home = self.home_actions.iter()
        .filter(|a| matches!(a.action, MatchActionType::Sortie))
        .count() as u8;
    let away = self.away_actions.iter()
        .filter(|a| matches!(a.action, MatchActionType::Sortie))
        .count() as u8;
    (home, away)
}

pub fn suggest_gains(&self) -> (u32, u32) {
    let fans_home = self.home_dedicated_fans + self.home_fan_roll.map(|r| r.value() as u32).unwrap_or(0);
    let fans_away = self.away_dedicated_fans + self.away_fan_roll.map(|r| r.value() as u32).unwrap_or(0);
    let (tds_home, tds_away) = self.compute_score();
    let base = (fans_home + fans_away) / 2 * 10_000;
    (base + tds_home as u32 * 10_000, base + tds_away as u32 * 10_000)
}

pub fn record_post_match(
    &self,
    home_gain: MatchGain,
    away_gain: MatchGain,
    home_fan_mod: FanFactorMod,
    away_fan_mod: FanFactorMod,
    summary_title: Option<String>,
    summary_body: Option<String>,
    recorded_by: CoachId,
) -> (MatchReportReadyToPublish, MatchReportDomainEvent) {
    let event = MatchReportDomainEvent::PostMatchRecorded {
        home_gain, away_gain, home_fan_mod, away_fan_mod,
        summary_title: summary_title.clone(),
        summary_body: summary_body.clone(),
        recorded_by,
    };
    let ready = MatchReportReadyToPublish::from_pre_match(
        self, home_gain, away_gain, home_fan_mod, away_fan_mod,
        summary_title, summary_body,
    );
    (ready, event)
}
```

---

## Nouvel agrégat `MatchReportReadyToPublish` (`domain/match_report_ready_to_publish.rs`)

```rust
#[derive(Debug, Clone)]
pub struct MatchReportReadyToPublish {
    // tous les champs de MatchReportPreMatch
    pub id: MatchReportId,
    pub space_id: SpaceId,
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
    pub created_by: CoachId,
    pub home_fan_roll: Option<D3Roll>,
    pub away_fan_roll: Option<D3Roll>,
    pub home_dedicated_fans: u32,
    pub away_dedicated_fans: u32,
    pub home_actions: Vec<MatchAction>,
    pub away_actions: Vec<MatchAction>,
    pub version: u64,
    // champs post-match
    pub home_gain: MatchGain,
    pub away_gain: MatchGain,
    pub home_fan_mod: FanFactorMod,
    pub away_fan_mod: FanFactorMod,
    pub summary_title: Option<String>,
    pub summary_body: Option<String>,
}
```

Méthode `record_post_match` pour la re-soumission :

```rust
pub fn record_post_match(
    &self,
    home_gain: MatchGain,
    away_gain: MatchGain,
    home_fan_mod: FanFactorMod,
    away_fan_mod: FanFactorMod,
    summary_title: Option<String>,
    summary_body: Option<String>,
    recorded_by: CoachId,
) -> (Self, MatchReportDomainEvent)
```

Même logique que sur `MatchReportPreMatch` — produit un nouveau `PostMatchRecorded` qui écrase les champs.

---

## `domain/match_report_state.rs` — nouvelle variante

```rust
pub enum MatchReportState {
    Draft(MatchReportDraft),
    PreMatch(MatchReportPreMatch),
    ReadyToPublish(MatchReportReadyToPublish),
    Cancelled(MatchReportCancelled),
}
```

Réhydration de `PostMatchRecorded` :
- Sur `PreMatch` → transition vers `ReadyToPublish`
- Sur `ReadyToPublish` → mise à jour des champs post-match (re-soumission)

---

## Tests prévus (`domain/match_report_pre_match.rs`)

| Test | Règle couverte |
|---|---|
| `compute_score_counts_touchdowns_by_side` | Règle 1 |
| `compute_score_ignores_other_actions` | Règle 1 |
| `compute_cas_counts_only_sortie` | Règle 2 |
| `compute_cas_ignores_blesse` | Règle 2 |
| `suggest_gains_applies_formula` | Règle 3 |
| `suggest_gains_with_zero_tds` | Règle 3 |
| `suggest_gains_integer_division` | Règle 3 (division entière) |
| `record_post_match_emits_correct_event` | Règle 9 |
| `fan_factor_mod_rejects_out_of_range` | Règle 6 |
| `fan_factor_mod_accepts_boundaries` | Règle 6 |
| `match_gain_rejects_zero` | Règle 5 |
| `match_gain_accepts_positive` | Règle 5 |
