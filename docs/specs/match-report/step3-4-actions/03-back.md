# Step 3 & 4 — Actions match — Architecture back

## Mapping widgets → BCs

| Widget | BC propriétaire | Handler existant ? |
|---|---|---|
| Page hôte (step3 / step4) | MatchReport | Non |
| turn-selector | MatchReport | Non |
| temp-player-selector | MatchReport | Non |
| action-panel | MatchReport | Non |
| action-log | MatchReport | Non |
| player-selector | Players | Non |

---

## Nouveaux fichiers — BC MatchReport

### Handlers

```
src/app/match_report/io/web/
├── actions_step_controller.rs      ← GET /step3, GET /step4  (page hôte)
├── record_action_controller.rs     ← POST /step3/actions, /step4/actions
│                                      DELETE /actions/{action_id}
└── widgets/                        ← nouveau dossier
    ├── mod.rs
    ├── turn_selector_widget.rs     ← GET /step3/turn-selector, /step4/turn-selector
    ├── temp_player_selector_widget.rs ← GET /step3/temp-players, /step4/temp-players
    ├── action_panel_widget.rs      ← GET /step3/action-panel, /step4/action-panel
    └── action_log_widget.rs        ← GET /step3/log, /step4/log
```

### Templates

```
src/app/match_report/io/web/templates/
├── match-report-actions.html       ← page hôte (partagée step3/step4)
└── widgets/                        ← nouveau dossier
    ├── turn-selector.html
    ├── temp-player-selector.html
    ├── action-panel.html
    └── action-log.html
```

---

## Nouveaux fichiers — BC Players

### Handler

```
src/app/players/io/web/
└── widgets/                        ← nouveau dossier
    ├── mod.rs
    └── match_player_selector_widget.rs   ← GET /players/teams/{team_id}/match-selector
```

### Template

```
src/app/players/io/web/templates/   ← à créer (même pattern que autres BCs)
└── widgets/
    └── match-player-selector.html
```

---

## Fichiers existants modifiés

| Fichier | Modification |
|---|---|
| `match_report/routes.rs` | Ajout des constantes `MATCH_REPORT_STEP4` et toutes les sous-routes widgets |
| `match_report/router.rs` | Câblage des nouveaux handlers |
| `match_report/io/web/mod.rs` | Exposition des nouveaux modules |
| `match_report/ports.rs` | Ajout du trait `IPlayerDataPort` |
| `match_report/context.rs` | Ajout du champ `player_data: Arc<dyn IPlayerDataPort>` |
| `players/routes.rs` | Ajout de `MATCH_PLAYER_SELECTOR` |
| `players/router.rs` | Câblage du nouveau handler widget |
| `players/io/web/mod.rs` | Exposition du nouveau module |
| `infrastructure/match_report/mod.rs` | Ajout du `player_data_adapter` |
| `main.rs` | Instanciation et injection de `player_data_adapter` |

---

## Routes

### BC MatchReport

```
GET    /app/{space_id}/match-report/{mr_id}/step3
GET    /app/{space_id}/match-report/{mr_id}/step4

GET    /app/{space_id}/match-report/{mr_id}/step3/turn-selector
GET    /app/{space_id}/match-report/{mr_id}/step4/turn-selector

GET    /app/{space_id}/match-report/{mr_id}/step3/temp-players
GET    /app/{space_id}/match-report/{mr_id}/step4/temp-players

GET    /app/{space_id}/match-report/{mr_id}/step3/action-panel
GET    /app/{space_id}/match-report/{mr_id}/step4/action-panel

GET    /app/{space_id}/match-report/{mr_id}/step3/log
GET    /app/{space_id}/match-report/{mr_id}/step4/log

POST   /app/{space_id}/match-report/{mr_id}/step3/actions
POST   /app/{space_id}/match-report/{mr_id}/step4/actions

DELETE /app/{space_id}/match-report/{mr_id}/actions/{action_id}
```

### BC Players

```
GET    /app/{space_id}/players/teams/{team_id}/match-selector
```

La page hôte BC MatchReport accède à cette route via `AppRoutes::default().players.match_selector(...)`.

---

## Port inter-BC nécessaire : `IPlayerDataPort`

Défini dans `src/app/match_report/ports.rs`.
Adapter dans `src/infrastructure/match_report/player_data_adapter.rs`.

```rust
#[derive(Debug)]
pub struct MatchPlayerCountDto {
    pub available: u8,   // joueurs en état de jouer
    pub total: u8,       // joueurs dans le roster
}

#[async_trait]
pub trait IPlayerDataPort: Send + Sync {
    /// Nombre de joueurs disponibles pour ce match (hors blessés/suspendus).
    /// Utilisé pour calculer le nombre de journaliers : max(0, 11 - available).
    async fn count_available_players(
        &self,
        team_id: &str,
    ) -> Result<MatchPlayerCountDto, String>;
}
```

Ce port est utilisé **uniquement dans la use case de création des TempPlayers** (fin d'étape 2), pas dans les widgets de step3/step4 (le widget player-selector appartient à BC Players et accède à ses propres données directement).

---

## Impact sur l'étape 2 (création des TempPlayers)

À la fin de l'enregistrement des inducements d'une équipe, le système doit créer les instances de joueurs temporaires pour cette équipe. Ce processus est déclenché dans un nouveau use case `init_temp_players_use_case.rs`.

### Déclenchement

Appelé depuis le handler `post_inducements` après l'appel au use case `record_inducements` existant, une fois les inducements de l'équipe persistés.

### Logique

```
Pour chaque équipe dont les inducements viennent d'être enregistrés :

1. Récupérer les StarPlayerEngaged depuis les events de la session
   → créer un TempPlayer par star player (kind: StarPlayer, ref_uid: star_player_uid)

2. Récupérer les inducements de type Mercenary depuis la liste achetée
   → créer un TempPlayer par mercenaire (kind: Mercenary, position: from ref data)

3. Appeler IPlayerDataPort.count_available_players(team_id)
   → journaliers = max(0, 11 - available)
   → créer N TempPlayer (kind: Journalier, position: lineman de base du roster)

4. Persister les TempPlayers dans le match report (nouvel event TempPlayersInitialized)
```

### Retour en arrière

Si les inducements sont re-soumis pour une équipe (navigation retour → re-POST), le use case supprime les TempPlayers existants de cette équipe avant d'en créer de nouveaux.

---

## Domain service nécessaire

Aucun domain service inter-BCs pour les widgets step3/step4 (chaque widget accède à ses propres données). Le seul traitement inter-BC est dans `init_temp_players_use_case.rs` via `IPlayerDataPort`.

---

## Règles métier à préciser à cette étape ?

Points ouverts identifiés :

1. **Disponibilité joueur** : la notion de joueur "disponible vs indisponible" (chip grisée dans player-selector) est-elle déjà trackée dans BC Players, ou est-ce à implémenter dans cette feature ?

2. **Journalier — position** : ✅ résolu — `PlayerPosition.is_journalier: bool` ajouté dans le référentiel (JSON + `models.rs`). Un seul `is_journalier = true` par roster, validé pour les 30 équipes. La résolution passe par `ITeamDataPort` (roster_id) + lookup dans BC References.

3. **Mercenaires dans le référentiel** : les mercenaires sont-ils déjà dans le catalogue d'inducements de BC References, ou est-ce à ajouter ? Un inducement "mercenaire" est-il distingué des autres inducements (flag `is_mercenary` analogue à `is_star_player`) ?
