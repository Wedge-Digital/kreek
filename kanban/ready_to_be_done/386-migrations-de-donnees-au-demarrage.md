# Migrations de données au démarrage

**Priorité : haute**
**Dépend de :** rien
**Bloque :** 387 et 388
**Fichiers :** `migrations/20260824000001_applied_data_migrations.sql`,
`src/infrastructure/data_migrations/mod.rs`, `src/main.rs`

## Objectif

Un endroit où poser une correction de données **qui a besoin du corpus de
règles**, exécutée une fois, au démarrage, avant que le serveur n'accepte une
requête.

```rust
pub async fn executer(state: &AppState, pool: &PgPool);
```

## Pourquoi ce n'est pas du SQL

Le corpus vit **hors du dépôt** : `REFERENCES__DIR` est fourni par
l'exploitant, et `load_references` ne le lit qu'au milieu de `run_server`, bien
après `run_migrations`. Une migration SQL ne peut donc savoir ni quelles
compétences sont Élite (carte 387), ni quels rosters portent
`LOW_COST_LINEMEN` (carte 388) : ces deux faits n'existent que dans des
fichiers JSON que la base ne voit pas.

D'où une seconde famille de migrations, écrites en Rust, qui reçoivent les
mêmes ports que l'application.

## La table de garde

```sql
CREATE TABLE applied_data_migrations (
    name       text PRIMARY KEY,
    applied_at timestamptz NOT NULL DEFAULT now()
);
```

Une ligne par migration appliquée. Le registre est **ordonné** et le restera :
la migration de la carte 388 recalcule des valeurs d'équipe à partir des
valeurs joueurs que celle de la carte 387 vient de corriger. Inverser les deux
donnerait des VEA fausses, sans que rien ne le signale.

## Où elle se branche

Dans `run_server`, entre `compose(cfg, pool)` et `build_router(state)` :
`compose` a construit tous les adapters, il n'y a rien à reconstruire, et le
serveur n'écoute pas encore.

## Un échec refuse le démarrage

`panic` — pas de `warn` puis on continue. Servir des pages sur des données à
moitié migrées est pire qu'un déploiement qui s'arrête : personne ne verra la
ligne d'avertissement, tout le monde verra les valeurs fausses.

Chaque migration est par ailleurs **idempotente** en elle-même : la table de
garde protège du rejeu normal, elle ne protège pas d'une interruption au
milieu. Une migration qui appende des événements écrit son nom dans la table
**dans la même transaction** que ses écritures — c'est la règle des projections
appliquée à un autre objet.

## Ce qu'elle journalise

Sur cible `kreek::`, donc `tracing::info!` depuis ce module — une cible hors
`kreek::` ne sortirait pas du filtre, et la migration serait muette en prod.

- une ligne par migration sautée (déjà appliquée)
- une ligne par migration appliquée, avec sa durée et le nombre d'agrégats touchés

## Checklist

- [ ] Migration SQL de la table de garde
- [ ] `src/infrastructure/data_migrations/mod.rs` : registre ordonné, exécution
      séquentielle, marque en base dans la transaction de la migration
- [ ] Appel dans `run_server`, avant `build_router`
- [ ] Journalisation `info` sur cible `kreek::`, sautée / appliquée / durée
- [ ] Test unitaire : une migration factice appliquée deux fois n'écrit qu'une
      fois ; une migration qui échoue ne marque rien
- [ ] `make lint`, `make check-arch`, `make test`
