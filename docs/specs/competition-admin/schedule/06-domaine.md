# Calendrier — Phase 6 : Domaine ✅

## Agrégat MatchDay

Fichier : `src/app/competitions/domain/match_day.rs`

Pas event sourcé — CRUD simple. L'ID est un ULID unique utilisé par le futur BC MatchReport.

```rust
pub struct MatchDay {
    pub id: String,
    pub season_id: String,
    pub name: String,
    pub day_type: MatchDayType,
    pub date_start: Option<time::Date>,
    pub date_end: Option<time::Date>,
    pub position: i32,
    pub pairings: Vec<Pairing>,
}

pub struct Pairing {
    pub id: String,
    pub home_team_id: String,
    pub away_team_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchDayType {
    FixedDate,
    TimeFrame,
    Rest,
}
```

## Fonction pure de génération de pairings

Fichier : `src/app/competitions/domain/match_day.rs`

```rust
pub fn generate_round_pairings(
    teams: &[String],
    already_played: &HashSet<(String, String)>,
) -> Vec<(String, String)>
```

### Algorithme

```
1. Générer toutes les paires possibles pour cette poule : (A,B), (A,C), ...
   Normaliser chaque paire : (min, max) pour que (A,B) == (B,A)
2. Filtrer : retirer les paires présentes dans already_played
3. Si aucune paire disponible → cycle épuisé → réautoriser toutes les paires (vider le filtre)
4. Allouer les paires pour cette journée (greedy) :
   - Ensemble "équipes_occupées" = vide
   - Pour chaque paire disponible (dans l'ordre) :
     - Si ni home ni away n'est dans équipes_occupées → ajouter la paire, marquer les deux équipes
   - Retourner les paires allouées
```

### Règle métier

Chaque paire d'équipes ne se rencontre qu'une seule fois dans la saison, tant que le nombre de journées le permet :
- N équipes → N×(N-1)/2 paires possibles
- Si toutes les paires sont consommées, un nouveau cycle commence

## Tests unitaires

Fichier : `src/app/competitions/domain/match_day.rs` (module `#[cfg(test)]`)

1. **4 équipes, aucune paire jouée** → 2 paires générées, les 4 équipes jouent
2. **4 équipes, 2 paires déjà jouées** → 2 nouvelles paires différentes des précédentes
3. **4 équipes, 6 paires jouées (cycle épuisé)** → recommence le cycle, 2 paires générées
4. **3 équipes** → 1 paire générée (une équipe est en bye)
5. **0 ou 1 équipe** → aucune paire générée (vec vide)
6. **Vérification de normalisation** → (A,B) et (B,A) comptent comme la même paire
