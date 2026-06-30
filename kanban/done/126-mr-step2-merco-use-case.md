# BC match_report — Extension use case + collect_mercs

**Priorité : haute**
**Dépend de :** 125
**Contexte :** `docs/specs/match-report/step2-mercenaires/05-use-cases.md`, `04-dtos.md`

## Objectif

Étendre `record_inducements_use_case` pour traiter les achats mercenaires (validation, specs synthétiques, appel domaine) et corriger `collect_mercs` dans `init_temp_players_use_case`.

## Conception

### 1. MercenaryLevel + MercenaryPurchaseCmd — record_inducements_use_case.rs

Ajouter en haut du fichier use case :

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MercenaryLevel { Base, Lvl1 }

impl MercenaryLevel {
    pub fn extra_cost(&self) -> u32 {
        match self { Self::Base => 30, Self::Lvl1 => 80 }
    }
    pub fn as_str(&self) -> &'static str {
        match self { Self::Base => "base", Self::Lvl1 => "lvl1" }
    }
    pub fn try_from_str(s: &str) -> Result<Self, &'static str> {
        match s { "base" => Ok(Self::Base), "lvl1" => Ok(Self::Lvl1), _ => Err("niveau inconnu") }
    }
}

pub struct MercenaryPurchaseCmd {
    pub position_id: PositionId,
    pub level:       MercenaryLevel,
}
```

### 2. RecordInducementsCommand — ajouter le champ

```rust
pub struct RecordInducementsCommand {
    // champs existants ...
    pub mercenary_purchases: Vec<MercenaryPurchaseCmd>,  // NOUVEAU
}
```

### 3. RecordInducementsError — nouveaux variants

```rust
InvalidMercenaryPosition(PositionId),
MercenaryPositionIsJournalier(PositionId),
PlayerCountUnavailable(String),
```

### 4. Orchestration use case — nouvelles étapes

Après `validate_purchase_uids` et `fetch_treasury`, avant l'appel domaine :

**Étape 5 — validate_mercenary_positions**

```rust
async fn validate_mercenary_positions(
    purchases: &[MercenaryPurchaseCmd],
    team_data: &dyn ITeamDataPort,
    team_id:   &TeamId,
) -> Result<Vec<ValidatedMercenary>, RecordInducementsError>
```

Pour chaque `MercenaryPurchaseCmd` :
1. Cherche dans `find_roster_positions(team_id)` par `position_id.to_string()`
2. Absent → `InvalidMercenaryPosition`
3. `is_journalier` → `MercenaryPositionIsJournalier`
4. Calcule `cost = position.base_cost + level.extra_cost()`

```rust
struct ValidatedMercenary {
    position_id:   PositionId,
    level:         MercenaryLevel,
    cost:          u32,
    max_qty:       u8,
}
```

**Étape 6 — fetch_player_counts**

```rust
async fn fetch_player_counts(team_id: &TeamId, player_data: &dyn IPlayerDataPort)
    -> Result<Vec<PositionCountDto>, RecordInducementsError>
```

Appelle `find_player_counts_by_position(team_id)`, erreur → `PlayerCountUnavailable`.

**Avant l'appel domaine — build specs synthétiques**

Pour chaque `ValidatedMercenary`, construire un `AllowedInducementSpec` synthétique et un purchase tuple, puis les ajouter aux listes existantes :

```rust
let count_in_team = player_counts
    .iter()
    .find(|c| c.position_uid == merc.position_id.to_string())
    .map(|c| c.count)
    .unwrap_or(0);
let available = merc.max_qty.saturating_sub(count_in_team);
AllowedInducementSpec {
    uid:           InducementId(format!("MERCO:{}:{}", merc.position_id, merc.level.as_str())),
    max_qty:       InducementQty::try_new(available).unwrap_or(InducementQty::try_new(0).unwrap()),
    unit_cost:     InducementCost::try_new(merc.cost).expect("cost validated"),
    is_star_player: IsStarPlayer(false),
}
// purchase tuple : (uid, qty=1)
```

Grouper les mercenaires par uid (même position+level → qty > 1) avant de construire les tuples.

### 5. collect_mercs — init_temp_players_use_case.rs

Remplacer le filtre existant (`p.uid.0 == "MERCENARY_PLAYER"`) par :

```rust
.filter(|p| p.uid.0.starts_with("MERCO:"))
.map(|p| {
    let position_uid = p.uid.0.splitn(3, ':').nth(1).unwrap_or("").to_string();
    TempPlayer {
        id:           TempPlayerId(ulid::Ulid::new().to_string()),
        team_id:      team_id.clone(),
        kind:         TempPlayerKind::Mercenary { position_uid },
        display_name: None,
    }
})
```

## Checklist

- [ ] `MercenaryLevel` défini avec `extra_cost`, `as_str`, `try_from_str`
- [ ] `MercenaryPurchaseCmd` défini
- [ ] `RecordInducementsCommand.mercenary_purchases` ajouté
- [ ] `RecordInducementsError` enrichi (3 nouveaux variants)
- [ ] `validate_mercenary_positions` implémentée
- [ ] `fetch_player_counts` implémentée
- [ ] Build des specs synthétiques implémenté (groupement par uid, available slots)
- [ ] `collect_mercs` mis à jour (filtre `starts_with("MERCO:")`, extraction position_uid)
- [ ] `cargo build` passe
- [ ] Test manuel : soumission avec 0 mercenaires ne régresse pas
