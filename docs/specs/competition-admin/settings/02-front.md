# Paramètres — Admin compétition

## Phase 2 — Architecture front ✅

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| Infos générales | competitions | inline (formulaire) | — | — | Édition |
| Administrateurs | competitions | `GET .../admin/settings/admins` | `load, adminsChanged from:body` | — | Lecture + action |
| Règles | competitions | inline (lecture seule) | — | — | Lecture |
| Params inscription | competitions | inline (formulaire) | — | — | Édition |
| Zone de danger | — | inline (actions) | — | — | Action |

### Événements

- `adminsChanged` — émis après ajout/retrait d'un admin

### Actions

- `POST .../admin/settings/general` → sauver nom/saison/logo
- `POST .../admin/settings/admins` → ajouter un admin → `HX-Trigger: adminsChanged`
- `DELETE .../admin/settings/admins/{coach_id}` → retirer un admin → `HX-Trigger: adminsChanged`
- `POST .../admin/settings/enrollment-params` → sauver mode d'accès/validation/places
- `POST .../admin/settings/reset-season` → réinitialiser la saison
- `DELETE .../admin/settings/delete-season` → supprimer la saison

### JS côté front

Aucun — formulaires HTMX standards.

### Widget réutilisable

- **Coach search widget** (BC spaces) — pour l'ajout d'admin (même widget que dans la création de compétition)
