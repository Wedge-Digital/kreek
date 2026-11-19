# BC match_report — Use cases + POST handlers step1

**Priorité : haute**
**Dépend de :** 91
**Contexte :** match_report step1, couche use cases + IO/web

## Objectif

Implémenter les deux use cases (Create + Update/Confirm) et les handlers POST correspondants.

## Conception

Cf. `docs/specs/match-report/step1-selection/05-use-cases.md`

### Fichiers

```
src/app/match_report/
├── use_cases/
│   ├── mod.rs
│   ├── create_match_report_use_case.rs
│   └── update_match_selection_use_case.rs
├── io/web/
│   └── match_selection_controller.rs      ← ajout handlers POST
```

### CreateMatchReportUseCase

1. Valide `home != away`
2. Appelle `MatchReportDraft::create()` → produit `MatchReportCreated`
3. Appende l'event dans l'event store (version 0)
4. Retourne le `match_report_id`

### UpdateMatchSelectionUseCase

1. Charge l'agrégat → doit être `MatchReportState::Draft`
2. Si sélection changée → `draft.update_selection()` → appende `SelectionUpdated`
3. Vérifie que les deux équipes sont en `ReadyToPlay` via `ITeamDataPort::is_team_ready_to_play()` → sinon `TeamNotAvailable`
4. `draft.confirm_selection()` → appende `SelectionConfirmed`
5. Retourne le `match_report_id`

### Handlers POST

| Handler | Route | Description |
|---------|-------|-------------|
| `create_match_report` | POST `/match-report/new` | Parse formulaire, construit commande avec VOs, appelle CreateUseCase, redirect vers `/match-report/{id}` |
| `update_match_selection` | POST `/match-report/{id}` | Parse formulaire, appelle UpdateUseCase, redirect vers step2 |

En cas d'erreur : re-rend la page avec le message d'erreur et les selects dans leur état précédent.

## Checklist

- [ ] `create_match_report_use_case.rs` : orchestration Create
- [ ] `update_match_selection_use_case.rs` : orchestration Update + Confirm avec vérification ReadyToPlay
- [ ] Handler POST `create_match_report`
- [ ] Handler POST `update_match_selection`
- [ ] Brancher les routes POST dans `router.rs`
- [ ] `cargo check` passe
- [ ] Test manuel : créer un rapport vierge, confirmer un rapport pré-rempli
