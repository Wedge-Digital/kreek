# MR-STEP5-04 — Handler + template step5

## Objectif

Implémenter la page step 5 : handler GET (rendu), handler POST (soumission), template Askama,
route et enregistrement dans le router.

## Dépendances

141 — `record_post_match_use_case` doit exister.

## Conception

Voir `docs/specs/match-report/step5-apres-match/02-front.md`, `03-back.md`, `04-dtos.md`, `07-integration.md`.

## Fichiers impactés

- `src/app/match_report/io/web/step5_controller.rs` (nouveau)
- `src/app/match_report/io/web/templates/step5.html` (nouveau)
- `src/app/match_report/routes.rs`
- `src/app/match_report/router.rs`
- `src/app/match_report/io/web/mod.rs`

## Checklist

### Routes
- [ ] Constante `MATCH_REPORT_STEP5` dans `routes.rs`
- [ ] Méthode `step5(&self, space_id, match_report_id) -> String` dans `impl Routes`

### Handler GET `get_step5`
- [ ] Struct `Step5Template` (champs définis dans `04-dtos.md`)
- [ ] `impl IntoResponse for Step5Template`
- [ ] Logique de redirection selon l'état (`Draft` → edit, `Cancelled` → 410)
- [ ] Appel `find_team_info` × 2 pour logos + initiales
- [ ] Pré-remplissage : `suggest_gains()` si `PreMatch`, valeurs existantes si `ReadyToPublish`
- [ ] `home_fan_mod` / `away_fan_mod` : `0` par défaut si pas encore saisi

### Handler POST `post_step5`
- [ ] `RecordPostMatchForm` — struct `Deserialize`
- [ ] Construction des value objects via smart constructors (`MatchGain::new`, `FanFactorMod::new`)
- [ ] Appel `record_post_match_use_case::execute`
- [ ] Redirect vers step5 (même page) après succès — à mettre à jour vers récap quand disponible
- [ ] Erreurs HTTP correctes (`404`, `409`, `500`)

### Template `step5.html`
- [ ] Extends `app-layout.html`
- [ ] `mr-header` + `mr-steps` (étapes 1✓ 2✓ 3✓ 4✓ 5●)
- [ ] Score banner avec logos/initiales, score, sorties
- [ ] Section gains — une ligne par équipe (logo + nom + suggestion + input)
- [ ] Section fan factor — boutons −2/−1/0/+1/+2 par équipe, sélection Alpine
- [ ] Section résumé — input titre + textarea corps (optionnels)
- [ ] `already_recorded` — bandeau info si re-soumission
- [ ] Navigation : retour step4 + bouton submit

### Router
- [ ] Route `MATCH_REPORT_STEP5` enregistrée (`get(get_step5).post(post_step5)`)
- [ ] Module `step5_controller` déclaré dans `io/web/mod.rs`

### Build & vérification
- [ ] Compiler sans erreur (`cargo build`)
- [ ] Rendu visuel vérifié en dev (correspondance maquette `app-match-report-step5.html`)
