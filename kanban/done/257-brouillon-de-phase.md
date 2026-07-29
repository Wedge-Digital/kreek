# Brouillon de phase — table, repository, version optimiste, purge

**Priorité : haute**
**Dépend de :** 251 (bus interne de `teams`)
**Bloque :** 262, 267
**Spec :** `docs/specs/phases-recrutement-renvois/recrutement/03-back.md` §1
**Fichiers :** `src/app/teams/ports.rs`,
`src/app/teams/io/repository/phase_draft_repository.rs` (nouveau),
`src/app/teams/io/listeners/phase_draft_purge_listener.rs` (nouveau),
`src/app/teams/context.rs`, migration

## Problème

Les phases de recrutement et de renvois fonctionnent au **panier** : le coach accumule
des lignes, les annule librement, et rien n'est engagé avant la validation de phase.
Ce panier vit côté serveur (décision D1) — il doit donc être persisté.

Rien de tel n'existe dans `teams`. Le modèle à reprendre est celui de la construction
d'équipe : `team_roster_selections (id, space_id, state JSONB, …)`, agrégat chargé et
sauvé entier à chaque mutation.

## Action

### 1. Une table pour les deux phases

```sql
CREATE TABLE teams__phase_drafts (
    team_id    TEXT        NOT NULL,
    phase      TEXT        NOT NULL,        -- 'Recruitment' | 'Dismissals'
    space_id   TEXT        NOT NULL,
    state      JSONB       NOT NULL,
    version    INT         NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (team_id, phase)
);
```

Une seule table, discriminée par phase : les deux brouillons ne coexistent jamais,
puisque les phases sont séquentielles.

**Seules les lignes du brouillon sont sérialisées** dans `state`. Le catalogue du
roster, l'effectif et la trésorerie sont rechargés à chaque hydratation — c'est ce qui
garantit qu'un brouillon vieux de dix minutes est évalué contre l'état d'aujourd'hui.

### 2. Version optimiste

```sql
UPDATE teams__phase_drafts
   SET state = $3, version = version + 1, updated_at = now()
 WHERE team_id = $1 AND phase = $2 AND version = $4
```

Zéro ligne affectée ⇒ `RepositoryError::ConcurrentWrite`.

L'event store détecte la concurrence par une contrainte d'unicité ; ici la table ne
porte qu'une ligne par équipe et par phase, donc la garde passe par le `WHERE`. Même
erreur, même vocabulaire, mécanisme adapté au type de table.

**La course à la création compte aussi** : deux onglets qui ajoutent en même temps
produisent un conflit de clé primaire, à ramener à `ConcurrentWrite` et non à une
erreur base.

### 3. Le port

```rust
#[async_trait]
pub trait IPhaseDraftRepository: Send + Sync {
    async fn load(&self, team_id: &str, phase: GamePhase)
        -> Result<Option<PhaseDraftState>, RepositoryError>;
    async fn save(&self, draft: &PhaseDraftState, expected_version: u32)
        -> Result<u32, RepositoryError>;
    async fn delete(&self, team_id: &str, phase: GamePhase)
        -> Result<(), RepositoryError>;
}
```

### 4. Le listener de purge (décision D6)

`init(event_bus: &EventBus, …)` — c'est cette convention que `check-arch` (axe 5)
utilise pour reconnaître un listener **intra-BC**.

Supprime **les deux** brouillons à chaque entrée en `ReadyToPlay`, soit les quatre
événements dont `apply()` pose cette phase : `TeamEnrolled`,
`DismissalsPhaseValidated`, `MatchReportingCancelled`, `CostlyMistakesApplied`.

Listener **distinct** de celui du recalcul de valeur d'équipe (carte 251), bien
qu'abonné aux mêmes événements : une responsabilité, un listener.

Effet utile : un brouillon ne survit jamais à un tour de séquence. Le coach qui revient
trouve une page vierge, jamais des lignes fantômes.

## Checklist

- [ ] Migration `teams__phase_drafts`
- [ ] `IPhaseDraftRepository` et son implémentation
- [ ] `UPDATE … WHERE version = $` → `ConcurrentWrite` sur zéro ligne
- [ ] Conflit de clé primaire à la création → `ConcurrentWrite`, pas erreur base
- [ ] Listener de purge en `init(event_bus: …)`, abonné aux 4 événements
- [ ] Test : deux `save` avec la même version attendue → le second échoue
- [ ] Test : entrée en `ReadyToPlay` → les deux brouillons disparaissent
- [ ] `make check-arch` au vert (axe 5), `make test` au vert
