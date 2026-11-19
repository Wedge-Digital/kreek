# Groupement O(n²) dans `find_with_seasons`

**Priorité : moyenne**
**Fichier :** `src/app/competitions/io/repository/competition_repository.rs:122-139`

## Problème

Le regroupement des saisons par compétition est fait en Rust avec une recherche linéaire :

```rust
for r in rows {
    match result.iter_mut().find(|c| c.competition_id == r.competition_id) {
        Some(existing) => existing.seasons.push(...),
        None => result.push(...),
    }
}
```

Pour N compétitions avec M saisons, la complexité est O(N×M). Acceptable aujourd'hui, mais le pattern ne passe pas à l'échelle.

## Action

Deux options :

**Option A — SQL** : Utiliser `array_agg` côté PostgreSQL pour retourner les saisons groupées directement, en évitant le post-traitement Rust.

**Option B — Rust** : Remplacer la recherche linéaire par une `IndexMap<String, CompetitionWithSeasons>` pour descendre à O(M) :

```rust
let mut map: IndexMap<String, CompetitionWithSeasons> = IndexMap::new();
for r in rows {
    map.entry(r.competition_id.clone())
        .or_insert_with(|| CompetitionWithSeasons { ... })
        .seasons.push(...);
}
let result: Vec<_> = map.into_values().collect();
```
