# Step 2 — Mercenaires — Use cases

## Périmètre

Une seule mutation : le formulaire POST du step 2 (`inducements_controller`).
Le GET du widget mercenary-selector n'a pas de use case — c'est une lecture pure orchestrée dans le handler.

Le use case existant `record_inducements_use_case` est **étendu** pour traiter les achats mercenaires.

---

## record_inducements_use_case — orchestration étendue

### Signature (inchangée, commande étendue)

```rust
pub async fn execute(
    cmd: RecordInducementsCommand,   // RecordInducementsCommand.mercenary_purchases ajouté (cf. 04-dtos)
    repo:             &dyn IMatchReportRepository,
    team_data:        &dyn ITeamDataPort,
    competition_data: &dyn ICompetitionDataPort,
    player_data:      &dyn IPlayerDataPort,
) -> Result<RecordInducementsOutcome, RecordInducementsError>
```

### Orchestration

Les étapes existantes sont **inchangées**. Les nouvelles étapes s'intercalent après `validate_purchase_uids` et avant l'appel domaine.

```
1. [existant] load_pre_match_with_tv(repo, mr_id)
2. [existant] fetch_tier_rules(pm, team_id, team_data, competition_data)
3. [existant] validate_purchase_uids(purchases, tier)
4. [existant] fetch_treasury(team_id, team_data)

── Nouvelles étapes ──────────────────────────────────────────────────────────

5. [NOUVEAU]  validate_mercenary_positions(mercenary_purchases, team_data)
              → Vec<ValidatedMercenary>   (voir ci-dessous)

6. [NOUVEAU]  fetch_player_counts(team_id, player_data)
              → Vec<PositionCountDto>

── Appel domaine ─────────────────────────────────────────────────────────────

7. [étendu]   pm.record_inducements(
                  team_id, purchases_tuples, budget, allowed_specs,
                  opponent_star_uids, recorded_by,
                  validated_mercs,    // NOUVEAU
                  player_counts,      // NOUVEAU
              )
              → (MatchReportPreMatch, Vec<Event>)

8. [existant] repo.append_many(events)
9. [existant] init_temp_players_use_case::execute(...)
10.[existant] route_outcome(pm)
```

---

### Étape 5 — validate_mercenary_positions

```rust
async fn validate_mercenary_positions(
    purchases: &[MercenaryPurchaseCmd],
    team_data: &dyn ITeamDataPort,
    team_id:   &TeamId,
) -> Result<Vec<ValidatedMercenary>, RecordInducementsError>
```

Pour chaque `MercenaryPurchaseCmd` :

1. Cherche la `RosterPositionDto` correspondante dans `find_roster_positions(team_id)` par `position_id.to_string()`
2. Si absente : `Err(RecordInducementsError::InvalidMercenaryPosition(cmd.position_id.clone()))`
3. Si `is_journalier` : `Err(RecordInducementsError::MercenaryPositionIsJournalier(cmd.position_id.clone()))`
4. Calcule le coût : `position.base_cost + cmd.level.extra_cost()`
5. Retourne un `ValidatedMercenary` (struct interne au use case)

```rust
struct ValidatedMercenary {
    position_id:   PositionId,
    position_name: String,
    level:         MercenaryLevel,
    cost:          u32,
    max_qty:       u8,    // transmis au domaine pour valider la limite roster
}
```

---

### Étape 6 — fetch_player_counts

```rust
async fn fetch_player_counts(
    team_id:     &TeamId,
    player_data: &dyn IPlayerDataPort,
) -> Result<Vec<PositionCountDto>, RecordInducementsError>
```

Appelle `player_data.find_player_counts_by_position(team_id)`. Erreur mappée en `RecordInducementsError::PlayerCountUnavailable`.

Ces counts sont transmis au domaine avec les `ValidatedMercenary` — le domaine valide la limite roster (count_in_team + nb de mercos pour cette position ≤ max_qty).

---

### Erreurs applicatives (enum étendu)

```rust
pub enum RecordInducementsError {
    // ── existantes ──────────────────────────────
    NotFound,
    NotInPreMatchPhase,
    TeamValuesNotRecorded,
    TreasuryUnavailable(String),
    TierRulesUnavailable(String),
    UnauthorizedInducement(String),
    Domain(DomainError),
    Repository(String),

    // ── nouvelles ───────────────────────────────
    InvalidMercenaryPosition(PositionId),       // position_id absent du roster de l'équipe
    MercenaryPositionIsJournalier(PositionId),  // journalier non recruitable comme mercenaire
    PlayerCountUnavailable(String),             // impossible de lire les counts par position
}
```

Les règles "max 3 mercenaires" et "limite roster cumulée" sont validées par le domaine et remontent via `Domain(DomainError)`.

---

## init_temp_players_use_case — collect_mercs (ajustement)

Pas de nouvelle signature. Seule `collect_mercs` est modifiée pour lire le `position_id` depuis l'UID encodé :

```
uid "MERCO:{position_id}:{level}" → TempPlayerKind::Mercenary { position_uid: position_id }
```

Le use case `init_temp_players` est appelé comme avant, aucun nouveau paramètre.

---

## Pas de nouveau use case

| Handler | Use case | Raison |
|---------|----------|--------|
| `GET mercenary_selector_widget` | aucun | Lecture pure, orchestration dans le handler |
| `POST inducements_controller` | `record_inducements_use_case` (étendu) | Mutation existante élargie |

---

## Règles métier identifiées à cette étape

- La validation `position_id ∈ roster` et `¬is_journalier` est une responsabilité du use case (données cross-BC)
- Le calcul du coût (`base_cost + extra_cost`) est arithmétique pure, effectué dans le use case via `MercenaryLevel::extra_cost()`
- Les règles "max 3" et "limite roster cumulée" sont des invariants domaine — délégués à `pm.record_inducements` (Phase 6)
- `PlayerCountUnavailable` ne devrait pas se produire en production mais est traité explicitement (principe de robustesse)
