# Poules — Phase 6 : Domaine ✅

## Logique métier

La distribution en poules est une logique simple sans invariants complexes. Pas d'agrégat dédié — les use cases manipulent directement les données via le repository.

### Algorithme de tirage aléatoire (random_draw)

```
1. Shuffle la liste des équipes enrolled
2. Pour chaque équipe (round-robin) :
   - Assigner au groupe avec le moins d'équipes
3. Persister toutes les assignations
```

### Règles

- On ne peut assigner que des équipes `Enrolled` (pas `PendingEnrollment`)
- Un tirage écrase les assignations existantes
- Un reset vide les assignations mais conserve les groupes
- Une équipe ne peut être que dans un seul groupe

## Tests unitaires

L'algorithme de distribution peut être testé unitairement :

1. `random_draw` avec 6 équipes et 2 groupes → chaque groupe a 3 équipes
2. `random_draw` avec 7 équipes et 2 groupes → un groupe a 4, l'autre 3
3. `random_draw` avec 0 équipes → erreur NoTeams
4. `random_draw` avec 0 groupes → erreur NoGroups
