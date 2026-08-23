# `competitions` réagit au retrait d'un membre

**Priorité : moyenne**
**Dépend de :** 367, qui fait émettre l'app event
**Conception :** `docs/specs/space-admin/membres/07-integration.md`
**Fichiers :** `src/app/competitions/io/app_events/user_unsubscribed_listener.rs`,
`context.rs` du BC, `io/repository/`

## Objectif

Un coach retiré d'un espace peut être **administrateur d'une compétition de cet
espace**. `competitions_members` est vivante et `competitions.space_id` la
scope : il faut l'en retirer.

C'est la seule conséquence inter-BC réelle du retrait.

## L'app event existe déjà

`SpacesAppEvent::UserUnsubscribed` est dans l'enum, avec son type
`"UserUnsubscribed"`, et personne ne l'émet ni ne l'écoute. La carte 365 lui
donne son mapping, celle-ci lui donne son premier auditeur.

## La convention de nommage n'est pas cosmétique

```rust
pub fn init(app_event_bus: &EventBus, repo: Arc<dyn …>)
```

`init(app_event_bus: …)` signale à l'**axe 5** de `check-arch` qu'il s'agit d'un
listener cross-BC, exempté de la règle de transaction unique. Un événement déjà
committé ailleurs ne peut pas partager sa transaction. Nommer le paramètre
`event_bus` ferait échouer la vérification — à juste titre.

## L'effet, et rien d'autre

```sql
DELETE FROM competitions_members
WHERE  coach_id = $1
AND    competition_id IN (SELECT id FROM competitions WHERE space_id = $2)
```

Du SQL de `competitions` sur des tables de `competitions` : la souveraineté est
respectée, l'espace n'étant qu'un critère porté par l'événement.

**Le listener ne touche pas aux équipes.** `team_proj.coach_id` continue de
pointer sur le coach retiré, l'équipe reste engagée, la compétition se déroule.
C'est la règle 5 de la phase 1, et l'état accepté est une équipe dans l'espace
dont le propriétaire n'y a plus accès.

**Il ne touche pas non plus les trois caches de `competitions`** —
`competitions__space_cache`, `competitions__user_cache`,
`competitions__user_space_cache` — parce qu'elles **n'existent pas** :
`20260525000001_drop_competitions_cache.sql` les supprime en `CASCADE`, dix
jours après leur création. Vérifié sur la base de dev, `pg_tables` ne les
connaît pas.

## Checklist

- [ ] Listener suivant le patron du dépôt : `spawn_listener(module_path!(), …)`,
      filtre sur `event_type`, `tokio::spawn` sous
      `tracing::info_span!("app_event", event, event_id)`
- [ ] Paramètre nommé `app_event_bus` — axe 5
- [ ] Câblé dans le `context.rs` de `competitions`
- [ ] Méthode de dépôt et fichier SQL dédiés
- [ ] Test unitaire du listener sur un dépôt factice, comme
      `user_created_listener`
- [ ] Test d'intégration sur `competitions_members`, vraie `PgPool` :
  - [ ] le coach est retiré des compétitions de **cet** espace
  - [ ] il **reste** administrateur des compétitions d'un **autre** espace
- [ ] Le second test est la raison d'être des deux : sans lui, un `DELETE` qui
      oublierait le sous-`SELECT` passerait
- [ ] `make lint`, `make check-arch`, `make test` passent
