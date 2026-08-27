# Savoir quelles équipes jouent un roster

**Épic :** E10 · **Ordre :** 1 · **Dépend de :** rien
**Conception :** `docs/specs/roster-personnalise/editeur-de-roster/03-back.md`

## Objectif

Répondre à « combien d'équipes jouent ce roster ». **Aucune requête ne sait le
faire aujourd'hui**, et c'est utile bien au-delà du roster personnalisé.

## Le constat

Aucune table ne porte le **uid** du roster d'une équipe. `team_proj` n'a qu'un
`roster_name` d'affichage. L'information vit dans la charge utile de
`TeamCreated`, au fond de l'event store de `teams` :

```sql
select payload->>'roster_id', count(*) from team_event_store
where event_type='TeamCreated' group by 1;
--  DEMO_GRANIT | 1855
```

Interroger l'event store fonctionnerait, mais ce serait **balayer un flux
d'événements pour répondre à une question d'état**, sans index — ce que le
projet range du côté des projections.

## Conception

### 1. La migration — schéma et rattrapage ensemble

```sql
-- migrations/<date>_team_proj_roster_id.sql
ALTER TABLE team_proj ADD COLUMN roster_id TEXT;

UPDATE team_proj p
SET    roster_id = e.payload->>'roster_id'
FROM   team_event_store e
WHERE  e.team_id = p.team_id
  AND  e.event_type = 'TeamCreated'
  AND  p.roster_id IS NULL;

CREATE INDEX ON team_proj (roster_id);
```

**Du SQL, et non le registre Rust.** `infrastructure/data_migrations/` existe
pour les rattrapages qui ont besoin du **corpus** — `m001_bonus_elite` et
`m002_recalcul_valeurs_equipe` recalculent des valeurs à partir des prix de
référence, et ne peuvent donc être écrits qu'en Rust. Ici la donnée est déjà en
base.

Le précédent : `20260824000001_..._notifications_off_for_existing.sql` corrige
des données existantes en SQL pur.

**L'index après le rattrapage**, pas avant : le construire sur une colonne qu'on
remplit coûte deux fois.

**La colonne reste nullable.** Une équipe dont l'événement serait introuvable
garde `NULL` plutôt que de faire échouer la migration — et un `NULL` ne compte
dans aucun `WHERE roster_id = $1`, donc il ne verrouille rien à tort.

Contrôle après passage :

```sql
SELECT count(*) FROM team_proj WHERE roster_id IS NULL;   -- doit valoir 0
```

### 2. Le projecteur écrit la colonne désormais

`team_repository.rs:32` destructure `TeamCreated` **sans prendre `roster_id`**,
alors que l'agrégat le porte (`team.rs:58`). L'ajouter à la destructuration, à
l'`INSERT` et à l'`ON CONFLICT DO UPDATE`.

**C'est le geste qui empêche la colonne de se re-vider** : sans lui, le
rattrapage serait juste le jour de la livraison et faux le lendemain.

### 3. Le port et son adapter

```rust
// references/ports.rs — le fichier n'existe pas
#[async_trait]
pub trait IRosterUsagePort: Send + Sync {
    async fn count_teams_using(&self, roster_uid: &str) -> Result<u32, String>;
}
```

`src/infrastructure/references/roster_usage_adapter.rs` — **le dossier n'existe
pas non plus**, `references` n'ayant jamais eu besoin de sortir de lui-même. Il
est le seul de ce BC à connaître `teams`.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `la_migration_remplit_les_equipes_existantes` | intégration, vraie base |
| `une_equipe_creee_apres_porte_son_roster_id` | le projecteur |
| `count_teams_using_compte_juste` | l'adapter |
| `count_teams_using_rend_zero_sur_un_roster_inconnu` | et non une erreur |

## Checklist

- [ ] La migration : colonne, rattrapage, index — dans cet ordre
- [ ] Le contrôle `roster_id IS NULL` à zéro
- [ ] Le projecteur destructure et écrit `roster_id`
- [ ] `references/ports.rs` et `infrastructure/references/`
- [ ] Les quatre tests
- [ ] `make lint && make test && make check-arch`
