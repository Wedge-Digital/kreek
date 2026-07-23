# Phase 2 — Architecture front (competition-rules-form)

## Constat

Le formulaire admin phase-2 (`new-competition-phase-2.html`) est une **page
monolithique historique** : inputs statiques + JS inline (`buildJSON()`,
`initFromExistingRules()`, gestion des tiers/pickers) + un **unique POST `fetch`**
de tout le JSON de règles. Ce **n'est pas** une page à widgets.

**Décision validée : on conserve le pattern inline legacy — pas d'introduction du
pattern widget sur cette page.** La section bonus reste des `<input>` inline,
cohérente avec les autres sections (VND, tiers).

## Composition

| Section | Type | Endpoint | Communication | Mode |
|---|---|---|---|---|
| Bonus de classement | Inputs inline (pas un widget) | Aucun propre — soumis avec le POST global | Aucune (pas d'événements DOM) | Édition, soumission groupée |

Pas de widget ⇒ pas de tableau d'événements DOM (N/A).

## Front (JS inline) vs Back (HTTP)

### Front — géré dans le `<script>` inline existant
- **`buildJSON()`** lit les inputs bonus, existants et nouveaux :
  - Offensif (existant) : `off_activated`, `off_points`, `off_diff_td`.
  - Défensif : `def_activated`, `def_points`, **`def_max_td` (nouveau input de seuil)**.
  - Agressif (nouveau) : `agg_activated`, `agg_points`, `agg_min_cas`.
- **`initFromExistingRules(rules)`** ré-hydrate ces inputs depuis
  `rules.ranking_rules.defensive_bonus.max_td_conceded` et
  `rules.ranking_rules.aggressive_bonus.*` (avec valeurs par défaut si absents —
  rétro-compat des règles enregistrées avant cette feature).

### Back — HTTP
- **Aucun nouvel endpoint.** Le POST unique existant (path courant, `submitRules()`)
  reçoit le JSON `ranking_rules` enrichi des blocs :
  - `defensive_bonus.max_td_conceded` (nouveau champ)
  - `aggressive_bonus: { activated, points, min_casualties }` (nouveau bloc)
- La désérialisation serveur et la persistance sont couvertes par les phases 3-7 et
  par l'unité `post-match-bonus-calc` (structs domaine côté competitions).

## Interaction

Édition simple (saisie clavier), pas d'auto-save. Soumission groupée au clic
« Enregistrer & continuer → ». Validation de saisie minimale : `min="0"` sur les
points/seuils, `min="1"` sur le seuil offensif.

## Widgets existants réutilisables

Aucun (la section bonus n'est pas un widget). Pickers de tiers (rosters/inducements/
star players) hors périmètre de cette feature.

## Règles métier à cette étape

Aucune nouvelle. Contraintes de saisie uniquement (bornes `min`). La logique de
calcul (activation, seuils, cumul) vit dans l'unité `post-match-bonus-calc`.