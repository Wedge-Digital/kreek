# `ISquadPort` — le port de consultation de l'effectif

**Priorité : haute**
**Dépend de :** 250 (qui crée `IPlayerValuePort`)
**Bloque :** 262, 267
**Spec :** `docs/specs/phases-recrutement-renvois/renvois/04-dtos.md` §4
**Fichiers :** `src/app/teams/ports.rs`,
`src/infrastructure/teams/player_value_adapter.rs`

## Problème

La carte 250 crée `IPlayerValuePort` pour le calcul de la valeur d'équipe :
`player_id`, `value_kpo`, `available_for_next_match`.

Les deux phases de cette feature l'élargissent **deux fois** :

- **recrutement** : `roster_line_id`, pour compter les effectifs par poste et faire
  respecter les quotas
- **renvois** : `personal_name`, `position_name`, `spp`, pour afficher l'effectif
  nominatif

À ce stade ce n'est plus un port de valeur mais un **port de consultation de
l'effectif**. Le garder sous son nom d'origine condamne à un renommage plus tard, ou
pire, à créer un second port qui doublonne.

## Action

### 1. Renommer

`IPlayerValuePort` → `ISquadPort`, `PlayerValueDto` → `SquadMemberDto`.

Si la carte 250 n'est pas encore réalisée, **poser directement le bon nom** et cette
carte se réduit à l'ajout des champs.

### 2. Le contrat complet

```rust
pub struct SquadMemberDto {
    pub player_id:                String,
    pub roster_line_id:           String,
    pub personal_name:            String,
    pub position_name:            String,
    pub spp:                      u32,
    pub value_kpo:                u32,
    pub available_for_next_match: bool,
}

#[async_trait]
pub trait ISquadPort: Send + Sync {
    async fn find_squad(&self, team_id: &str) -> Vec<SquadMemberDto>;
}
```

### 3. `available_for_next_match` combine deux axes

Après la carte 260, un joueur compte **uniquement s'il est membre actif ET disponible
au prochain match**. L'adapter fait la traduction : `teams` n'importe ni
`RosterMembership` ni `PlayerParticipationStatus`.

C'est le rôle de l'ACL — traduire le vocabulaire de l'autre BC, pas l'importer.

### 4. Un seul port, pas deux

Vérifier qu'aucun autre port de `teams` ne demande déjà des données d'effectif.
`IPlayerCountPort::count_for_team` fait un `SELECT COUNT(*)` direct
(`player_count_adapter.rs:19`) : il devient un cas particulier de `find_squad` et doit
être **supprimé**, pas maintenu en parallèle.

## Checklist

- [ ] `ISquadPort` / `SquadMemberDto` nommés ainsi dès le départ
- [ ] Les sept champs présents
- [ ] `available_for_next_match` = membre actif **et** participation disponible
- [ ] `teams` n'importe aucun type du domaine `players`
- [ ] `IPlayerCountPort` supprimé, ses appelants basculés
- [ ] Le calcul de valeur d'équipe (carte 250) consomme le port renommé
- [ ] `make check-arch` au vert, `make test` au vert
