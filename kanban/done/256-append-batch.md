# `append_batch` — appliquer un lot d'événements atomiquement

**Priorité : haute**
**Dépend de :** 255
**Bloque :** 263, 268
**Spec :** `docs/specs/phases-recrutement-renvois/recrutement/05-use-cases.md` §1
**Fichiers :** `src/app/teams/ports.rs`, `src/app/teams/io/repository/team_repository.rs`

## Problème

`TeamRepository::append` **ouvre sa propre transaction** (`self.pool.begin()`) et la
committe. Appliquer un lot de N événements atomiquement est donc impossible : N appels
valent N transactions, et une panne au milieu laisse l'équipe dans un état incohérent
— la moitié des achats appliqués, la phase non validée.

Or la validation d'une phase applique **tout le brouillon d'un coup** : N recrutements
ou N renvois, plus l'événement de transition de phase.

## Action

### 1. Nouvelle méthode

```rust
async fn append_batch(
    &self,
    team_id: &str,
    events: &[TeamDomainEvent],
    expected_version: u64,
) -> Result<u64, RepositoryError>;
```

Une seule transaction :

1. insertion des N événements à versions croissantes `expected_version + 1 … + N`
2. `update_projection_in_tx` pour chacun
3. alimentation du grand livre pour ceux dont `treasury_movement()` retourne `Some`
4. commit

`append` reste **inchangée** — les mutations unitaires existantes ne bougent pas.

### 2. Concurrence

La détection reste celle en place : la contrainte unique `team_event_store_version`
sur `(team_id, version)`, dont le nom est lu dans l'erreur Postgres
(`team_repository.rs:204`). Un conflit sur **n'importe lequel** des N événements fait
échouer toute la transaction.

### 3. Publication sur le bus interne

La carte 251 fait publier `append` sur le bus interne de `teams`. `append_batch` doit
publier **les N événements**, dans l'ordre, après le commit.

C'est ce qui permet au listener de purge (carte 257) et au recalcul de valeur d'équipe
(carte 251) de réagir à `RecruitmentPhaseValidated` et `DismissalsPhaseValidated`.

### 4. Lot vide accepté

Un coach peut terminer sa phase sans rien acheter : `append_batch` avec un seul
événement de transition doit fonctionner. Ne pas rejeter les lots courts.

## Checklist

- [ ] `append_batch` dans `ITeamRepository` et son implémentation
- [ ] Une seule transaction pour les N événements, projection et grand livre
- [ ] Versions croissantes, contrainte d'unicité respectée
- [ ] Publication des N événements sur le bus interne, dans l'ordre, après commit
- [ ] Test : lot de 3 → 3 événements, 3 lignes de projection, versions consécutives
- [ ] Test : conflit de version sur le 2ᵉ → **aucun** des 3 n'est écrit
- [ ] Test : lot d'un seul événement de transition → accepté
- [ ] `append` inchangée, ses tests toujours verts
- [ ] `make check-arch` au vert, `make test` au vert
