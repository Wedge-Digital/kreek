# Administration de compétition — Progression

## Maquettes (Phase 1 ✅)

Toutes dans `assets/rawpages/html/` :
- `app-competition-admin.html` — Tableau de bord
- `app-competition-admin-enrollments.html` — Inscriptions
- `app-competition-admin-groups.html` — Poules
- `app-competition-admin-schedule.html` — Calendrier
- `app-competition-admin-results.html` — Résultats
- `app-competition-admin-settings.html` — Paramètres

## Progression par page

| Page | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| Dashboard | ✅ | ✅ | | | | | |
| Inscriptions | ✅ | | | | | | |
| Poules | ✅ | | | | | | |
| Calendrier | ✅ | | | | | | |
| Résultats | ✅ | | | | | | |
| Paramètres | ✅ | | | | | | |

## Structure des specs

```
docs/specs/competition-admin/
├── README.md
├── dashboard/
│   ├── 02-front.md     ✅
│   └── 03-back.md      ✅
├── enrollments/
│   └── 02-front.md     ✅
├── groups/
│   └── 02-front.md     ✅
├── schedule/
│   └── 02-front.md     ✅
├── results/
│   └── 02-front.md     ✅
└── settings/
    └── 02-front.md     ✅
```

## Accès

- URL de base : `/app/{space_id}/competitions/{competition_id}/{season_id}/admin`
- Accès réservé aux administrateurs de l'espace et de la compétition

## Priorités

- **Haute** : Dashboard, Inscriptions, Poules, Calendrier
- **Moyenne** : Résultats
- **Basse** : Paramètres
