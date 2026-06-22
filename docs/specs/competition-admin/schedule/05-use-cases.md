# Calendrier — Phase 5 : Use cases ✅

## Use case : `generate_pairings.rs`

Fichier : `src/app/competitions/use_cases/admin/generate_pairings.rs`

Génère les rencontres d'une journée par round-robin intra-poule, en respectant la règle de non-répétition.

### Signature

```rust
pub async fn execute(
    match_day_id: &str,
    season_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
    group_repo: &dyn IGroupRepository,
) -> Result<(), GenerateError>
```

### Orchestration

1. Charger la journée — vérifier qu'elle n'est pas de type `Rest`
2. Vider les pairings existants de cette journée
3. Charger les groupes avec leurs équipes
4. Charger tous les pairings existants des **autres** journées de la saison (paires déjà jouées)
5. Pour chaque poule, générer les paires home/away en excluant celles déjà jouées
6. Si toutes les paires d'une poule sont épuisées, recommencer le cycle (réautoriser toutes les paires)
7. Sauvegarder les pairings avec des IDs ULID uniques

### Règle métier : non-répétition des paires

Chaque paire d'équipes (A vs B = B vs A) ne doit se rencontrer qu'une seule fois dans la saison, tant que le nombre de journées le permet.

- N équipes dans une poule → N×(N-1)/2 paires possibles
- Si nb journées ≤ N-1 : la règle est respectée (chaque équipe joue au plus une fois par journée, chaque paire apparaît au plus une fois)
- Si nb journées > N-1 : toutes les combinaisons sont épuisées, on recommence un nouveau cycle

### Erreurs

```rust
#[derive(Debug)]
pub enum GenerateError {
    MatchDayNotFound,
    IsRestDay,
    NoGroups,
    Repository(String),
}
```

## Use case : `generate_all_pairings.rs`

Fichier : `src/app/competitions/use_cases/admin/generate_all_pairings.rs`

### Signature

```rust
pub async fn execute(
    season_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
    group_repo: &dyn IGroupRepository,
) -> Result<(), GenerateError>
```

### Orchestration

1. Charger toutes les journées de la saison
2. Pour chaque journée non-repos, appeler `generate_pairings::execute`
3. L'ordre des journées est respecté (position croissante) — les premières journées consomment les paires en premier

## Opérations CRUD (pas de use case)

Les opérations suivantes sont directes sur le repository, orchestrées par le handler :

- Ajouter une journée → `match_day_repo.save_match_day()`
- Ajouter un repos → idem avec `day_type = Rest`
- Modifier dates/type → `match_day_repo.save_match_day()`
- Supprimer une journée → `match_day_repo.delete_match_day()`
- Ajouter un match → `match_day_repo.save_pairing()`
- Supprimer un match → `match_day_repo.delete_pairing()`
- Vider les matchs d'une journée → `match_day_repo.clear_pairings()`
- Vider toutes les rencontres → `match_day_repo.clear_all_pairings()`

## Algorithme round-robin intra-poule (détail)

Pour une poule de N équipes :

```
1. Lister toutes les paires possibles : (A,B), (A,C), (A,D), (B,C), (B,D), (C,D)
2. Soustraire les paires déjà jouées sur les autres journées
3. Si le résultat est vide → toutes les paires sont épuisées → réautoriser toutes les paires
4. Attribuer les paires disponibles à cette journée :
   - Chaque équipe joue au plus 1 match par journée
   - Sélectionner les paires greedily (première paire dispo dont les deux équipes sont libres)
```

### Tests unitaires

1. 4 équipes, 1 poule, 3 journées → chaque paire apparaît exactement une fois (6 paires / 2 par journée)
2. 4 équipes, 1 poule, 4 journées → la 4ème journée recommence le cycle
3. 6 équipes, 2 poules de 3 → chaque poule a ses propres paires, pas de match inter-poule
4. Journée de repos → erreur `IsRestDay`
5. Pas de groupes → erreur `NoGroups`
