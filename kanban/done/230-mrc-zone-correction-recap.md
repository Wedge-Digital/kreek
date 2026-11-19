# `match_report` — Zone de correction sur la page recap

**Priorité : haute**
**Dépend de :** `228-mrc-garde-fou-ports.md`
**Fichiers :** `src/app/match_report/io/web/{builders,recap_controller}.rs`, `src/app/match_report/io/web/templates/recap.html`, `assets/static/css/pages/match-report-recap.css`
**Spec :** `docs/specs/match-report-correction/recap/02-front.md`, `04-dtos.md`
**Maquette :** `assets/rawpages/html/app-match-report-recap-correction.html`

## Objectif

Afficher la zone de correction et ses états. Indépendante de la carte 229 : elle
peut être menée avant, le bouton pointant alors vers une route encore absente.

## Conception

### View model — dans `builders.rs`, pas `view_models.rs`

```rust
pub struct CorrectionZoneVm {
    pub can_correct:    bool,
    pub blocked_reason: Option<String>,
    pub unpublish_url:  String,
}

pub fn build_correction_zone(
    eligibility:   &CorrectionEligibility,
    home_info:     &TeamInfoDto,
    away_info:     &TeamInfoDto,
    unpublish_url: String,
) -> CorrectionZoneVm
```

Placé dans `builders.rs` parce qu'il dépend de `TeamInfoDto`, un DTO de port —
même règle que `PerformanceRowVm`, `RoundContextVm` et `TeamBannerVm`.

**C'est ici que le nom d'équipe entre en jeu** : le `CorrectionBlocker` domaine
ne porte qu'un `TeamSide`, le builder le résout en nom via les deux
`TeamInfoDto`. `blocked_reason` est une **phrase complète** — le template
n'assemble rien.

Libellés :

| Blocker | Message |
|---|---|
| `SppAlreadySpent { side }` | « **{équipe}** a déjà utilisé les SPP de ses joueurs. Le rapport n'est plus corrigeable. » |
| `PhaseAdvanced { side }` | « **{équipe}** a validé sa phase d'amélioration. Le rapport n'est plus corrigeable. » |
| `EligibilityUnknown` | « Impossible de vérifier si ce rapport est corrigeable pour le moment. » — sans nom d'équipe |

### Contrôleur

`get_recap` alimente deux champs supplémentaires de `RecapTemplate` :

```rust
pub correction:       Option<CorrectionZoneVm>,  // Some(_) si le rapport est publié
pub under_correction: bool,                      // !is_published && was_published_before
```

Les 4 appels de port du garde-fou rejoignent le `tokio::join!` existant.

### Template et CSS

`recap.html` — bandeau conditionnel sur `under_correction` avant `ms-cta-row` ;
zone de correction après, dans la branche `is_published`.

`match-report-recap.css` — classes `.ms-correct-*` et `.ms-unpublished-banner`,
reprises **telles quelles** de la maquette.

Le bouton utilise `hx-post` + `hx-confirm`, pas un `<form method="post">` :
`hx-confirm` ne se déclenche que sur une requête pilotée par HTMX, et le futur
middleware CSRF rejettera les POST sans `HX-Request`.

Aucun style inline — interdits par CLAUDE.md.

## Checklist

- [ ] `CorrectionZoneVm` et `build_correction_zone` dans `builders.rs`
- [ ] Les 3 libellés, dont `EligibilityUnknown` sans nom d'équipe
- [ ] `get_recap` alimente `correction` et `under_correction`
- [ ] Appels de port intégrés au `tokio::join!` existant
- [ ] Zone de correction dans `recap.html`, branche `is_published`
- [ ] Bandeau conditionnel sur `under_correction`
- [ ] `hx-post` + `hx-confirm` sur le bouton
- [ ] CSS repris de la maquette, aucun style inline
- [ ] Test : `build_correction_zone` nomme la bonne équipe selon le `TeamSide`
- [ ] Test : `EligibilityUnknown` ne nomme aucune équipe
- [ ] Test : `can_correct` à `true` si `Eligible`
- [ ] Rendu vérifié dans le navigateur sur les 5 états de la maquette
- [ ] `make test` passe
- [ ] `make check-arch` passe
