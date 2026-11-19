# MR-STEP2-07 — Page step 2 : handler POST + wiring

## Objectif

Gérer la soumission du formulaire fan factor : validation, appel use case, redirect.

## Dépendances

- MR-STEP2-04 (use case), MR-STEP2-06 (page GET)

## Fichiers

- `src/app/match_report/io/web/pre_match_controller.rs` — handler POST
- `src/app/match_report/router.rs` — ajouter POST sur la route

## Checklist

- [ ] Struct `RecordFanFactorForm` (Deserialize)
- [ ] Handler `post_pre_match` : validation D3Roll, construction commande, appel use case
- [ ] Redirect vers page suivante en cas de succès
- [ ] 400 si D3 invalide, 404 si match report introuvable
