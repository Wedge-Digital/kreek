# Step 5 — Architecture back

## BC responsable

`match_report` — aucune donnée inter-BC au-delà de `ITeamDataPort::find_team_info` (déjà existant).

---

## Fichiers à créer

| Fichier | Rôle |
|---|---|
| `io/web/step5_controller.rs` | Handlers `get_step5` + `post_step5` |
| `io/web/templates/step5.html` | Template Askama |
| `use_cases/record_post_match_use_case.rs` | Use case POST |
| `domain/match_report_ready_to_publish.rs` | Nouvel agrégat `MatchReportReadyToPublish` |

## Fichiers à modifier

| Fichier | Modification |
|---|---|
| `routes.rs` | Constante `MATCH_REPORT_STEP5` + méthode `step5()` |
| `router.rs` | Enregistrement `GET` + `POST` sur le path step5 |
| `io/web/mod.rs` | Déclaration du module `step5_controller` |
| `use_cases/mod.rs` | Déclaration du module `record_post_match_use_case` |
| `domain/mod.rs` | Déclaration du module `match_report_ready_to_publish` |
| `domain/events.rs` | Nouvel événement `PostMatchRecorded { ... }` |
| `domain/match_report_state.rs` | Nouvelle variante `ReadyToPublish(MatchReportReadyToPublish)` |
| `domain/match_report_pre_match.rs` | Nouvelles méthodes `compute_score()`, `compute_cas()`, `suggest_gains()`, `record_post_match()` |

---

## Ports

Aucun nouveau port. `ITeamDataPort::find_team_info` (existant) est appelé dans `get_step5` pour les logos et initiales du score banner.

---

## Domain services

Aucun. Score, sorties et suggestion de gain sont des méthodes de l'agrégat `MatchReportPreMatch` (logique domaine pure, sans dépendance externe).

---

## Transition d'état

La soumission du step 5 transite `MatchReportPreMatch` → `MatchReportReadyToPublish`.

```
Draft ──SelectionConfirmed──► PreMatch ──PostMatchRecorded──► ReadyToPublish
                                                                     │
                                                              (récap + publication)
```

L'événement `PostMatchRecorded` est réhydraté dans `match_report_state.rs` pour produire
un `MatchReportReadyToPublish` à partir du `MatchReportPreMatch` courant.

---

## Responsabilités des handlers

### `get_step5`

1. Charger l'agrégat via le repository
2. Extraire `MatchReportPreMatch` (rediriger si état incompatible)
3. Appeler `ITeamDataPort::find_team_info` pour les deux équipes (logos/initiales)
4. Appeler les méthodes agrégat : `compute_score()`, `compute_cas()`, `suggest_gains()`
5. Rendre le template — pré-remplir si l'état est déjà `ReadyToPublish` (revisitation)

### `post_step5`

1. Parser le form (gains home/away, fan mods home/away, titre optionnel, corps optionnel)
2. Construire les value objects
3. Appeler `record_post_match_use_case`
4. Rediriger vers la page récap

---

## Règles métier confirmées à cette phase

- `suggest_gains()` est une méthode d'agrégat : `(fans_home + fans_away) / 2 × 10 000 + nb_tds × 10 000`
- `compute_score()` est une méthode d'agrégat : compte les `MatchActionType::Touchdown` dans `home_actions` / `away_actions`
- `compute_cas()` est une méthode d'agrégat : compte les `MatchActionType::Sortie` (ou équivalent blessure) par side
- La soumission fait passer le rapport en `ReadyToPublish` via `PostMatchRecorded`
- La page est revisitable : `get_step5` accepte un rapport en état `ReadyToPublish` et pré-remplit le formulaire
