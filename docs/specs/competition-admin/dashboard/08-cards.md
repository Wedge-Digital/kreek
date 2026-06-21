# Dashboard — Phase 8 : Cartes kanban ✅

## Cartes produites

| # | Carte | Dépend de | Fichiers clés |
|---|---|---|---|
| 82 | Page hôte admin (layout commun) | — | `admin_page.rs`, `admin-page.html`, `competition-admin.css` |
| 83 | Query service dashboard | 82 | `dashboard_query.rs`, méthodes repository |
| 84 | Fragment dashboard (handler + template + VMs) | 82, 83 | `dashboard.rs`, `dashboard.html`, VMs |
| 85 | Tests E2E dashboard | 84 | `test_competition_admin_dashboard.py` |

## Ordre d'implémentation

```
82 (page hôte) → 83 (query service) → 84 (fragment) → 85 (E2E)
```

Cartes dans `kanban/ready_to_be_done/`.
