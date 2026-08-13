# Phase 6 — Domaine — player-detail

## La table des directions doit descendre dans le domaine

Point relevé en préparant cette phase, et qui conditionne le reste.

`apply_increase()` — qui sait qu'améliorer l'agilité **descend** le seuil de dé
et qu'améliorer l'armure le monte — vit aujourd'hui dans
`use_cases/player_stats_service.rs`, c'est-à-dire dans la **couche
applicative**. Elle y est parce que la résolution des caractéristiques a besoin
du catalogue, donc d'un port.

Or l'agrégat panier doit connaître cette table : sans elle, il ne peut pas
juger si une amélioration d'agilité franchit la borne `1+`. Et la phase 1 a
posé qu'il ne doit **pas** exister deux conventions.

**Décision : la direction descend dans le domaine**, sous forme d'un
comportement porté par `StatKind` :

```rust
// domain/match_impact.rs — auprès de StatKind
impl StatKind {
    /// Ce qu'un cran d'**amélioration** fait à la valeur brute.
    /// MV/FO/AR montent ; AG/PA sont des seuils de dé, ils descendent.
    pub fn improvement_step(self) -> i8 { … }
    pub fn bounds(self) -> (u8, u8) { … }
}
```

`player_stats_service` s'en sert au lieu de la porter. La séparation reste
juste : le **service** compose base + ajustements en interrogeant un port, le
**domaine** dit dans quel sens et entre quelles bornes. Le premier orchestre,
le second sait.

C'est le seul moyen de tenir la règle de la phase 1 autrement que par
discipline.

---

## L'agrégat `CustomisationBasket`

Calqué sur `RecruitmentBasket`, dont il partage la nature : des invariants
forts, une hydratation qui lui apporte ce dont ses gardes ont besoin, et
ensuite du pur synchrone — aucun `async`, aucun port.

```rust
// domain/customisation_basket.rs

pub enum CustomisationLine {
    Skill { id: BasketLineId, skill_id: SkillId },
    Stat  { id: BasketLineId, stat: StatKind, crans: StatCrans },
    Price { id: BasketLineId, delta: KpoDelta },
    Spp   { id: BasketLineId, amount: SppAmount },
}

pub struct CustomisationBasket {
    player_id: PlayerId,
    version:   BasketVersion,
    lines:     Vec<CustomisationLine>,

    // Hydratés, jamais persistés — rechargés à chaque affichage.
    base_stats:      ResolvedStats,   // caractéristiques **résolues** du joueur
    owned_skills:    Vec<SkillId>,    // base + acquises + customisées
    catalog_skills:  Vec<SkillId>,    // le catalogue complet, pour valider l'existence
    current_value:   Kpo,
    current_spp:     Spp,
}
```

`base_stats` porte les caractéristiques **déjà résolues** — base du poste,
séquelles et augmentations SPP comprises. L'agrégat n'a donc pas à connaître le
catalogue de postes ; il reçoit un point de départ et raisonne en deltas.

### Méthodes

```rust
pub fn hydrate(…) -> Self;

pub fn add_skill(&mut self, skill: SkillId)   -> Result<BasketLineId, DomainError>;
pub fn add_stat(&mut self, stat: StatKind, crans: StatCrans) -> Result<BasketLineId, DomainError>;
pub fn adjust_price(&mut self, delta: KpoDelta) -> Result<BasketLineId, DomainError>;
pub fn add_spp(&mut self, amount: SppAmount)  -> Result<BasketLineId, DomainError>;
pub fn remove_line(&mut self, id: &BasketLineId) -> Result<(), DomainError>;

/// Rejoue toutes les lignes sur un panier vidé : chacune est jugée contre
/// l'état accumulé des précédentes. Tout ou rien (phase 5).
pub fn validate_all(&self) -> Result<Vec<AppliedCustomisation>, Vec<RejectedLine>>;

// Lecture, pour les view models
pub fn lines(&self) -> &[CustomisationLine];
pub fn effective_stat(&self, stat: StatKind) -> u8;
pub fn effective_value(&self) -> Kpo;
pub fn effective_spp(&self) -> Spp;
pub fn action_for_stat(&self, stat: StatKind, sens: i8) -> ActionState;
pub fn action_for_skill(&self, skill: &SkillId) -> ActionState;
```

`validate_all` reprend la mécanique de `RecruitmentBasket` : cloner, vider les
lignes, les rejouer une à une. C'est ce qui fait qu'une ligne est validée
contre les précédentes et non contre l'état initial — deux améliorations
d'agilité depuis `2+` ne peuvent pas passer si la seconde franchit `1+`.

`action_for_*` rend l'`ActionState` du projet — `Allowed`, `Blocked { cause }`,
`Forbidden { cause }` — qui alimente `can_improve` / `can_degrade` et le grisage
des boutons de la phase 4.

La distinction porte du sens, telle que `teams` l'emploie déjà :
**`Forbidden`** est structurel et ne changera pas en vidant le panier — une
compétence déjà possédée de base. **`Blocked`** est conjoncturel — une borne
atteinte à cause des lignes en attente, qui se libère si on en retire une.

### Les gardes, une par règle de la phase 1

| Garde | Règle |
|---|---|
| `add_skill` | compétence inconnue du catalogue → `UnknownSkill` ; déjà possédée (base, SPP, ou customisation en attente) → `SkillAlreadyAcquired` |
| `add_stat` | résultat hors bornes → `StatOutOfBounds { stat, bound }` |
| `adjust_price` | résultat < 0 → `NegativePlayerValue` |
| `add_spp` | `SppAmount` borne déjà `1..=100` au niveau du VO — la garde ne rejoue pas la contrainte |
| `remove_line` | ligne absente → `BasketLineNotFound` |

`SkillAlreadyAcquired` **existe déjà** dans `DomainError` (achat en SPP). Elle
est réutilisée : c'est le même fait métier, quelle que soit l'origine.

---

## Les quatre événements

```rust
// domain/events.rs — nouvelles variantes de PlayerDomainEvent
PlayerSkillCustomised { player_id, team_id, customisation_id, skill_id, author },
PlayerStatCustomised  { player_id, team_id, customisation_id, stat, offset, author },
PlayerValueCustomised { player_id, team_id, customisation_id, delta, author },
PlayerSppCustomised   { player_id, team_id, customisation_id, amount, author },
```

**`customisation_id`** porte l'identifiant unique de la phase 1. Il n'est pas
l'identifiant de ligne du panier : celui-ci meurt avec le panier, celui-là vit
dans l'event store.

**`offset` est brut**, pas en crans. Le domaine a traduit — c'est lui qui
détient la table de directions, et l'événement enregistre ce qui a été
réellement appliqué. Un rejeu ne doit pas dépendre d'une convention externe.

**`author`** est le nom du commissaire. La phase 2 ayant écarté la traçabilité
par ligne, c'est le validateur, et il est le même pour tout un lot.

### `apply()`

Quatre branches, toutes de la même forme : appliquer le delta, incrémenter la
version. Aucune ne touche `player.value` **sauf** `PlayerValueCustomised` — la
phase 1 pose que ni compétence ni caractéristique customisée ne déplacent la
valeur d'équipe, contrairement à leurs équivalents payés en SPP.

C'est l'asymétrie assumée du README, et c'est ici qu'elle se matérialise : les
événements de customisation de compétence et de caractéristique **ne portent
pas de `value_delta`**. Non pas qu'il vaille zéro : il n'existe pas.

### Méthodes de commande sur `Player`

```rust
pub fn customise_skill(&self, …) -> Result<PlayerDomainEvent, DomainError>;
pub fn customise_stat(&self, …)  -> Result<PlayerDomainEvent, DomainError>;
pub fn customise_value(&self, …) -> Result<PlayerDomainEvent, DomainError>;
pub fn customise_spp(&self, …)   -> Result<PlayerDomainEvent, DomainError>;
```

Elles ne mutent pas `self` — elles calculent et rendent l'événement, comme
`increase_stat` et `rename`.

**Aucune garde de phase, aucune garde d'appartenance.** La phase 1 pose que les
customisations s'appliquent **toujours**, indépendamment de tout le reste. Un
joueur renvoyé reste customisable — c'est délibéré, et c'est ce qui distingue
ces méthodes de `rename`, qui exige `membership == Active`.

Les invariants de validité, eux, ont déjà été joués par l'agrégat panier. Ces
méthodes n'ont donc rien à revérifier : le panier est le gardien, `Player` est
le greffier.

---

## Nouveaux `DomainError`

```rust
UnknownSkill,
StatOutOfBounds { stat: StatKind, bound: u8 },
NegativePlayerValue,
BasketLineNotFound,
```

`SkillAlreadyAcquired` est réutilisée.

---

## Règles métier (identifiées phase 6)

- **La direction et les bornes sont du domaine**, portées par `StatKind`.
  `player_stats_service` les consomme au lieu de les détenir.
- **Le panier est le gardien, `Player` est le greffier.** Les méthodes de
  customisation sur `Player` ne revérifient pas les invariants du panier.
- **Aucune garde de phase ni d'appartenance** sur les méthodes de
  customisation — un joueur renvoyé reste customisable.
- **Les événements de compétence et de caractéristique ne portent pas de
  `value_delta`** : il n'existe pas, il ne vaut pas zéro.

## Points ouverts

- Durée de vie d'un panier abandonné, et sort d'un panier visant un joueur
  renvoyé entre-temps (hérités des phases 2 et 3). La règle « un renvoyé reste
  customisable » ci-dessus **répond en partie au second** : le panier reste
  applicable. Reste à dire si l'interface doit le signaler.
