# MR-STEP2-01 — Value object D3Roll

## Objectif

Créer le value object `D3Roll` (jet de D3, valeurs {1, 2, 3}) et l'erreur domaine associée.

## Fichiers

- `src/app/match_report/domain/value_objects.rs` — ajouter `D3Roll`
- `src/app/match_report/domain/error.rs` — ajouter `InvalidD3Roll(u8)`

## Conception

Voir `docs/specs/match-report/step2-pre-match/06-domaine.md`

## Checklist

- [ ] Smart constructor `D3Roll::try_new(u8)`
- [ ] Méthode `value() -> u8`
- [ ] Derive `Serialize, Deserialize` (persisté dans les events)
- [ ] `DomainError::InvalidD3Roll(u8)`
- [ ] Test `d3roll_accepte_1_2_3`
- [ ] Test `d3roll_rejette_0_et_4`
