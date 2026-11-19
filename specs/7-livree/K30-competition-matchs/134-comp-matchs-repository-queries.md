# 134 — Repository : list_resultats et list_calendrier

## Objectif

Ajouter les deux méthodes de lecture sur `IMatchDayRepository` et leurs implémentations SQL pour alimenter les onglets Résultats et Calendrier.

## Dépendances

- 131 (table créée)

## Conception détaillée

### DTO de retour — `match_day_repository_port.rs`

Ajouter dans le fichier existant :

```rust
pub struct PairingDisplayDto {
    pub pairing_id: String,
    pub round_id: String,
    pub round_name: String,
    pub round_position: i32,
    pub round_date_start: Option<String>,
    pub round_date_end: Option<String>,
    pub round_day_type: String,
    pub home_team_name: String,
    pub home_roster_name: String,
    pub home_coach_name: String,
    pub home_logo_url: Option<String>,
    pub home_initials: String,
    pub away_team_name: String,
    pub away_roster_name: String,
    pub away_coach_name: String,
    pub away_logo_url: Option<String>,
    pub away_initials: String,
    pub match_status: String,
    pub home_score: Option<i32>,
    pub away_score: Option<i32>,
    pub home_casualties: Option<i32>,
    pub away_casualties: Option<i32>,
    pub match_report_url: Option<String>,
}
```

### Trait `IMatchDayRepository` — nouvelles méthodes

```rust
async fn list_resultats(
    &self,
    season_id: &str,
    cursor_position: Option<i32>,
    limit_rounds: u32,
) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError>;

async fn list_calendrier(
    &self,
    season_id: &str,
    cursor_position: Option<i32>,
    limit_rounds: u32,
) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError>;
```

### Fichiers SQL

`io/repository/sql/match_days/list_resultats.sql` :
```sql
SELECT *
FROM competition_match_display_proj
WHERE season_id = $1
  AND match_status IN ('in_progress', 'completed')
  AND ($2::integer IS NULL OR round_position < $2)
ORDER BY round_position DESC
LIMIT 500
```

`io/repository/sql/match_days/list_calendrier.sql` :
```sql
SELECT *
FROM competition_match_display_proj
WHERE season_id = $1
  AND match_status = 'upcoming'
  AND ($2::integer IS NULL OR round_position > $2)
ORDER BY round_position ASC
LIMIT 500
```

### Implémentation dans `match_day_repository.rs`

Utiliser `sqlx::query_as!` sur la struct `PairingDisplayDto` avec les fichiers SQL ci-dessus.

Le groupement par journée (max 3) est fait côté Rust dans le controller (pas dans le SQL).

## Checklist

- [ ] `PairingDisplayDto` ajouté dans `match_day_repository_port.rs`
- [ ] `list_resultats` et `list_calendrier` ajoutés au trait
- [ ] Fichiers SQL créés
- [ ] Implémentation dans `match_day_repository.rs`
- [ ] `cargo build` passe (sqlx vérifie les requêtes à la compilation)
