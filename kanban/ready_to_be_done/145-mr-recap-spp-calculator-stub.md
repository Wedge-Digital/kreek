# MR-RECAP-02 — Mini BC spp_calculator (stub)

## Objectif

Créer le mini BC `spp_calculator` avec une implémentation **stub** du calcul SPP : renvoie une
valeur plausible (10 SPP) par acteur distinct ayant agi, en excluant les acteurs n'ayant que des
actions subies (`Blesse{injury}`, BR5). Le vrai calcul (quelles actions donnent combien de SPP,
sélection de ruleset Normal/Brutal) est **hors scope** — carte dédiée future.

## Dépendances

Aucune — module autonome, ne dépend d'aucune autre carte de cette page.

## Conception

Voir `docs/specs/match-report/recap/06-domaine.md` (section « spp_calculator — domaine stubbé »)
et `docs/specs/match-report/recap/07-integration.md`.

## Fichiers impactés

- `src/app/spp_calculator/mod.rs` (nouveau)
- `src/app/spp_calculator/domain/calculator.rs` (nouveau)
- `src/app/spp_calculator/domain/mod.rs` (nouveau)

## Checklist

- [ ] `SppActionInput { actor_key: String, is_injury: bool }` (ou équivalent minimal) — type d'entrée opaque, ne dépend d'aucun type du domaine `match_report`
- [ ] `SppCalculationResult { home: Vec<(String, u8)>, away: Vec<(String, u8)> }` (ou équivalent)
- [ ] `calculate(home_actions, away_actions) -> SppCalculationResult` — stub à `10` SPP par acteur distinct, excluant les acteurs n'ayant que des actions `is_injury: true`
- [ ] Test `calculate_stub_never_credits_spp_to_an_injury_only_actor`
- [ ] Test `calculate_stub_credits_flat_spp_to_other_actors`
- [ ] **Ne pas créer** `IRosterSppPort`, `roster_spp_adapter.rs`, `spp_rules.rs`, `spp_rules.json` dans cette carte (décision de descope — aucun appelant tant que le calcul réel n'existe pas)
- [ ] Compiler sans erreur (`cargo build`)
- [ ] Tests verts (`make test`)
