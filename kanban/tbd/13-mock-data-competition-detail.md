# 100% mock data dans `competition_detail.rs`

**Priorité : moyenne**
**Fichier :** `src/app/competitions/io/web/competition_detail.rs`

## Problème

Les onglets Classement, Matchs, Équipes et Stats du détail de compétition servent de la donnée entièrement hardcodée (~180 lignes) :

```rust
fn mock_standings() -> Vec<StandingRow> { vec![...] }
fn mock_journees()  -> Vec<Journee>     { vec![...] }
fn mock_teams()     -> Vec<TeamCard>    { vec![...] }
fn mock_top_tds()   -> Vec<StatRow>     { vec![...] }
// ...
```

Ces fonctions sont appelées directement dans les handlers de production. L'utilisateur voit la même donnée fictive quelle que soit la compétition consultée. Il n'y a aucun marqueur `TODO` ou feature-flag visible.

## Action

1. Identifier quelles données ont une table en base (équipes, matchs) et lesquelles ne sont pas encore modélisées
2. Pour les données existantes : brancher les vrais repositories
3. Pour les données non encore modélisées : retourner des collections vides avec un état vide explicite dans l'UI, et créer un ticket de modélisation
4. Supprimer toutes les fonctions `mock_*` du fichier de production
