# Dashboard — Phase 3 : Architecture back

## BC responsable

Le dashboard est entièrement fourni par le BC `competitions`. Pas de données inter-BC — toutes les informations (équipes inscrites, matchs, journées) sont des projections locales au BC `competitions`.

## Page hôte

Le dashboard est un onglet de la page d'administration. La page hôte `/admin` rend le banner + tabs. Le contenu de chaque onglet est un fragment chargé par HTMX.

### Routes

```
GET /app/{space_id}/competitions/{competition_id}/{season_id}/admin
    → page hôte (banner + tabs), redirige vers /admin/dashboard par défaut

GET /app/{space_id}/competitions/{competition_id}/{season_id}/admin/dashboard
    → fragment contenu du dashboard
```

### Fichiers

```
src/app/competitions/io/web/
├── admin/
│   ├── mod.rs
│   ├── admin_page.rs              ← page hôte (banner + tabs)
│   └── dashboard.rs               ← handler GET dashboard fragment
├── templates/
│   ├── admin-page.html            ← layout admin (banner + tabs + #admin-content)
│   └── admin/
│       └── dashboard.html         ← fragment dashboard
└── ...

src/app/competitions/routes.rs     ← routes ajoutées
src/app/competitions/router.rs     ← routes câblées
```

## Handler dashboard

Le handler `dashboard` est un GET qui :
1. Charge les données de synthèse depuis les repositories du BC `competitions`
2. Construit les VMs
3. Retourne le fragment HTML

Données nécessaires :
- **Alertes** : nombre d'inscriptions en attente, journée en attente de validation
- **Stats** : nb équipes inscrites, en attente, matchs joués, matchs restants, journées validées
- **Progression** : ratios inscriptions/places, matchs joués/total, journées validées/total
- **Activité récente** : derniers événements (inscriptions, résultats soumis, validations)

## Ports nécessaires

Aucun — le dashboard ne consomme que des données du BC `competitions`.

## Domain services nécessaires

Aucun — le dashboard est en lecture seule, pas de transformation port → domaine.

## Middleware d'autorisation

Un middleware ou un guard vérifie que l'utilisateur est admin de l'espace OU admin de la compétition. Ce guard est partagé par tous les onglets admin.

```rust
// Vérification dans le handler ou via un extracteur dédié
let is_space_admin = /* check space admin */;
let is_comp_admin = competition.admin_ids.contains(&user.id);
if !is_space_admin && !is_comp_admin {
    return StatusCode::FORBIDDEN;
}
```
