# Phase 4 — Contrats de données — player-detail

## Encodage des corps de requête

**`Form` (urlencoded)** pour les sept `POST`.

`players` n'a pas de convention unique aujourd'hui : `post_purchase_skill`
prend un `Json`, l'augmentation de caractéristique ne prend aucun corps. Le
projet charge bien l'extension `json-enc` dans le layout, mais elle doit être
activée élément par élément.

Les charges utiles ici sont des scalaires plats. `Form` est natif à HTMX, ne
dépend d'aucune extension, et c'est ce qu'utilisent déjà les endpoints de
panier de `teams` — le patron dont cette fonctionnalité s'inspire.

---

## DTOs d'entrée

Primitives assumées : ce sont des DTOs de **transport**, validés par les smart
constructors au moment de bâtir la commande. Un formulaire mal formé est un
`400`, pas une erreur métier.

```rust
// customisation_controller.rs
#[derive(Deserialize)] pub struct AddSkillForm    { pub skill_id: String }
#[derive(Deserialize)] pub struct AddStatForm     { pub stat: String, pub crans: i8 }
#[derive(Deserialize)] pub struct AdjustPriceForm { pub delta_kpo: i32 }
#[derive(Deserialize)] pub struct AddSppForm      { pub amount: i32 }
#[derive(Deserialize)] pub struct RemoveLineForm  { pub line_id: String }
```

`validate` et `cancel` n'ont pas de corps — le `player_id` du chemin suffit.

**`crans` porte le sens en qualité du joueur** (`+1` améliore, `-1` dégrade),
jamais l'offset brut. La traduction vers la valeur stockée dépend de la
caractéristique et appartient au domaine, seul détenteur de la table de
directions. Un front qui enverrait `-1` pour améliorer l'agilité dupliquerait
la convention et la ferait diverger au premier oubli.

| DTO | Émis par | Consommé par |
|---|---|---|
| `AddSkillForm` | bouton « + Ajouter » du panneau (`hx-post`) | `customisation_controller::post_add_skill` |
| `AddStatForm` | boutons « Améliorer » / « Dégrader » | `post_add_stat` |
| `AdjustPriceForm` | bouton « Ajuster » | `post_adjust_price` |
| `AddSppForm` | bouton « Ajouter » de l'onglet SPP | `post_add_spp` |
| `RemoveLineForm` | bouton « Annuler » d'une ligne du panier | `post_remove_line` |

---

## Commandes

Value objects obligatoires — ce sont des commandes (`CLAUDE.md`, interdiction
des primitives nues côté écriture).

```rust
// use_cases/commands.rs
pub struct AddCustomisationSkillCommand  { pub player_id: PlayerId, pub skill_id: SkillId }
pub struct AddCustomisationStatCommand   { pub player_id: PlayerId, pub stat: StatKind, pub crans: StatCrans }
pub struct AdjustCustomisationPriceCommand { pub player_id: PlayerId, pub delta: KpoDelta }
pub struct AddCustomisationSppCommand    { pub player_id: PlayerId, pub amount: SppAmount }
pub struct RemoveCustomisationLineCommand { pub player_id: PlayerId, pub line_id: BasketLineId }
pub struct ValidateCustomisationCommand  { pub player_id: PlayerId, pub author: CoachName }
pub struct CancelCustomisationCommand    { pub player_id: PlayerId }
```

Nouveaux value objects, dans `domain/value_objects.rs` :

| VO | Contrainte |
|---|---|
| `StatCrans(i8)` | non nul ; l'amplitude est bornée par les bornes de caractéristique, pas par le VO |
| `KpoDelta(i32)` | non nul ; signé — le plancher à 0 porte sur le **résultat**, pas sur le delta |
| `SppAmount(u8)` | `1..=100` — le plafond de la phase 1 est par opération, il tient donc dans le VO |
| `BasketLineId(String)` | non vide |

`CoachName` existe déjà (`auth`). Il est porté par la commande de validation
parce que le journal doit nommer le commissaire — et c'est **le validateur**,
la phase 2 ayant écarté la traçabilité par ligne.

| Commande | Émise par | Consommée par |
|---|---|---|
| `Add*` / `Remove*` | `customisation_controller` | `customisation_basket_mutation` |
| `ValidateCustomisationCommand` | `customisation_controller` | `validate_customisation_use_case` |
| `CancelCustomisationCommand` | `customisation_controller` | `validate_customisation_use_case` (branche annulation) |

---

## DTOs de port

### Panier persisté

```rust
// ports.rs — calqué sur teams::PhaseBasketState
pub struct CustomisationBasketState {
    pub player_id: String,
    pub space_id:  String,
    pub state:     serde_json::Value,  // les lignes, rien d'autre
    pub version:   u32,
}
```

`state` ne porte **que les lignes**. Joueur, catalogue et caractéristiques de
base sont rechargés à chaque hydratation — c'est ce qui fait qu'un panier d'une
heure est jugé contre le joueur d'aujourd'hui.

| DTO | Émis par | Consommé par |
|---|---|---|
| `CustomisationBasketState` | `customisation_basket_repository` | `customisation_basket_hydration_service` |

### Catalogue de compétences

`ISkillCatalogPort` gagne un listing. Le DTO existe déjà :

```rust
fn list_all_skills(&self) -> Vec<SkillCatalogEntryDto>;
```

Non filtré par l'accès du poste, contrairement au `skill-picker` de
`references` : la customisation ignore les règles du jeu par définition.

| DTO | Émis par | Consommé par |
|---|---|---|
| `SkillCatalogEntryDto` | `skill_catalog_adapter` | `customisation_basket_hydration_service`, jamais un handler |

---

## DTOs de sortie — le panneau

Un seul template. Le panier vivant côté serveur, **tout ce que la maquette
calculait en JS devient une donnée de VM** : valeurs effectives, aperçus,
lignes en attente, disponibilité des boutons.

```rust
// widgets/player_customisation_widget.rs
#[derive(Template)]
#[template(path = "widgets/player-customisation-widget.html")]
pub struct PlayerCustomisationTemplate {
    pub app_routes:  AppRoutes,
    pub space_id:    String,
    pub player_id:   String,
    pub vm:          CustomisationVm,
}

pub struct CustomisationVm {
    pub player_name: String,
    pub spp_reserve: u32,
    pub skills:      Vec<AddableSkillVm>,
    pub stats:       Vec<StatCardVm>,
    pub price_kpo:   u32,        // effectif, panier compris
    pub spp_earned:  u32,        // effectif, panier compris
    pub pending:     Vec<PendingLineVm>,
    /// Motif du dernier refus. Rendu à côté de l'action refusée, pas en tête
    /// de panneau — le refus tombe au clic, il doit se lire là où on a cliqué.
    pub refusal:     Option<RefusalVm>,
}

pub struct AddableSkillVm {
    pub skill_id:       String,
    pub name:           String,
    pub description:    String,
    pub category_css:   String,
    pub category_label: String,
}

pub struct StatCardVm {
    pub key:            String,   // ma, st, ag, pa, av
    pub label:          String,   // MV, FO, AG, PA, AR
    pub name:           String,   // Mouvement, Force, Agilité, Passe, Armure
    pub current:        String,   // effectif, déjà formaté (« 3+ », « 7 »)
    pub pending_offset: Option<i8>,
    pub preview_up:     Option<String>,
    pub preview_down:   Option<String>,
    pub can_improve:    bool,     // faux si la borne est atteinte
    pub can_degrade:    bool,
}

pub struct PendingLineVm {
    pub line_id: String,
    pub label:   String,   // « Amélioration d'Agilité -1 »
    pub family:  String,   // Compétence, Caractéristique, Prix, SPP
}

pub struct RefusalVm {
    pub message: String,
    /// Ce que le refus visait — permet au template de l'afficher au bon endroit.
    pub target:  RefusalTarget,   // Skill(id) | Stat(key) | Price | Spp
}
```

**`current`, `preview_*` et `label` sont des chaînes déjà formatées.** Le
suffixe `+` des seuils de dé et le sens de l'offset dépendent de la
caractéristique ; les résoudre dans le template reviendrait à y remettre la
table de directions du domaine. Le VM livre du texte, le template affiche.

**`can_improve` / `can_degrade` portent les bornes.** Un bouton grisé vaut
mieux qu'un refus après clic — le refus reste néanmoins nécessaire, un panier
concurrent pouvant avoir consommé la marge entre l'affichage et le clic.

| VM | Produit par | Consommé par |
|---|---|---|
| `CustomisationVm` et ses enfants | `player_customisation_widget::build_vm`, depuis l'agrégat hydraté | `player-customisation-widget.html` |
| `PlayerCustomisationTemplate` | le widget `GET` **et** les sept `POST` | HTMX, en swap `outerHTML` sur `#pd-right-panel` |

Les sept `POST` rendent **le même template** que le `GET` : c'est ce qui fait
qu'un refus s'affiche sans quitter le mode, et que le panier se met à jour sans
événement DOM.

---

## Ce que la fiche joueur doit gagner

`player_detail_controller` choisit l'occupant du slot. Son VM existant gagne
donc de quoi le décider :

```rust
pub struct PlayerDetailVm {
    // … existant …
    pub can_customise: bool,          // resserré : plus le coach de l'équipe
    pub right_panel_widget_url: String, // pointe la customisation si panier + droit
}
```

`can_customise` existe déjà mais s'appuie sur `check_admin_rights`, qui inclut
le coach. La phase 1 l'exclut : **la valeur change, pas le type**.

---

## Règles métier (identifiées phase 4)

- **Les bornes sont rendues, pas seulement vérifiées.** `can_improve` /
  `can_degrade` désactivent le bouton avant le clic ; le refus serveur reste la
  vérité.
- **Le refus s'affiche là où l'on a cliqué**, d'où `RefusalTarget`. Un bandeau
  en tête de panneau obligerait à deviner quelle action a échoué.
- **Le validateur est nommé dans la commande**, pas déduit côté domaine : le
  domaine ne connaît pas la session.

## Points ouverts

- Inchangés depuis la phase 3 : durée de vie d'un panier abandonné, et sort
  d'un panier visant un joueur renvoyé entre-temps.
