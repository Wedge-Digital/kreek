# `match_report` — Garde-fou de correction : ports, adapters, domain service

**Priorité : haute**
**Dépend de :** `227-mrc-domaine-depublication.md`
**Fichiers :** `src/app/match_report/ports.rs`, `src/app/match_report/use_cases/correction_eligibility_service.rs` (nouveau), `src/infrastructure/match_report/{ref_team_data_adapter,player_data_adapter}.rs`, `src/app/players/ports.rs`
**Spec :** `docs/specs/match-report-correction/recap/05-use-cases.md`, `07-integration.md`

## Objectif

Répondre à « ce rapport est-il corrigeable ? » en composant deux consultations
inter-BC en un value object domaine.

## Conception

### Ports (`match_report/ports.rs`)

```rust
// ITeamDataPort
async fn is_team_in_player_improvement(&self, team_id: &str) -> Result<bool, String>;

// IPlayerDataPort
async fn has_spent_spp_since_match(&self, team_id: &str, match_report_id: &str)
    -> Result<bool, String>;
```

Extension des ports existants, pas de nouveau port : un port unique répondant
d'un coup placerait la composition « 2 BCs → 1 verdict » dans un adapter, donc
dans `infrastructure/`.

### Adapter équipe

Miroir exact de l'`is_team_ready_to_play` voisine :

```rust
Ok(team.map(|t| t.game_phase == Some(GamePhase::PlayerImprovement)).unwrap_or(false))
```

Équipe introuvable ou dissoute (`game_phase == None`) → `false`, donc bloque.
Conforme à la règle 16 : l'imprécision du libellé est acceptée.

### Adapter joueur — accès à l'historique

`PlayerDataAdapter` ne dispose que de `IPlayerProjectionRepository`, qui ne
porte pas l'historique nécessaire. **Ne pas lui donner une `PgPool` nue** :
ajouter une méthode sur un port du BC `players`, qui reste propriétaire de ses
tables.

Requête sur `players_events` (`id BIGSERIAL` monotone, index sur `team_id`) :

```sql
SELECT EXISTS (
  SELECT 1 FROM players_events
  WHERE team_id = $1
    AND event_type IN ('PlayerSkillPurchased', 'PlayerStatIncreased')
    AND id > (
      SELECT MIN(id) FROM players_events
      WHERE team_id = $1
        AND event_type = 'MatchConcluded'
        AND payload -> 'MatchConcluded' -> 'context' ->> 'match_report_id' = $2
    )
)
```

**À confirmer avant de coder** : le chemin JSON suppose une représentation serde
*externally tagged* (`PlayerDomainEvent` ne porte aucun `#[serde(tag = ...)]`).
Vérifier sur une ligne réelle. Si le chemin ne tient pas, la variante suivante
est équivalente **sous le garde-fou « à chaud »** — le match corrigible étant
nécessairement le dernier :

```sql
AND id > COALESCE((SELECT MAX(id) FROM players_events
                   WHERE team_id = $1 AND event_type = 'MatchConcluded'), 0)
```

### Domain service (`use_cases/correction_eligibility_service.rs`)

```rust
pub async fn evaluate(
    home_team_id: &TeamId, away_team_id: &TeamId, match_report_id: &MatchReportId,
    team_data: &dyn ITeamDataPort, player_data: &dyn IPlayerDataPort,
) -> CorrectionEligibility
```

Les 4 appels de port partent en parallèle via `tokio::join!`
(`build_recap_template` utilise déjà ce pattern dans ce BC).

Ordre d'évaluation, déterministe :

```
home SPP → home phase → away SPP → away phase → Eligible
```

Toute erreur de port → `Blocked(EligibilityUnknown)` : **échouer fermé**
(règle 12).

Découpage (20 lignes) :

| Fonction | Nature |
|---|---|
| `evaluate` | async — lance les appels, délègue |
| `verdict_from(..4 résultats..)` | **pure, synchrone** |
| `blocker_for_side(side, spp, in_improvement)` | **pure, synchrone** |

Les deux fonctions pures rendent les tests d'ordre d'évaluation indépendants de
tout mock de port.

## Checklist

- [ ] `is_team_in_player_improvement` sur `ITeamDataPort` + adapter
- [ ] `has_spent_spp_since_match` sur `IPlayerDataPort` + adapter
- [ ] Méthode d'historique exposée par un port du BC `players` (pas de `PgPool` dans l'adapter)
- [ ] Représentation JSON des events vérifiée sur une ligne réelle avant de figer la requête
- [ ] `correction_eligibility_service` avec `verdict_from` et `blocker_for_side` pures
- [ ] Test : home avant away quand les 2 bloquent
- [ ] Test : SPP avant phase pour un même camp
- [ ] Test : erreur de port → `EligibilityUnknown`
- [ ] Test : les 2 camps sains → `Eligible`
- [ ] Test d'intégration de la requête `players_events` sur vraie base
- [ ] `make test` passe
- [ ] `make check-arch` passe
