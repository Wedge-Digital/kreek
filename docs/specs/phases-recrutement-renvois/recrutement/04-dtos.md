# Recrutement — Phase 4 : contrats de données

**Entrée** : `03-back.md` validé.

## Un bénéfice du panier-agrégat

Comme le panier **porte** ses données de référence après hydratation, les VMs se
construisent à partir de **types domaine**, jamais de DTOs de port. Tous les
constructeurs sont donc des `from_domain()` co-localisés dans `view_models.rs` :
**aucun `builders.rs` n'est nécessaire** sur cette page.

C'est une conséquence directe du choix de la phase 3, et elle simplifie la couche
présentation d'un fichier entier.

## 1. DTOs d'entrée — corps HTTP

Types primitifs, désérialisés depuis le corps de la requête. Le handler les traduit
en commandes à value objects.

```rust
// io/web/widgets/recruitment_catalog_widget.rs
#[derive(Deserialize)]
pub struct AddPlayerBody {
    pub roster_line_id: String,
    pub version:        i32,
}

#[derive(Deserialize)]
pub struct AddStaffBody {
    pub staff_uid: String,
    pub version:   i32,
}

// io/web/widgets/recruitment_cart_widget.rs
#[derive(Deserialize)]
pub struct RemoveLineBody {
    pub line_id: String,
    pub version: i32,
}

// io/web/validate_phase_actions.rs
#[derive(Deserialize)]
pub struct ValidatePhaseBody {
    pub version: i32,
}
```

| DTO | Émis par | Consommé par |
|---|---|---|
| `AddPlayerBody` | le navigateur (`hx-vals` du bouton « Recruter ») | handler `add_basket_player` |
| `AddStaffBody` | le navigateur (`hx-vals` du bouton « Acheter ») | handler `add_basket_staff` |
| `RemoveLineBody` | le navigateur (`hx-vals` du bouton `×`) | handlers de retrait |
| `ValidatePhaseBody` | le navigateur (bouton de validation) | handler `validate_recruitment_phase` |

## 2. Commandes applicatives

Aucun type primitif nu — smart constructors appelés par le handler.

```rust
// use_cases/commands.rs
pub struct AddBasketPlayerCommand {
    pub team_id:          TeamId,
    pub space_id:         SpaceId,
    pub roster_line_id:   RosterLineId,
    pub expected_version: BasketVersion,
}

pub struct AddBasketStaffCommand {
    pub team_id:          TeamId,
    pub space_id:         SpaceId,
    pub staff_type:       StaffType,
    pub expected_version: BasketVersion,
}

pub struct RemoveBasketLineCommand {
    pub team_id:          TeamId,
    pub space_id:         SpaceId,
    pub line_id:          BasketLineId,
    pub expected_version: BasketVersion,
}

pub struct ValidateRecruitmentPhaseCommand {
    pub team_id:          TeamId,
    pub expected_version: BasketVersion,
}
```

| Commande | Émise par | Consommée par |
|---|---|---|
| `AddBasketPlayerCommand` | handler | `add_basket_player_use_case` |
| `AddBasketStaffCommand` | handler | `add_basket_staff_use_case` |
| `RemoveBasketLineCommand` | handler | `remove_basket_line_use_case` |
| `ValidateRecruitmentPhaseCommand` | handler | `validate_recruitment_phase_use_case` |

## 3. Value objects nouveaux

```rust
// domain/value_objects.rs
pub struct BasketLineId(pub String);            // ULID, généré à l'ajout
pub struct BasketVersion(pub u32);
pub struct Jersey(u8);                         // smart constructor : 1..=16
pub struct RosterLineId(String);               // smart constructor : non vide
```

`Jersey` a son smart constructor parce que le numéro est attribué à l'application du
lot et doit être borné ; `BasketLineId` est un identifiant technique, sans invariant.

## 4. DTOs de port

Vivent dans `ports.rs`, primitives acceptées (convention du projet pour les DTOs de
lecture).

```rust
// teams/ports.rs — vers references
pub struct RosterCatalogDto {
    pub positions:        Vec<CatalogPositionDto>,
    pub cross_limits:     Vec<CrossLimitDto>,
    pub allowed_staff:    Vec<String>,
    pub staff_prices:     Vec<StaffPriceDto>,
    pub reroll_base_cost: u32,
}

pub struct CatalogPositionDto {
    pub uid:           String,
    pub position_name: String,
    pub stats:         String,        // « 7/3/2+/4+/8+ », déjà formaté
    pub skills:        Vec<String>,
    pub cost:          u32,
    pub max_quantity:  u8,
    pub is_journeyman: bool,
}

pub struct CrossLimitDto {
    pub max:           u8,
    pub position_uids: Vec<String>,   // schéma unifié — cf. écart README
}

pub struct StaffPriceDto {
    pub uid:          String,
    pub name:         String,
    pub price:        u32,
    pub max_quantity: u8,
}

// teams/ports.rs — vers players (port étendu, cf. carte 250)
pub struct SquadMemberDto {
    pub player_id:                String,
    pub roster_line_id:           String,
    pub value_kpo:                u32,
    pub available_for_next_match: bool,
}
```

| DTO | Émis par | Consommé par |
|---|---|---|
| `RosterCatalogDto` | `roster_catalog_adapter` (infrastructure) | `basket_hydration_service` **uniquement** |
| `SquadMemberDto` | `squad_adapter` (infrastructure) | `basket_hydration_service` **uniquement** |

**Aucun handler, aucun template ne voit ces types.** Le domain service les convertit
en objets domaine portés par l'agrégat.

## 5. VMs de sortie

### Le VM central : l'état d'un bouton

```rust
// io/web/view_models.rs
pub enum ActionVm {
    Enabled  { label: String },              // « Recruter », « Acheter »
    Blocked  { reason: String },             // « Quota atteint », « Trésorerie », …
    Forbidden{ explanation: String },        // « Ce roster n'a pas droit à un apothicaire. »
}
```

C'est lui qui matérialise la décision de la phase 2 : **la raison du blocage est
calculée côté serveur**, jamais devinée par le client. C'est ce qui permet de n'écrire
chaque règle qu'une fois.

### Catalogue

```rust
pub struct RecruitmentCatalogVm {
    pub context:       ContextVm,
    pub positions:     Vec<PositionRowVm>,
    pub staff:         Vec<StaffRowVm>,
    pub composition:   Vec<CompositionRowVm>,
    pub squad_is_full: bool,
    pub version:       u32,          // cuit dans les hx-vals
}

pub struct ContextVm {
    pub roster_name:     String,
    pub treasury_kpo:    u32,        // trésorerie RÉELLE
    pub squad_count:     u8,         // effectif PROJETÉ (possédé + en attente)
    pub squad_max:       u8,
    pub team_value_kpo:  u32,
}

pub struct PositionRowVm {
    pub line_id:   String,
    pub name:      String,
    pub stats:     String,
    pub skills:    Vec<String>,
    pub owned:     u8,
    pub pending:   u8,               // affiché « +N » en vert
    pub max:       u8,
    pub price_kpo: u32,
    pub action:    ActionVm,
}

pub struct StaffRowVm {
    pub staff_uid:      String,
    pub name:           String,
    pub owned:          u8,
    pub pending:        u8,
    pub max:            u8,
    pub price_kpo:      u32,
    pub base_price_kpo: Option<u32>, // Some pour la relance : « prix de saison — base 50 »
    pub action:         ActionVm,
}

pub struct CompositionRowVm {
    pub name:        String,
    pub owned:       u8,
    pub pending:     u8,
    pub max:         u8,
    pub owned_pct:   u8,
    pub pending_pct: u8,
}
```

### Panier

```rust
pub struct RecruitmentCartVm {
    pub lines:         Vec<CartLineVm>,
    pub remaining_kpo: u32,
    pub is_low:        bool,          // < 50 kPo → couleur de risque
    pub cta_label:     String,        // « Valider 3 achats → »
    pub version:       u32,
}

pub struct CartLineVm {
    pub line_id:   String,
    pub label:     String,
    pub price_kpo: u32,
}
```

### Erreur

```rust
pub struct BasketErrorVm {
    pub kind:    BasketErrorKind,      // Domain | Concurrent
    pub message: String,
    pub lines:   Vec<String>,         // lignes fautives au refus en bloc
}
```

| VM | Émis par | Consommé par |
|---|---|---|
| `RecruitmentCatalogVm` | `RecruitmentCatalogVm::from_domain(&basket)` | template `recruitment-catalog.html` |
| `RecruitmentCartVm` | `RecruitmentCartVm::from_domain(&basket)` | template `recruitment-cart.html` |
| `ActionVm` | calculé par le domaine, exposé par les deux VMs ci-dessus | templates, pour l'état et le libellé du bouton |
| `BasketErrorVm` | handlers, sur erreur domaine ou `ConcurrentWrite` | template `basket-error.html` |

## 6. Règles métier identifiées à cette étape

- **Deux nombres d'argent, deux libellés.** `ContextVm.treasury_kpo` est la trésorerie
  réelle, `RecruitmentCartVm.remaining_kpo` le reste après achats. Jamais un seul mot
  pour les deux.
- **L'effectif du contexte est projeté**, pas possédé : c'est lui qui décide du
  plafond de 16, donc c'est lui qu'on affiche.
- **`base_price_kpo` n'est renseigné que quand le prix affiché diffère du prix de
  base** — c'est-à-dire pour la relance en cours de saison. Ailleurs il vaut `None` et
  le template n'affiche aucune mention.
- **`ActionVm::Forbidden` est distinct de `Blocked`** : un quota atteint peut se
  libérer, un roster sans apothicaire n'y aura jamais droit. Deux états, deux
  formulations.

## 7. Points ouverts pour la phase 5

- `stats: String` déjà formaté dans le DTO de port, ou cinq champs typés que la VM
  assemble ? Le format « 7/3/2+/4+/8+ » est une convention d'affichage, ce qui plaide
  pour que `references` le compose — mais c'est de la présentation dans un port.
