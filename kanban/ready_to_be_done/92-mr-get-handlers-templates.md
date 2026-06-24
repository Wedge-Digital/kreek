# BC match_report — GET handlers + templates step1

**Priorité : haute**
**Dépend de :** 91
**Contexte :** match_report step1, couche IO/web

## Objectif

Implémenter les handlers GET pour la page step1 (formulaire vierge + rapport pré-rempli) et les fragments HTMX de cascade, avec les templates Askama.

## Conception

Cf. `docs/specs/match-report/step1-selection/02-front.md` et `04-dtos.md`

### Fichiers

```
src/app/match_report/io/web/
├── mod.rs
├── match_selection_controller.rs          ← handlers GET
├── view_models.rs                         ← VMs
└── templates/
    ├── match-selection.html               ← page complète
    └── fragments/
        ├── season-options.html
        ├── round-options.html
        └── team-options.html
```

### Handlers GET

| Handler | Route | Description |
|---------|-------|-------------|
| `new_match_report` | GET `/match-report/new` | Formulaire vierge — charge compétitions via port, rend la page |
| `edit_match_report` | GET `/match-report/{id}` | Charge l'agrégat, redirige vers la bonne étape si phase != Draft, sinon rend le formulaire pré-rempli |
| `seasons_fragment` | GET `/match-report/new/seasons?competition_id=X` | Fragment options saisons (anti-chronologique) |
| `rounds_fragment` | GET `/match-report/new/rounds?season_id=X` | Fragment options journées |
| `teams_fragment` | GET `/match-report/new/teams?season_id=X` | Fragment options équipes (filtrées ReadyToPlay) |

### Contrôle d'accès dans les handlers

- Déterminer le rôle (admin espace / admin compétition / coach lambda)
- Filtrer les compétitions et équipes selon le rôle
- Passer `UserRoleVm` au template

### Template

Basé sur la maquette `app-match-report-step1.html`. TomSelect searchable sur tous les selects. Pour les équipes : searchable sur nom + coach.

### Cascade HTMX

Chaque select déclenche un `hx-get` avec `hx-trigger="change"` qui recharge le select suivant.

## Checklist

- [ ] `view_models.rs` : VMs (CompetitionOptionVm, SeasonOptionVm, RoundOptionVm, TeamOptionVm, SelectedMatchVm, UserRoleVm)
- [ ] `match-selection.html` : template page complète avec stepper, selects TomSelect, cartes preview, bannière pré-remplissage, erreur
- [ ] `season-options.html`, `round-options.html`, `team-options.html` : fragments
- [ ] Handler `new_match_report` : charge compétitions, rend la page vierge
- [ ] Handler `edit_match_report` : charge agrégat, redirige si phase != Draft, sinon pré-remplit
- [ ] Handlers fragments : `seasons_fragment`, `rounds_fragment`, `teams_fragment`
- [ ] Brancher les routes dans `router.rs`
- [ ] CSS dans `match-report-shared.css` (ou fichier dédié step1)
- [ ] `cargo check` passe
- [ ] Test manuel : formulaire vierge charge, cascade fonctionne
