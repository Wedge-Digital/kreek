# Step 2 — Mercenaires — Intégration

## Migration SQL

**Aucune migration nécessaire.**

Les mercenaires sont persistés comme des `InducementPurchase` (UID `"MERCO:{pos}:{level}"`) dans les colonnes `home_inducements` / `away_inducements` déjà existantes (JSONB). La projection repository, la rehydratation de l'agrégat et l'event handler `InducementsRecorded` n'ont pas à changer.

---

## Ports — src/app/match_report/ports.rs

### Nouveaux DTOs (ajouter après `JournalierPositionDto`)

```rust
pub struct RosterPositionDto {
    pub position_uid:  String,
    pub position_name: String,
    pub base_cost:     u32,
    pub max_qty:       u8,
    pub is_journalier: bool,
}

pub struct PositionCountDto {
    pub position_uid: String,
    pub count:        u8,
}
```

### Nouvelle méthode sur ITeamDataPort

```rust
async fn find_roster_positions(&self, team_id: &str) -> Vec<RosterPositionDto>;
```

### Nouvelle méthode sur IPlayerDataPort

```rust
async fn find_player_counts_by_position(&self, team_id: &str) -> Vec<PositionCountDto>;
```

---

## Infrastructure — ref_team_data_adapter.rs

Nouveau bloc `impl ITeamDataPort` :

```rust
async fn find_roster_positions(&self, team_id: &str) -> Vec<RosterPositionDto> {
    let Ok(Some(team)) = self.team_repo.find_by_id(team_id).await else { return vec![]; };
    let roster_id = team.roster_id.to_string();
    let Some(ref_team) = self.reference_repo.find_team_by_uid(&roster_id) else { return vec![]; };
    ref_team
        .available_players
        .iter()
        .map(|p| RosterPositionDto {
            position_uid:  p.uid.clone(),
            position_name: p.position_name.clone(),
            base_cost:     p.cost,
            max_qty:       p.max_quantity,
            is_journalier: p.is_journalier,
        })
        .collect()
}
```

Les deux repos sont déjà injectés dans `RefTeamDataAdapter` — pas de changement de constructeur.

---

## Infrastructure — player_data_adapter.rs

Nouveau bloc `impl IPlayerDataPort` :

```rust
async fn find_player_counts_by_position(&self, team_id: &str) -> Vec<PositionCountDto> {
    use crate::app::players::domain::player::TeamId;
    let tid = TeamId(team_id.to_string());
    let players = self
        .player_projection_repo
        .find_by_team_id(&tid)
        .await
        .unwrap_or_default();
    let mut counts: std::collections::HashMap<String, u8> = std::collections::HashMap::new();
    for p in &players {
        *counts.entry(p.roster_line_id.clone()).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .map(|(position_uid, count)| PositionCountDto { position_uid, count })
        .collect()
}
```

`roster_line_id` de `PlayerProjection` est le `position_uid` de la position de référence. Déjà disponible dans la projection.

---

## Routes — src/app/match_report/routes.rs

### Nouvelle constante dans `pub mod path`

```rust
pub const MATCH_REPORT_MERCENARY_SELECTOR: &str =
    "/app/{space_id}/match-report/{match_report_id}/step2/{team_id}/mercenaires";
```

### Nouvelle méthode dans `impl Routes`

```rust
pub fn mercenary_selector(
    &self,
    space_id:        &str,
    match_report_id: &str,
    team_id:         &str,
) -> String {
    path::MATCH_REPORT_MERCENARY_SELECTOR
        .replace("{space_id}", space_id)
        .replace("{match_report_id}", match_report_id)
        .replace("{team_id}", team_id)
}
```

---

## Router — src/app/match_report/router.rs

### Import

```rust
use crate::app::match_report::io::web::widgets::mercenary_selector_widget::get_mercenary_selector;
```

### Route

```rust
.route(
    path::MATCH_REPORT_MERCENARY_SELECTOR,
    get(get_mercenary_selector),
)
```

---

## Module web — src/app/match_report/io/web/widgets/mod.rs (ou mod.rs racine)

```rust
pub mod mercenary_selector_widget;
```

Vérifier le fichier `mod.rs` qui déclare les widgets existants et y ajouter cette ligne.

---

## Fichiers créés / modifiés — récapitulatif

| Action | Fichier |
|--------|---------|
| MODIFIÉ | `src/app/match_report/ports.rs` — 2 DTOs + 2 méthodes de trait |
| MODIFIÉ | `src/infrastructure/match_report/ref_team_data_adapter.rs` — impl `find_roster_positions` |
| MODIFIÉ | `src/infrastructure/match_report/player_data_adapter.rs` — impl `find_player_counts_by_position` |
| MODIFIÉ | `src/app/match_report/routes.rs` — const + méthode `mercenary_selector` |
| MODIFIÉ | `src/app/match_report/router.rs` — import + route GET |
| MODIFIÉ | `src/app/match_report/io/web/widgets/mod.rs` — `pub mod mercenary_selector_widget` |
| MODIFIÉ | `src/app/match_report/io/web/mod.rs` — si nécessaire |
| CRÉÉ | `src/app/match_report/io/web/widgets/mercenary_selector_widget.rs` |
| CRÉÉ | `src/app/match_report/io/web/inducements_controller.rs` — étendu |
| CRÉÉ | `templates/match_report/widgets/mercenary-selector-widget.html` |
| MODIFIÉ | `src/app/match_report/use_cases/record_inducements_use_case.rs` |
| MODIFIÉ | `src/app/match_report/use_cases/init_temp_players_use_case.rs` |
| MODIFIÉ | `src/app/match_report/domain/match_report_pre_match.rs` |
| MODIFIÉ | `src/app/match_report/domain/error.rs` |
| MODIFIÉ | `templates/match_report/inducements.html` |
| MODIFIÉ | `templates/references/widgets/inducement-selector.html` |

---

## Plan de tests E2E (Playwright / pytest)

Fichier cible : `tests/e2e/test_match_report_step2_mercenaires.py`

### Prérequis

Même setup que `test_match_report_step2_inducements.py` : deux équipes en `ReadyToPlay`, fan factors enregistrés, TV enregistrées, un match report en phase inducements.

---

### TC-MERC-01 — Tab Mercenaires visible et widget se charge

```
1. Naviguer vers la page inducements d'une équipe
2. Vérifier que le tab "Mercenaires" est présent dans la tab bar
3. Cliquer sur le tab "Mercenaires"
4. Vérifier que le widget se charge (hx-trigger="mercenairesActivated from:body once")
5. Vérifier que des cartes de position sont affichées
```

---

### TC-MERC-02 — Journaliers exclus de la grille

```
1. Charger le widget Mercenaires pour une équipe dont le roster a des journaliers
2. Vérifier qu'aucune carte n'affiche le nom de la position journalier
```

---

### TC-MERC-03 — Sélection d'une position → hire panel

```
1. Charger le widget Mercenaires
2. Cliquer sur une carte de position (non disabled)
3. Vérifier que le hire panel s'affiche (x-show="selectedPosition")
4. Vérifier les deux options : "Recruter (Base)" et "Mercenaire Niv.1"
5. Vérifier que le prix affiché = base_cost + 30 kPo / base_cost + 80 kPo
```

---

### TC-MERC-04 — Recrutement et compteur 1/3

```
1. Cliquer "Recruter (Base)" sur une position
2. Vérifier que le panier affiche 1 mercenaire
3. Vérifier que le compteur dots passe à 1/3
4. Vérifier que le mercenaire est affiché dans le résumé panier (nom position + prix)
```

---

### TC-MERC-05 — Max 3 mercenaires enforced frontend

```
1. Recruter 3 mercenaires successivement
2. Vérifier que le compteur affiche 3/3
3. Vérifier que le bouton "Recruter" est disabled (ou que l'ajout est bloqué)
```

---

### TC-MERC-06 — Suppression d'un mercenaire via ✕

```
1. Recruter 2 mercenaires
2. Cliquer ✕ sur le premier mercenaire dans le panier
3. Vérifier que le compteur repasse à 1/3
4. Vérifier que la carte de position redevient sélectionnable si elle était la seule mercenaire de ce type
```

---

### TC-MERC-07 — Position full = carte disabled

```
1. Charger le widget pour une équipe dont une position est à max_qty (ex. 2/2 Blitzeurs)
2. Vérifier que la carte de cette position porte la classe disabled
3. Vérifier que cliquer dessus n'ouvre pas le hire panel
```

---

### TC-MERC-08 — Soumission formulaire avec mercenaires → temp players créés

```
1. Recruter 1 mercenaire
2. Soumettre le formulaire
3. Vérifier la navigation vers step 3 (ou l'UI correspondante au flow)
4. En step 3, vérifier que le joueur temporaire mercenaire apparaît dans le sélecteur de joueurs
```

---

### TC-MERC-09 — Soumission sans mercenaires (régression)

```
1. Ne sélectionner aucun mercenaire
2. Soumettre le formulaire avec uniquement des inducements classiques
3. Vérifier que la soumission réussit (comportement existant non régressé)
```
