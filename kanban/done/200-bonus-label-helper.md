# Competitions — Extraire le helper de label bonus (refacto iso-comportement)

**Priorité : haute**
**Dépend de :** —
**Contexte :** `src/app/competitions/io/web/rules_labels.rs` (nouveau)
**Spec :** `docs/specs/ranking-bonus-points/competition-rules-form/03-back.md`, `07-integration.md`

## Objectif

Factoriser le formatage du label bonus du récap, aujourd'hui **copié-collé** dans
`new_competition_phase_5.rs` (~135-155) et `admin/summary_tab.rs` (~192-209).
Refacto **pure, iso-comportement** — préalable au renommage `diff_td`→`min_td`
(carte 201) pour qu'il ne touche qu'un seul endroit.

## Conception

- Créer `src/app/competitions/io/web/rules_labels.rs` avec :
  ```rust
  pub fn format_bonus_label(rr: &RankingRules) -> Option<String>
  ```
- **Copier-coller** le bloc existant (règle 5 CLAUDE.md — pas de réécriture de
  mémoire) depuis l'un des deux sites, adapter uniquement la signature.
- Découper les sous-formatages (offensif / défensif) en fonctions courtes si besoin
  pour respecter la règle des 20 lignes.
- Remplacer les deux blocs inline par un appel à `format_bonus_label(rr)`.
- Aucun changement de comportement : le seuil défensif reste « ≤ 1 » en dur à ce
  stade (rendu dynamique en carte 201).

## Checklist

- [ ] `rules_labels.rs` créé, `format_bonus_label` exposé
- [ ] `new_competition_phase_5.rs` câblé sur le helper (bloc inline supprimé)
- [ ] `admin/summary_tab.rs` câblé sur le helper (bloc inline supprimé)
- [ ] Aucune régression d'affichage (mêmes chaînes qu'avant)
- [ ] Test unitaire du helper (offensif/défensif activés/désactivés → chaîne attendue)
- [ ] `make test` + `make check-arch` passent
