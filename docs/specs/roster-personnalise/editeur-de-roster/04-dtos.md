# Éditeur de roster · Phase 4 : contrats de données

**Phase 3** : `03-back.md`

## Entrée : un seul POST, en JSON

Le formulaire est d'un seul tenant (phase 2) : un roster ne s'enregistre pas par
morceaux. Le corps est du JSON, sérialisé par l'état Alpine.

**JSON et non formulaire** : la structure est imbriquée sur deux niveaux — un
roster, ses postes, et pour chaque poste trois listes. Aucun encodage de
formulaire ne rend ça lisible, et le magicien de compétition a déjà tranché de
la même façon pour ses tiers.

## Le piège : `Team` ne valide rien

On serait tenté de désérialiser directement dans le type du corpus, comme le
POST du barème le fait avec `CompetitionRules`. **Ça ne marche pas ici.**

```rust
// references/domain/models.rs:127
pub struct Team {
    pub uid: String,
    pub name: String,
    pub reroll_cost: u32,
    …
}
```

`CompetitionRules` est fait de **nutypes**, qui valident à la désérialisation.
`Team` est fait de `String` et de `u32` nus : c'est un DTO de lecture du corpus,
écrit pour lire un fichier de confiance. Le désérialiser depuis un navigateur
accepterait un nom vide, un prix de 4 milliards, une caractéristique à 200.

**La commande est le garde, `Team` est le format de stockage.** Deux types, et
la conversion ne se fait que dans un sens : commande validée → `Team` écrit en
base et servi comme n'importe quel roster du corpus.

## La commande

```rust
pub struct CreateCustomRosterCommand {
    pub space_id: SpaceId,
    pub created_by: CoachId,
    pub name: RosterName,
    pub logo: Option<CloudinaryImage>,
    pub tier: TierName,
    pub reroll_cost: RerollBasePrice,
    pub special_rules: Vec<SpecialRuleUid>,
    pub allowed_staff: Vec<StaffUid>,
    pub cross_limits: Vec<CrossLimitCmd>,
    pub positions: Vec<PositionCmd>,
}

pub struct PositionCmd {
    pub name: PlayerName,
    pub cost: PlayerPrice,
    pub stats: StatLine,
    pub max_quantity: PlayerMaxQuantity,
    pub is_journeyman: bool,
    pub skills: Vec<SkillUid>,
    pub primary_access: Vec<SkillCategoryUid>,
    pub secondary_access: Vec<SkillCategoryUid>,
    pub keywords: Vec<KeywordUid>,
}

pub struct CrossLimitCmd {
    pub max: CrossLimitCount,
    pub position_indexes: Vec<u8>,   // l'index dans `positions`, pas un uid
}
```

**`position_indexes` et non des uid** : à la création, les postes n'ont pas
encore d'uid — c'est le use case qui les engendre. Une limite croisée désigne
donc ses postes par leur rang dans la liste soumise, et le use case remplace les
rangs par les uid qu'il vient de créer.

`is_journeyman` reste un `bool` : c'est un drapeau, pas une valeur avec des
invariants, et le `CLAUDE.md` n'interdit la primitive que là où une règle se
perdrait.

## Les value objects — cinq existent, six manquent

### Ce qui existe déjà, avec ses bornes

`team_creation/domain/roster.rs` les porte, et ils ont exactement les bornes
qu'il faut :

| Value object | Bornes |
|---|---|
| `RosterName`, `PlayerName` | `trim`, non vide, ≤ 50 caractères, `TEXTE_SAISI` |
| `PlayerPrice` | 1 à 300 |
| `PlayerMaxQuantity` | 1 à 16 |
| `RerollBasePrice` | 1 à 100 |
| `CrossLimitCount` | 1 à 16 |

### Ce qui manque : les caractéristiques

**Aucune borne n'existe nulle part pour MA, ST, AG, PA et AV.** Le corpus est un
fichier de confiance ; personne n'a jamais tapé une caractéristique. Cet écran
est le premier.

Le LRB les donne — *Livre de Règles Bonifiées Saison 3*, section 5 à 9,
page 25 :

> Les Caractéristiques ont une valeur maximum et minimum, et on ne peut jamais
> les améliorer ni les réduire au-delà ou en deçà des valeurs indiquées
> ci-dessous.

| | M | F | AG | CP | AR |
|---|---|---|---|---|---|
| Maximum | 9 | **8** | 1+ | 1+ | 11+ |
| Minimum | 1 | 1 | 6+ | 6+ | 3+ |

```rust
pub struct StatLine {
    pub ma: Movement,        // 1 à 9
    pub st: Strength,        // 1 à 8
    pub ag: AgilityTarget,   // 1 à 6
    pub pa: PassingTarget,   // 1 à 6
    pub av: ArmourTarget,    // 3 à 11
}
```

**Le piège du tableau : « maximum » veut dire « le meilleur », pas « le plus
grand ».** Pour l'Agilité et la Capacité de Passe, le meilleur est `1+` —
numériquement le plus petit. Pour l'Armure, le meilleur est `11+` —
numériquement le plus grand. Transcrire la colonne « Maximum » en
`less_or_equal` inverserait deux bornes sur cinq.

Les intervalles numériques sont donc ceux du bloc ci-dessus, et il vaut mieux
les écrire ainsi que recopier le tableau.

### `pa` n'est pas optionnel — correction

Une version précédente de cette phase le posait en `Option`, au motif qu'un
poste peut ne pas savoir lancer. **Vérifié : c'est faux pour un poste de
roster.**

| Type | Champ | Peut valoir « – » |
|---|---|---|
| `PlayerPosition` (poste de roster) | `pa: u8` | **non** |
| `StarPlayer` | `pa: String` | **oui** — « Grumo le Terne » porte `"-"` |

Le corpus encode donc la même caractéristique de deux façons selon le type, et
seuls les joueurs vedettes admettent l'absence. Aucun des neuf postes du corpus
de démonstration n'omet sa Capacité de Passe, et le LRB ne prévoit pas de poste
sans elle.

**Conséquence pour la maquette** : le Kroxigor y affiche « — » en Capacité de
Passe. C'est une invention, à corriger.

### Où ces types doivent vivre

Les cinq existants sont dans `team_creation`. **`references` n'a pas le droit de
les importer** — deux BCs, et le `CLAUDE.md` l'interdit.

Deux voies :

| Voie | Ce qu'elle coûte |
|---|---|
| `references` redéclare les siens | deux définitions de « ce qu'est un nom de poste valide », qui dériveront |
| **Les déplacer dans `shared_kernel::bloodbowl::roster`** | un déplacement, `team_creation` suit |

**La seconde, recommandée.** Ces types ne décrivent pas un cas d'usage, ils
décrivent **le jeu** : un prix de poste est un prix de poste, qu'on lise le
corpus ou qu'on écrive un roster. `shared_kernel::bloodbowl::` existe pour ça,
et `TEXTE_SAISI` y est déjà.

Le risque de la duplication est concret : un roster accepté par `references` et
refusé par `team_creation` à la lecture, ou l'inverse — le roster escamoté sans
un mot de la carte 438.

**Le déplacement se fait par copier-coller** (règle 5 du `CLAUDE.md`), pas par
réécriture.

## La conversion vers `Team`

Elle vit dans le use case, et c'est là que les uid naissent :

```rust
uid du roster : CUSTOM_<sulid>
uid d'un poste : <uid du roster>__<SULID>
```

Le second suit la convention du corpus — `DEMO_GRANIT__PIETAILLE` — mais avec un
identifiant engendré plutôt qu'un slug du nom. **Un slug se casse au renommage**
et deux postes homonymes produiraient le même uid.

## Sortie : trois templates

### L'éditeur

```rust
#[derive(Template)]
#[template(path = "references-roster-editor.html")]
pub struct RosterEditorTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub mode: EditorMode,
    pub vm: RosterVm,
    pub catalogs: CatalogsVm,
}

pub enum EditorMode {
    Create,
    Edit,
    /// Le compteur d'équipes accompagne le verrou : l'écran dit la cause,
    /// pas seulement l'interdit.
    ReadOnly { teams_using: u32 },
}
```

`EditorMode` et non deux booléens `can_edit` / `can_delete` : les trois états
sont exclusifs, et deux booléens en autorisent quatre — dont un qui n'existe
pas.

### Les catalogues, rendus une fois

```rust
pub struct CatalogsVm {
    pub skills: Vec<SkillOptionVm>,      // 72
    pub traits: Vec<TraitOptionVm>,      // 74, familles repliées
    pub keywords: Vec<KeywordOptionVm>,  // 38
    pub categories: Vec<LabelVm>,        // 6
    pub staff: Vec<LabelVm>,
    pub special_rules: Vec<LabelVm>,
}

pub struct SkillOptionVm {
    pub uid: String,
    pub name: String,
    pub category: String,      // le libellé, pas l'uid
    pub is_elite: bool,        // 4 sur 72 — et 10 kPo de plus à l'achat en SPP
    pub description: String,
}

pub struct KeywordOptionVm {
    pub uid: String,
    pub label: String,
    pub is_species: bool,      // dérivé de league_hate_selectable
    pub hate_note: String,     // ce que la ligne dit sous le nom
}
```

**`is_species` est dérivé, pas lu tel quel.** Le corpus porte
`league_hate_selectable`, qui répond à « une Haine de ligue peut-elle le
viser ? ». Les deux coïncident pour 37 mots-clefs sur 38 — **`BIG_GUY` est
l'exception** : `league_hate_selectable: false`, et pourtant un
`hate_skill_uid` non nul. C'est un rôle, et `hate_note` doit le dire au lieu de
laisser croire à une incohérence.

**`traits` a les familles repliées** — `HATRED_*` compte 31 entrées, 42 % des
traits. Le repliage se fait au **builder**, pas au gabarit :

```rust
pub struct TraitOptionVm {
    pub uid: Option<String>,     // None pour une famille : le choix produira l'uid
    pub name: String,
    pub family: Option<TraitFamily>,
    pub description: String,
}
pub enum TraitFamily { Hatred, Animosity, Loner, Bloodlust }
```

**Le gabarit nu du corpus est écarté.** `LONER` sans nombre existe à côté de
`LONER_3` et `LONER_4` : c'est un modèle de rédaction, pas un trait
attribuable. Le builder le reconnaît à sa liste de familles — quatre, tenues
dans le code — et ne rend que la famille.

### La liste

```rust
pub struct RosterListTemplate {
    …,
    pub own: Vec<RosterRowVm>,      // les rosters de l'espace
    pub corpus: Vec<RosterRowVm>,   // ceux du règlement
    pub can_manage: bool,           // admin d'espace
}

pub struct RosterRowVm {
    pub uid: String,
    pub name: String,
    pub initials: String,
    pub tier: String,
    pub position_count: u32,
    pub reroll_cost: u32,
    /// `None` pour un roster du règlement : le compteur n'y décide de rien et
    /// coûterait une lecture globale (phase 2).
    pub teams_using: Option<u32>,
    pub created_at: Option<String>,
}
```

`teams_using: Option<u32>` porte la distinction des deux sections dans le type.
Un `u32` avec zéro par convention laisserait le gabarit deviner s'il s'agit
d'« aucune équipe » ou de « on ne compte pas ».

## Les DTOs de port

```rust
// references/ports.rs — le fichier n'existe pas encore
#[async_trait]
pub trait IRosterUsagePort: Send + Sync {
    async fn count_teams_using(&self, roster_uid: &str) -> Result<u32, String>;
}

#[async_trait]
pub trait IReferencesSpaceAdminPort: Send + Sync {
    async fn is_space_admin(&self, user_id: &str, space_id: &str) -> bool;
}
```

Le second a un précédent mot pour mot dans `match_report::ISpaceAdminPort`. On
ne le partage pas : chaque BC déclare le besoin qu'il a, c'est ce qui les garde
séparables.

## Règles métier à préciser

1. **Tranché — au moins une catégorie d'accès primaire est obligatoire.** Un
   poste sans accès ne progresserait jamais par compétence. Le contrôle rejoint
   ceux du pied de cohérence (phase 2) et le refus du serveur.

   L'accès **secondaire**, lui, peut être vide : le corpus en compte, et un
   poste qui ne progresse que dans sa catégorie principale est un poste
   ordinaire.

2. **Le tier d'un roster est-il libre ?** La maquette propose Tier 1 à 3 ; le
   corpus écrit `"tier": "Tier 1"` en texte. Une liste fermée ou un champ libre
   changent le contrat.
