# Onglets Résultats & Calendrier — Phase 4 : Contrats de données

---

## DTOs d'entrée (query params)

Partagé par les deux controllers :

```rust
// Émis par : HTMX (query string)
// Consommé par : resultats_tab_controller, calendrier_tab_controller
pub struct TabCursorQuery {
    pub cursor: Option<String>,  // journee_id absent = chargement initial (3 premières journées)
}
```

---

## DTOs de repository (lecture depuis `competition_match_display_proj`)

```rust
// Émis par : match_day_repository (list_resultats / list_calendrier)
// Consommé par : resultats_tab_controller, calendrier_tab_controller

pub struct PairingDisplayDto {
    pub pairing_id: String,
    pub round_id: String,
    pub round_name: String,
    pub round_position: i32,
    pub round_date_start: Option<String>,
    pub round_date_end: Option<String>,
    pub round_day_type: String,          // "fixed_date" | "time_frame" | "rest"
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
    pub match_status: String,            // "upcoming" | "in_progress" | "completed"
    pub home_score: Option<i32>,
    pub away_score: Option<i32>,
    pub home_casualties: Option<i32>,
    pub away_casualties: Option<i32>,
    pub match_report_url: Option<String>,
}
```

---

## View Models (sortie vers les templates Askama)

### Onglet Résultats

```rust
// Émis par : resultats_tab_controller
// Consommé par : competition-tab-resultats.html

pub enum MatchStatusVm {
    Completed {
        home_score: u32,
        away_score: u32,
        home_cas: u32,
        away_cas: u32,
    },
    InProgress {
        report_url: String,
    },
}

pub struct MatchResultatVm {
    pub home_name: String,
    pub home_roster: String,
    pub home_coach: String,
    pub home_logo: Option<String>,
    pub home_initials: String,
    pub away_name: String,
    pub away_roster: String,
    pub away_coach: String,
    pub away_logo: Option<String>,
    pub away_initials: String,
    pub status: MatchStatusVm,
    pub date: String,
}

pub struct JourneeResultatsVm {
    pub label: String,              // "Journée 8"
    pub matches: Vec<MatchResultatVm>,
}
```

### Onglet Calendrier

```rust
// Émis par : calendrier_tab_controller
// Consommé par : competition-tab-calendrier.html

pub struct MatchCalendrierVm {
    pub home_name: String,
    pub home_logo: Option<String>,
    pub home_initials: String,
    pub away_name: String,
    pub away_logo: Option<String>,
    pub away_initials: String,
    pub date: String,               // date du match individuel (si fixe) ou vide
}

pub struct JourneeCalendrierVm {
    pub label: String,              // "Journée 9"
    pub date_range: String,         // "25 – 26 mai" | "25 mai" | "" (calculé depuis round_date_*)
    pub match_count: usize,
    pub matches: Vec<MatchCalendrierVm>,
}
```

### Template structs Askama

```rust
// Émis par : resultats_tab_controller
// Consommé par : competition-tab-resultats.html
#[derive(Template)]
#[template(path = "competition-tab-resultats.html")]
pub struct ResultatsTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub journees: Vec<JourneeResultatsVm>,
    pub next_cursor: Option<String>,  // None = dernière page (pas de sentinel)
    pub is_initial: bool,             // true = enveloppe dans #resultats-list
}

// Émis par : calendrier_tab_controller
// Consommé par : competition-tab-calendrier.html
#[derive(Template)]
#[template(path = "competition-tab-calendrier.html")]
pub struct CalendrierTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub journees: Vec<JourneeCalendrierVm>,
    pub next_cursor: Option<String>,
    pub is_initial: bool,
}
```

---

## Mapping DTO → VM (dans les controllers)

### `date_range` (Calendrier)

Calculé dans le controller à partir des champs `round_day_type`, `round_date_start`, `round_date_end` :

```rust
fn format_date_range(day_type: &str, start: Option<&str>, end: Option<&str>) -> String {
    match day_type {
        "fixed_date" => start.unwrap_or("").to_string(),
        "time_frame" => match (start, end) {
            (Some(s), Some(e)) => format!("{} – {}", s, e),
            (Some(s), None) => s.to_string(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}
```

### `next_cursor`

Après la requête repository, `next_cursor` = `round_id` de la dernière journée du lot si le nombre de journées retournées == limit (3). Sinon `None`.

### Groupement par journée

Les `PairingDisplayDto` sont retournés triés par `round_position`. Le controller les groupe par `round_id` pour construire les `Vec<JourneeResultatsVm>` / `Vec<JourneeCalendrierVm>`.

---

## Interfaces d'utilisation (émetteur → consommateur)

| DTO / VM | Émetteur | Consommateur |
|---|---|---|
| `TabCursorQuery` | HTMX (query string) | `resultats_tab_controller`, `calendrier_tab_controller` |
| `PairingDisplayDto` | `match_day_repository` | `resultats_tab_controller`, `calendrier_tab_controller` |
| `JourneeResultatsVm` | `resultats_tab_controller` | `ResultatsTabTemplate` → `competition-tab-resultats.html` |
| `MatchStatusVm` | `resultats_tab_controller` | `MatchResultatVm` |
| `JourneeCalendrierVm` | `calendrier_tab_controller` | `CalendrierTabTemplate` → `competition-tab-calendrier.html` |
| `ResultatsTabTemplate` | `resultats_tab_controller` | Askama → HTML fragment |
| `CalendrierTabTemplate` | `calendrier_tab_controller` | Askama → HTML fragment |
