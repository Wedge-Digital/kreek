# BC `competitions` — Fragment dashboard admin (handler + template + VMs)

**Priorité : haute**
**Dépend de :** carte 82 (page hôte), carte 83 (query service)
**Contexte :** BC `competitions` — administration de compétition
**Spec :** `docs/specs/competition-admin/dashboard/04-dtos.md`, `07-integration.md`

## Objectif

Créer le handler fragment GET du dashboard, le template Askama, les VMs, et câbler le rendu inline dans la page hôte.

---

## Fichiers à créer

| Fichier | Rôle |
|---|---|
| `src/app/competitions/io/web/admin/dashboard.rs` | Handler GET + `DashboardFragmentTemplate` + VMs |
| `src/app/competitions/io/web/templates/admin/dashboard.html` | Template fragment dashboard |
| `assets/static/css/pages/competition-admin-dashboard.css` | Styles spécifiques dashboard |

## Fichiers à modifier

| Fichier | Modification |
|---|---|
| `src/app/competitions/io/web/admin/mod.rs` | Ajouter `pub mod dashboard;` |
| `src/app/competitions/io/web/admin/admin_page.rs` | Remplacer le placeholder par l'appel au query service + rendu inline |
| `src/app/competitions/routes.rs` | Ajouter route `/admin/dashboard` |
| `src/app/competitions/router.rs` | Câbler le handler fragment |

## Détails

### VMs

```rust
pub struct DashboardAlertVm {
    pub message: String,
    pub action_label: String,
    pub action_url: String,
    pub level: String,       // "warn" | "info"
}

pub struct DashboardStatVm {
    pub icon: String,
    pub value: String,
    pub label: String,
    pub style: String,       // "default" | "warn" | "ok"
}

pub struct DashboardProgressVm {
    pub label: String,
    pub current: u32,
    pub total: u32,
    pub pct: u32,
    pub color: String,       // "blue" | "green" | "orange"
}

pub struct DashboardActivityVm {
    pub icon_type: String,   // "enroll" | "match" | "warn"
    pub text: String,
    pub time: String,
}
```

### Handler fragment

Route : `GET /admin/dashboard`
1. Guard admin → 403
2. Appeler `dashboard_query::execute`
3. Construire les alertes à partir des compteurs (`pending_count > 0` → alerte warn)
4. Construire les stats chips
5. Construire les barres de progression (calculer le pourcentage)
6. Formater les `ActivityEntry` en `DashboardActivityVm` (temps relatif)
7. Rendre `DashboardFragmentTemplate`

### Câblage inline

Modifier `admin_page.rs` :
- Appeler le même code que le handler fragment
- Rendre le fragment, récupérer le HTML string
- Passer dans `content` de `AdminPageTemplate`

### Template `admin/dashboard.html`

Sections (cf. maquette `app-competition-admin.html`) :
- Alertes : boucle `{% for alert in alerts %}` avec `alert-banner alert-banner--{{ alert.level }}`
- Stats bar : boucle `{% for stat in stats %}` avec `stat-chip stat-chip--{{ stat.style }}`
- Deux colonnes : progression (boucle `progress`) + actions rapides (liens statiques vers les onglets)
- Activité récente : boucle `{% for entry in activity %}`

---

## Checklist

- [ ] Créer `dashboard.rs` avec VMs et `DashboardFragmentTemplate`
- [ ] Implémenter le handler fragment GET
- [ ] Créer `admin/dashboard.html` (reproduire la maquette en Askama)
- [ ] Créer `competition-admin-dashboard.css`
- [ ] Modifier `admin_page.rs` : remplacer le placeholder par le rendu inline
- [ ] Ajouter la route `/admin/dashboard` dans `routes.rs` et `router.rs`
- [ ] Vérifier dans le navigateur : le dashboard s'affiche correctement
