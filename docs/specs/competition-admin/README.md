# Administration de compétition — Progression

## Maquettes

Toutes dans `assets/rawpages/html/` :
- `app-competition-admin.html` — Tableau de bord
- `app-competition-admin-enrollments.html` — Inscriptions
- `app-competition-admin-groups.html` — Poules
- `app-competition-admin-schedule.html` — Calendrier
- `app-competition-admin-results.html` — Résultats
- `app-competition-admin-settings.html` — Paramètres

## Progression par page

| Page | Maquette | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|---|
| Dashboard | ✅ | ✅ | | | | | | |
| Inscriptions | ✅ | ✅ | | | | | | |
| Poules | ✅ | ✅ | | | | | | |
| Calendrier | ✅ | ✅ | | | | | | |
| Résultats | ✅ | ✅ | | | | | | |
| Paramètres | ✅ | ✅ | | | | | | |

## Accès

- URL de base : `/app/{space_id}/competitions/{competition_id}/{season_id}/admin`
- Accès réservé aux administrateurs de l'espace et de la compétition

## Priorités

- **Haute** : Dashboard, Inscriptions, Poules, Calendrier
- **Moyenne** : Résultats
- **Basse** : Paramètres (modifier règles), Tableau de bord (stats détaillées)
