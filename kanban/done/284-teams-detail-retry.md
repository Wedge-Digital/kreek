# BC teams — retry borné sur team_detail pour tolérer le délai de projection async post-création

**Priorité : haute**
**Contexte :** investigation du 2026-08-04. La redirection après soumission d'équipe (`src/app/team_creation/io/web/build_team/submit_team.rs:109-120`) pointe vers `team_detail`, alors que la ligne `team_event_store`/`team_proj` du BC `teams` n'est écrite que par un listener cross-BC asynchrone (`src/app/teams/io/app_events/team_created_listener.rs`), déclenché en aval de deux sauts `tokio::spawn` (publisher `team_creation` → bus applicatif → listener `teams`). Rien ne garantit que cette écriture soit commitée avant que le navigateur suive le `HX-Redirect`, d'où un 404 possible sur la page de détail juste après création.

## Objectif

Dans le handler `team_detail` (`src/app/teams/io/web/team_detail.rs:283-305`), si `find_by_id` renvoie `None`, retenter avec un court backoff avant de renvoyer 404, au lieu d'échouer immédiatement sur la première lecture.

## Conception

### Emplacement

La logique de retry vit dans la couche web (`team_detail.rs`), **pas** dans le repository. C'est une tolérance de latence propre à ce point d'entrée HTTP (juste après une redirection de création), pas une règle de lecture générale — les autres appelants de `find_by_id` ne doivent pas hériter d'un comportement de retry silencieux.

Extraire une fonction dédiée (règle des 20 lignes), par exemple :

```rust
async fn find_team_with_retry(
    repo: &dyn ITeamRepository,
    team_id: &str,
) -> Option<Team> {
    const MAX_ATTEMPTS: u32 = 3;
    const BACKOFF: Duration = Duration::from_millis(50);

    for attempt in 0..MAX_ATTEMPTS {
        if let Ok(Some(team)) = repo.find_by_id(team_id).await {
            return Some(team);
        }
        if attempt + 1 < MAX_ATTEMPTS {
            tokio::time::sleep(BACKOFF).await;
        }
    }
    None
}
```

Le handler `team_detail` appelle cette fonction à la place de l'appel direct à `find_by_id`, et renvoie 404 seulement si elle renvoie `None`.

### Paramètres

- 3 tentatives max, backoff de 50ms entre chaque → ~100ms de latence ajoutée **seulement** dans le cas où la donnée n'est pas encore là. Le cas normal (trouvé au premier essai) n'est pas affecté.

### Observabilité

Si les tentatives s'épuisent sans trouver la team, logger un `tracing::warn!` avant de renvoyer 404 — permet de détecter en production le cas aggravant où l'event a été perdu (`RecvError::Lagged` sur le bus interne) et où le 404 est en réalité permanent, pas juste une histoire de délai.

## Checklist

- [ ] Fonction `find_team_with_retry` dans `src/app/teams/io/web/team_detail.rs`
- [ ] Handler `team_detail` utilise cette fonction à la place de l'appel direct
- [ ] `tracing::warn!` si les tentatives sont épuisées, avant le 404
- [ ] `cargo check` passe
- [ ] Test unitaire : faux repository renvoyant `None` puis `Some` à la Nème tentative → `find_team_with_retry` renvoie bien la team
- [ ] Test unitaire : faux repository renvoyant toujours `None` → `find_team_with_retry` renvoie `None` après épuisement des tentatives
- [ ] Test E2E : régression du parcours normal (créer équipe → redirect → détail se charge en 200) — pas de tentative de simuler la race elle-même en E2E, dépendante du scheduler tokio
- [ ] `make check-arch` passe
