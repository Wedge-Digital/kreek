# TV — ports, domain service et fonction de calcul

**Priorité : haute**
**Dépend de :** 249 (l'unité doit être juste avant de sommer des valeurs)
**Bloque :** 251, 252
**Fichiers :** `src/app/teams/ports.rs`,
`src/app/teams/domain/team_value.rs` (nouveau),
`src/app/teams/use_cases/team_value_service.rs` (nouveau),
`src/infrastructure/teams/player_value_adapter.rs` (nouveau),
`src/infrastructure/teams/roster_info_adapter.rs`,
`src/infrastructure/teams/journeyman_type_adapter.rs`,
`src/app/teams/context.rs`, `src/main.rs`

## Problème

La TV doit devenir une **somme calculée**, pas une accumulation de deltas. Le
calcul est :

```
TV =   somme des valeurs des joueurs qui participent au prochain match
     + (11 − nombre de joueurs disponibles) × prix de la ligne journalier
     + apothicaires × 50 + assistants × 10 + pom-pom girls × 10
     + relances × prix de base du roster
```

Trois règles à respecter, toutes issues du grill :

- Un joueur `MissingNextGame`, `Retired` ou `Dead` **vaut zéro**.
- Le **Facteur Fans et la trésorerie n'entrent pas** dans la TV. C'est un
  changement de comportement : `apply(StaffBought)` fait aujourd'hui
  `team_value += cost_kpo` **quel que soit le type de staff**, Facteur Fans
  compris — les TV actuelles sont donc surévaluées.
- Les relances comptent à leur **prix de base**, pas au prix payé. L'incrémental
  ajoutait le montant déboursé ; l'écart apparaîtra le jour où l'achat de
  relance en cours de saison coûtera double.

`teams` ne dispose aujourd'hui d'aucune de ces données : la disponibilité et la
valeur des joueurs vivent dans `players`, les prix du staff et de la ligne
journalier dans `references`.

## Action

### 1. Port vers `players` — vocabulaire `teams`

```rust
pub struct PlayerValueDto {
    pub player_id:                String,
    pub value_kpo:                u32,
    pub available_for_next_match: bool,
}

#[async_trait]
pub trait IPlayerValuePort: Send + Sync {
    async fn find_valued_players(&self, team_id: &str) -> Vec<PlayerValueDto>;
}
```

L'adapter (`src/infrastructure/teams/player_value_adapter.rs`) traduit
`participation_status` en booléen : `Available` → `true`, tout le reste → `false`.

**`teams` n'importe jamais `PlayerParticipationStatus`** — c'est le rôle de l'ACL
de traduire le vocabulaire de l'autre BC. La règle « zéro si indisponible »
reste, elle, une règle de `teams`, et vit dans son domaine.

Suivre le modèle de `player_count_adapter.rs`, déjà en place.

### 2. Étendre les ports vers `references`

`IRosterInfoPort` fournit déjà `reroll_cost`. Ajouter les prix du staff, lus dans
`staff_fr.json` (Apothicaire 50, Pom-pom girls 10, Assistants 10 — tous en kPo) :

```rust
pub struct RosterInfoDto {
    pub logo:               Option<String>,
    pub reroll_cost:        u32,
    pub apothecary_price:   u32,
    pub assistant_price:    u32,
    pub cheerleader_price:  u32,
}
```

`IJourneymanTypePort` ne renvoie aujourd'hui qu'un nom d'affichage. Il doit aussi
donner le **prix** de la ligne de roster correspondante, puisqu'un journalier
vaut ce prix.

**Au passage, corriger la règle de résolution.** L'adapter actuel
(`journeyman_type_adapter.rs`) prend le poste au `max_quantity` le plus élevé —
une heuristique. Or les données de référence portent un champ explicite
`is_journeyman`, exactement un par roster (`HUMAN__HUMAN_LINEMAN` pour les
Humains), et c'est ce champ qu'utilise déjà `match_report`
(`ref_team_data_adapter.rs:90`). Les deux règles donnent la même réponse
aujourd'hui, par coïncidence. Aligner `teams` sur `is_journeyman`.

### 3. Fonction pure du domaine

`src/app/teams/domain/team_value.rs` — aucune dépendance framework, aucun accès
port, aucun `async` :

```rust
pub struct ValuedPlayer {
    pub value_kpo:                Kpo,
    pub available_for_next_match: bool,
}

pub struct TeamValueInputs {
    pub players:            Vec<ValuedPlayer>,
    pub rerolls:            RerollCount,
    pub reroll_price:       Kpo,
    pub apothecaries:       ApothecaryCount,
    pub apothecary_price:   Kpo,
    pub assistants:         AssistantCount,
    pub assistant_price:    Kpo,
    pub cheerleaders:       CheerleaderCount,
    pub cheerleader_price:  Kpo,
    pub journeyman_price:   Kpo,
}

pub fn compute_team_value(inputs: &TeamValueInputs) -> Kpo
```

Le nombre de journaliers est `11.saturating_sub(nombre de disponibles)` — la
même règle que `init_temp_players_use_case::collect_journeymen`.

Découper en fonctions nommées (règle des 20 lignes) : `players_value`,
`journeymen_value`, `staff_value`, `rerolls_value`.

### 4. Domain service

`src/app/teams/use_cases/team_value_service.rs` — charge les DTOs des deux ports
et construit `TeamValueInputs` à partir de l'agrégat `Team` (qui porte déjà
`rerolls`, `apothecaries`, `assistants`, `cheerleaders`).

Aucun handler, aucun template ne doit voir un DTO de port.

### 5. Câblage

`TeamsContext` gagne `player_value_port: Arc<dyn IPlayerValuePort>`, instancié
dans `main.rs`.

## Fin de carte

Le calcul existe et est couvert par ses tests unitaires, **mais rien ne l'appelle
encore** : `team_value` continue d'être maintenue par l'incrémental. C'est la
carte 251 qui bascule.

## Tests unitaires attendus

- Effectif complet, aucun indisponible → somme simple
- Un joueur `MissingNextGame` → sa valeur exclue **et** un journalier ajouté
- Un joueur `Retired` → même traitement
- Effectif de 9 disponibles → 2 journaliers valorisés
- Effectif de 14 disponibles → aucun journalier
- Le Facteur Fans n'entre pas dans le total
- Relances et staff comptés au prix de base

## Checklist

- [ ] `IPlayerValuePort` + `PlayerValueDto` définis dans `teams/ports.rs`
- [ ] `player_value_adapter.rs` traduit `participation_status` en booléen
- [ ] `teams` n'importe aucun type du domaine `players`
- [ ] `RosterInfoDto` étendu aux trois prix de staff
- [ ] `IJourneymanTypePort` renvoie aussi le prix de la ligne
- [ ] `compute_team_value` pure, sans `async`, sans port, découpée en < 20 lignes
- [ ] Domain service dans `use_cases/`, aucun DTO de port hors de lui
- [ ] Les 7 tests unitaires ci-dessus passent
- [ ] `make check-arch` au vert, `make test` au vert
