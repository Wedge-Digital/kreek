# `ranking` — Compensation d'une dépublication

**Priorité : haute**
**Dépend de :** `231-mrc-publisher-app-events.md`
**Fichiers :** `migrations/` (nouvelle), `src/app/ranking/ports.rs`, `src/app/ranking/io/repository/ranking_repository.rs`, `src/app/ranking/use_cases/revert_match_ranking_use_case.rs` (nouveau), `src/app/ranking/io/app_events/match_report_unpublished_listener.rs` (nouveau), `src/app/ranking/context.rs`
**Spec :** `docs/specs/match-report-correction/recap/05-use-cases.md`, `07-integration.md`

## Objectif

Retirer du classement les 2 lignes du match dépublié. Inclut la dette
préexistante n°2 : l'absence d'unicité sur `ranking_lines`.

## Conception

### Migration — index unique

```sql
CREATE UNIQUE INDEX ranking_lines_match_team
    ON ranking_lines (match_report_id, team_id);
```

`ranking_lines` n'a aujourd'hui que `id` en clé primaire. Sans cet index, un
double comptage passe inaperçu ; avec, il échoue bruyamment.

**Vérifier l'absence de doublons préexistants avant d'appliquer** — la création
de l'index échouerait sinon. Si des doublons existent, les traiter dans cette
carte et le consigner ici.

### Port et repository

```rust
// IRankingRepository
async fn delete_lines_for_match(&self, match_report_id: &str)
    -> Result<(), RankingRepositoryError>;
```

```sql
DELETE FROM ranking_lines WHERE match_report_id = $1
```

### Use case

```rust
pub async fn execute(match_report_id: &MatchReportId, repo: &dyn IRankingRepository)
    -> Result<(), RevertMatchRankingError>
```

Aucune règle de compétition à charger, **aucun recalcul en cascade** : les 2
lignes du match sont les dernières de chaque équipe, garanti par le garde-fou
« à chaud ». C'est précisément ce que la restriction achète.

Symétrique de `record_match_ranking_use_case`, qui existe déjà — d'où le choix
d'un use case ici alors que `competitions` fait du SQL direct.

### Listener et câblage

Souscrit à l'app event bus, filtre `MatchReportUnpublished`, appelle le use case.
Câblé dans `ranking::context::init_listeners`, déjà invoqué depuis `main.rs`.

### Idempotence

`DELETE` : un second passage supprime 0 ligne (règle 11). Aucune garde
supplémentaire.

## Checklist

- [ ] Doublons préexistants vérifiés, et traités le cas échéant
- [ ] Migration d'index unique `(match_report_id, team_id)`
- [ ] `delete_lines_for_match` sur le port et le repository
- [ ] `revert_match_ranking_use_case`
- [ ] Listener créé et câblé dans `context.rs`
- [ ] Test d'intégration : les 2 lignes du match sont supprimées
- [ ] Test d'intégration : un second appel supprime 0 ligne sans erreur
- [ ] Test d'intégration : les lignes d'un autre match ne sont pas touchées
- [ ] Test : après compensation puis rejeu, une seule paire de lignes existe
- [ ] `make test` passe
- [ ] `make check-arch` passe
