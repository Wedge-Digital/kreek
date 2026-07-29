# Port catalogue de roster — et les limites croisées enfin appliquées

**Priorité : haute**
**Dépend de :** rien
**Bloque :** 262
**Spec :** `docs/specs/phases-recrutement-renvois/recrutement/03-back.md` §3
**Fichiers :** `assets/references/teams_fr.json`,
`src/app/references/domain/models.rs`, `src/app/teams/ports.rs`,
`src/infrastructure/teams/roster_catalog_adapter.rs` (nouveau),
`src/app/team_creation/use_cases/roster_service.rs`, `src/app/team_creation/ports.rs`

## Problème

### Les limites croisées ne s'appliquent nulle part

Certains rosters limitent le cumul de plusieurs postes — « pas plus de 3 joueurs parmi
Ogre, Troll, Minotaure, Rat Ogre ». **Quatre rosters sur trente** en ont : Renégats du
Chaos, Habitants des Bas-Fonds, Élus du Chaos, Alliance du Vieux Monde.

Les données existent dans `teams_fr.json`. Le modèle domaine existe
(`CrossLimit`, `check_cross_limits` dans `team_creation/domain/team_roster_selected.rs:138`).
Mais `roster_service.rs:68` écrit **`cross_limits: vec![]` en dur**, et ni
`references::TeamDefinition` ni le port de `team_creation` ne portent le champ.

La vérification court-circuite donc toujours, **y compris à la création d'équipe**.
C'est un trou ouvert depuis l'origine.

### Deux schémas JSON incompatibles

```json
"cross_limit": [{"max": 3, "in": ["…OGRE", "…TROLL"]}]                    // 3 rosters
"cross_limit": [{"limit": 1, "limitedPlayerIds": ["…TROLL", "…OGRE"]}]    // Élus du Chaos
```

La struct Rust ne correspond qu'au second. Même correctement câblée, elle échouerait à
désérialiser trois rosters sur quatre.

## Action

### 1. Unifier les données

Retenir un seul schéma dans `teams_fr.json` — `{max, in}` est le plus lisible et
majoritaire. Corriger l'entrée des Élus du Chaos.

### 2. Remonter le champ jusqu'aux consommateurs

`references::TeamDefinition` expose `cross_limit`, puis le port de `team_creation`, et
`roster_service.rs:68` cesse d'écrire un vecteur vide.

**Conséquence immédiate** : les limites croisées s'appliquent enfin à la construction
d'équipe. À vérifier en e2e — une équipe Renégats du Chaos ne doit plus pouvoir
recruter 4 gros joueurs.

### 3. Le port de `teams`

```rust
pub struct RosterCatalogDto {
    pub positions:        Vec<CatalogPositionDto>,   // uid, nom, stats, compétences,
                                                     // prix, max_quantity, is_journeyman
    pub cross_limits:     Vec<CrossLimitDto>,        // { max, position_uids }
    pub allowed_staff:    Vec<String>,
    pub staff_prices:     Vec<StaffPriceDto>,        // uid, nom, prix, max_quantity
    pub reroll_base_cost: u32,
}

pub trait IRosterCatalogPort: Send + Sync {
    fn find_catalog(&self, roster_id: &str) -> Option<RosterCatalogDto>;
}
```

Adapter dans `src/infrastructure/teams/`. **`IRosterInfoPort` devient un
sous-ensemble** : le fusionner plutôt que de laisser deux ports vers la même source.

Le prix de relance exposé est le **prix de base** ; le doublement hors création est une
règle de saison, appliquée par le domaine (carte 262), pas par le catalogue.

### 4. Corriger la résolution du poste de journalier

`teams/infrastructure/journeyman_type_adapter.rs` prend le poste au `max_quantity` le
plus élevé — une heuristique. Les données portent un champ explicite `is_journeyman`,
exactement un par roster, déjà utilisé par `match_report`
(`ref_team_data_adapter.rs:90`). Aligner `teams` dessus.

## Checklist

- [ ] `teams_fr.json` : un seul schéma de limite croisée, les 4 rosters cohérents
- [ ] `cross_limit` exposé par `references::TeamDefinition`
- [ ] `roster_service.rs:68` n'écrit plus `vec![]`
- [ ] Test : une équipe Renégats du Chaos ne peut pas dépasser sa limite croisée **à la création**
- [ ] `IRosterCatalogPort` + adapter, `IRosterInfoPort` fusionné dedans
- [ ] `is_journeyman` remplace l'heuristique `max_quantity` dans `teams`
- [ ] E2E de construction d'équipe toujours vert
- [ ] `make check-arch` au vert, `make test` au vert
