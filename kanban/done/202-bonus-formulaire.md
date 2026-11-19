# Competitions — Saisie des bonus dans le formulaire admin (phase-2)

**Priorité : haute**
**Dépend de :** `201-bonus-domaine.md`
**Contexte :** `src/app/competitions/io/web/templates/new-competition-phase-2.html`
**Spec :** `docs/specs/ranking-bonus-points/competition-rules-form/01-mockup.md`, `02-front.md`

## Objectif

Ajouter au formulaire des règles (étape 2) la saisie du **seuil défensif
configurable** et du **bonus agressif**, en restant sur le pattern inline legacy
(inputs + JS `buildJSON`/`initFromExistingRules`, pas de widget).

## Conception

### Markup (section « Bonus de classement », cf. `01-mockup.md`)
- Bonus offensif : wording « TDs marqués » (inchangé fonctionnellement).
- Bonus défensif : remplacer le « ≤ 1 TD encaissé » statique par un input
  `def_max_td` (défaut 1).
- Bonus agressif (nouveau) : ligne calquée sur l'offensif — `agg_activated`
  (décoché par défaut), `agg_points`, comparateur `>`, `agg_min_cas` (défaut 2),
  « sorties infligées ».
- Réutilisation stricte des classes CSS existantes — **aucune CSS ajoutée**.

### JS inline
- `buildJSON()` : écrire `defensive_bonus.max_td_conceded` et le bloc
  `aggressive_bonus: { activated, points, min_casualties }`.
- `initFromExistingRules()` : ré-hydrater `def_max_td`, `agg_activated`,
  `agg_points`, `agg_min_cas` depuis les règles, avec défauts si champs absents
  (rétro-compat des règles enregistrées avant la feature).
- Respecter la règle des 20 lignes (extraire des sous-fonctions si `buildJSON` /
  `initFromExistingRules` dépassent).

## Checklist

- [ ] Markup des 3 lignes bonus (défensif avec input seuil, agressif nouveau)
- [ ] `buildJSON()` sérialise `max_td_conceded` + `aggressive_bonus`
- [ ] `initFromExistingRules()` ré-hydrate + applique les défauts si absents
- [ ] Aucune CSS ajoutée (classes existantes réutilisées)
- [ ] Vérif manuelle : saisie → enregistrement → ré-édition restaure les valeurs
