# Step 5 — Architecture front

## Vue d'ensemble

Page formulaire simple : un GET pour le rendu, un POST pour la soumission.
Toutes les données sont issues du même agrégat `MatchReport` — pas de lazy loading,
pas de communication inter-sections. Aucun widget HTMX séparé n'est justifié.

La page peut être revisitée : si step 5 a déjà été soumis, le formulaire est
pré-rempli avec les valeurs existantes. La suggestion de gain est la valeur par
défaut si aucune saisie n'existe encore.

---

## Endpoints

| Méthode | URL | Handler |
|---|---|---|
| GET | `/spaces/{space_id}/match-report/{mr_id}/step5` | `get_step5` |
| POST | `/spaces/{space_id}/match-report/{mr_id}/step5` | `post_step5` |

---

## Structure de la page

| Section | Source données | Interaction |
|---|---|---|
| `mr-header` + `mr-steps` | Template | Aucune |
| Score banner (TDs + sorties) | Agrégat → handler (computed) | Lecture seule |
| Gains par équipe | Agrégat + suggestion calculée | Inputs numériques |
| Fan factor modifier | Agrégat (valeurs actuelles) | Boutons sélection Alpine |
| Résumé (titre + corps) | Agrégat (si déjà saisi) | Input texte + textarea |
| Navigation | Template | Lien retour + submit |

---

## Interactivité Alpine.js

Un seul `x-data` au niveau du formulaire, pour :

- Sélection exclusive des boutons fan factor par équipe (-2 / -1 / 0 / +1 / +2)
- Mise en évidence visuelle du bouton actif (classes `active`, `positive`, `negative`)

Pas d'Alpine pour les gains ni le résumé — inputs natifs suffisants.

---

## Pas de widgets HTMX

Pas de widget car :

- Toutes les données viennent du même agrégat
- Pas de section avec un cycle de vie indépendant
- Pas de rafraîchissement partiel nécessaire
- Soumission en une seule fois (formulaire complet)

---

## Règles métier confirmées à cette phase

- Le score affiché est calculé par l'agrégat depuis le journal d'actions (méthode `compute_score()`) — pas de saisie manuelle
- Les sorties affichées sont calculées depuis le journal d'actions (`compute_cas()`)
- La suggestion de gain est calculée par le handler : `(fans_home + fans_away) / 2 × 10 000 + nb_tds × 10 000`
- Le fan factor modifier est libre entre -2 et +2 (pas de validation métier côté front)
- Le résumé (titre + corps) est entièrement optionnel
- La re-soumission est autorisée et écrase les données précédentes
