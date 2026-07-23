# Phase 8 — Cartes kanban (competition-rules-form)

Découpage de l'implémentation de l'unité `competition-rules-form`. Cartes
ordonnées par dépendance ; chaque carte compile, est testable et commitable.
(L'unité `post-match-bonus-calc` — propagation ACL + calcul — aura ses propres
cartes après passage de son propre workflow.)

## Cartes

| # | Carte | Portée | Dépend de |
|---|---|---|---|
| 200 | `200-bonus-label-helper.md` | Extraire `format_bonus_label` (helper présentation) depuis phase-5 & summary_tab — refacto iso-comportement | — |
| 201 | `201-bonus-domaine.md` | VOs (`MinTd` renommé, `MaxTdConceded`, `MinCasualties`), struct `AggressiveBonus`, serde defaults ; enrichir le helper (seuil défensif dynamique + agressif) ; tests unitaires | 200 |
| 202 | `202-bonus-formulaire.md` | Markup section bonus + `buildJSON()` + `initFromExistingRules()` (inline, pas de widget) | 201 |
| 203 | `203-bonus-formulaire-e2e.md` | Scénario Playwright : saisie, récap, ré-hydratation, rétro-compat | 201, 202 |

## Justification de l'ordre

- **200 avant 201** : créer le helper d'abord (refacto pure) fait que le renommage
  `diff_td`→`min_td` de la carte 201 ne touche qu'un seul endroit (le helper) au lieu
  des deux sites récap.
- **201 avant 202** : le formulaire (JS `buildJSON`/`initFromExistingRules`) s'appuie
  sur les clés serde et la structure des règles fixées côté domaine.
- **203 en dernier** : l'E2E valide le bout-en-bout front + persistance.

## Découpage respecté (règles workflow)

- Carte domaine (VOs + méthodes) : 201.
- Carte par mutation/handler : ici aucun handler nouveau ⇒ la carte 202 porte le
  template + JS (le POST existant est réutilisé).
- Carte tests E2E : 203.
- Chaque carte réalisable en une session.
