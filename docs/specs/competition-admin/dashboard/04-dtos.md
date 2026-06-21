# Dashboard — Phase 4 : Contrats de données ✅

## Partie commune (page hôte admin)

Partagée par tous les onglets. Définie dans `admin_page.rs`.

```rust
#[derive(Template)]
#[template(path = "admin-page.html")]
pub struct AdminPageTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub competition_name: String,
    pub season_name: String,
    pub admin_count: usize,
    pub active_tab: String,
    pub content: String,  // HTML du fragment d'onglet, rendu inline
}
```

## Fragment dashboard

Défini dans `dashboard.rs`. Rendu comme fragment (pas de `extends`).

```rust
#[derive(Template)]
#[template(path = "admin/dashboard.html")]
pub struct DashboardFragmentTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub alerts: Vec<DashboardAlertVm>,
    pub stats: Vec<DashboardStatVm>,
    pub progress: Vec<DashboardProgressVm>,
    pub activity: Vec<DashboardActivityVm>,
}
```

## View Models

```rust
pub struct DashboardAlertVm {
    pub message: String,
    pub action_label: String,
    pub action_url: String,
    pub level: String,  // "warn" | "info"
}

pub struct DashboardStatVm {
    pub icon: String,
    pub value: String,
    pub label: String,
    pub style: String,  // "default" | "warn" | "ok"
}

pub struct DashboardProgressVm {
    pub label: String,
    pub current: u32,
    pub total: u32,
    pub pct: u32,
    pub color: String,  // "blue" | "green" | "orange"
}

pub struct DashboardActivityVm {
    pub icon_type: String,  // "enroll" | "match" | "warn"
    pub text: String,
    pub time: String,
}
```

## DTO d'entrée

Aucun — le handler GET reçoit `space_id`, `competition_id`, `season_id` via le path uniquement.
