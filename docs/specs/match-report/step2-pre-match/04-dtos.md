# Step 2 — Avant-match — Contrats de données

## DTO d'entrée (POST soumission fan factor)

```rust
#[derive(Deserialize)]
pub struct RecordFanFactorForm {
    pub home_fan_roll: u8,  // 1, 2 ou 3
    pub away_fan_roll: u8,  // 1, 2 ou 3
}
```

Validé dans le handler : valeur hors {1, 2, 3} → 400 Bad Request.

## Commande use case

```rust
pub struct RecordFanFactorCommand {
    pub match_report_id: MatchReportId,
    pub home_fan_roll: D3Roll,   // value object
    pub away_fan_roll: D3Roll,   // value object
    pub recorded_by: CoachId,
}
```

## Value object D3Roll

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct D3Roll(u8);

impl D3Roll {
    pub fn try_new(value: u8) -> Result<Self, DomainError> {
        if (1..=3).contains(&value) { Ok(Self(value)) }
        else { Err(DomainError::InvalidD3Roll(value)) }
    }
    pub fn value(&self) -> u8 { self.0 }
}
```

## Domain event

```rust
FanFactorRecorded {
    home_fan_roll: D3Roll,
    away_fan_roll: D3Roll,
    recorded_by: CoachId,
}
```

Ajouté à l'enum `MatchReportDomainEvent`.

## VM sortie (template page GET)

```rust
pub struct PreMatchTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub match_report_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
    pub home_team_name: String,
    pub away_team_name: String,
    pub home_coach_name: String,
    pub away_coach_name: String,
    pub home_roster_name: String,
    pub away_roster_name: String,
    pub home_team_context_url: String,  // URL endpoint JSON BC Teams
    pub away_team_context_url: String,  // URL endpoint JSON BC Teams
}
```

Les données dynamiques (dedicated fans, player count, CTV, treasury, journeyman type) ne sont PAS dans le template — elles sont chargées côté client via `fetch()` vers les URLs JSON.

## DTO JSON (BC Teams — endpoint match-context)

```rust
#[derive(Serialize)]
pub struct TeamMatchContextJson {
    pub team_id: String,
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
    pub dedicated_fans: u32,
    pub player_count: u32,
    pub ctv: u32,
    pub treasury: u32,
    pub journeyman_type: String,
}
```

Ce DTO est retourné par l'endpoint JSON du BC Teams. Consommé uniquement côté client (JS).
