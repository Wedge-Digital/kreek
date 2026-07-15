# BC `players` — Use case d'augmentation de caractéristique

**Priorité : haute**
**Dépend de :** `175-players-domain-improvement.md`, `177-players-skill-catalog-port.md`, `178-players-purchase-skill-use-case.md` (réutilise `resolve_stat_cost`)
**Contexte :** `players/use_cases` + `players/io/web`

## Objectif

Câbler l'augmentation d'une caractéristique (MA/ST/AG/PA/AV), même patron
que l'achat de compétence mais sans notion de mode/catégorie. Spec
complète : `docs/specs/player-spp-spending/README.md`.

---

## Conception

### Commande + use case

```rust
pub struct IncreaseStatCommand { pub player_id: PlayerId, pub stat: StatKind }

pub enum IncreaseStatError { PlayerNotFound, Domain(DomainError), Repository(RepositoryError) }

pub async fn execute(cmd: IncreaseStatCommand, player_repo: &dyn IPlayerRepository, catalog: &dyn ISkillCatalogPort) -> Result<(), IncreaseStatError> {
    let player = player_repo.find_by_id(&cmd.player_id).await.map_err(IncreaseStatError::Repository)?.ok_or(IncreaseStatError::PlayerNotFound)?;
    let level = player.next_improvement_level();
    let (cost, value_delta) = resolve_stat_cost(catalog, cmd.stat, level);
    let event = player.increase_stat(cmd.stat, cost, value_delta).map_err(IncreaseStatError::Domain)?;
    player_repo.append(&player.id, &player.team_id, &event, player.version).await.map_err(IncreaseStatError::Repository)?;
    Ok(())
}
```

### Handler + route

Même vérification phase + permission que la carte 178, avant appel du use
case.

```
POST /app/{space_id}/players/{player_id}/stats/{stat}
```

---

## Checklist

- [ ] `IncreaseStatCommand` + `increase_stat_use_case::execute`
- [ ] Handler avec vérification phase + permission
- [ ] Route POST câblée
- [ ] Tests : niveau croissant partagé avec les achats de compétences (si un joueur a déjà 2 compétences + 1 stat, le niveau de la 4e amélioration est 4), SPP insuffisant → erreur
