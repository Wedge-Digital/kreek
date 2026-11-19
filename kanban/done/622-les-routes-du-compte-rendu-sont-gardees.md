# Les routes du compte rendu sont gardées

**Épic :** E19 — Saisie du compte rendu : droits et robustesse · **Ordre :** 2 · **Dépend de :** 621
**Conception :** `docs/specs/saisie-mobile/E19-cadrage.md` (R1, R2, § 4)

## Objectif

Tout membre connecté d'un espace peut aujourd'hui saisir n'importe quel compte
rendu, pour n'importe quel camp. Après la carte, l'écriture et les écrans de
saisie sont réservés aux admins et aux coachs des deux équipes ; les autres
reçoivent 403.

## Conception

**Un garde commun** aux contrôleurs du BC : il charge la portée du rapport
(compétition, deux équipes), appelle `is_authorized`, rend `403` sinon.
Appelé en tête de chaque handler concerné — le patron de
`require_admin_access` côté `competitions`.

| Routes | Règle |
|---|---|
| `GET/POST /{id}` (sélection) | R1 — en plus de la règle actuelle du hors-calendrier |
| `GET/POST /step2`, `/inducements/{team}` | R1, R2 |
| `GET /step3`, `/step4`, widgets tour, joueurs temporaires, panneau d'action, journal | R2 |
| `POST /actions`, `DELETE /actions/{id}` | R1 |
| `GET/POST /step5` | R1, R2 |

**Inchangées** (décision du 2026-11-01) : `/new` (hors calendrier), les
feuilles imprimables, le récapitulatif — déjà gardé.

## Données de production

Aucune donnée modifiée. Un tiers qui saisit aujourd'hui pour d'autres sans être
admin est bloqué, ainsi que l'ancien coach d'une équipe reprise — accepté sans
mesure préalable (2026-11-01).

## Checklist

- [x] Le garde, et son appel sur chaque route du tableau
- [x] Tests unitaires du garde : admin, coach, tiers, rapport introuvable
- [x] E2E : un membre simple (`X-Bypass-Auth-Profile: simple`) refusé sur une
      écriture et sur un écran de saisie ; la suite existante reste verte
- [x] `tests/impact-map.toml` mis à jour
- [x] `make lint`, `make check-arch`, `make test`, e2e impactés
