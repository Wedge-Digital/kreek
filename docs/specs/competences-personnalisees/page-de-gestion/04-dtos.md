# Page de gestion · Phase 4 : contrats de données

**Phase 3** : `03-back.md`

## Une seconde correction de comptage

La phase 3 donnait trois appelants à `list_skills()`. Le troisième —
`team_created_listener.rs:288` — est **dans un `#[cfg(test)]`**.

En production, `list_skills()` a **deux** appelants : `skill_picker.rs:158` et
`skill_catalog_adapter.rs:52`. La table de la phase 3 le laissait « à
vérifier » ; c'est vérifié, et la réponse allège le chantier.

Le test, lui, mérite mieux qu'un rejet — voir « La pastille » plus bas.

## Entrée : un formulaire à plat, et non du JSON

Six champs, aucun imbriqué. C'est de l'`application/x-www-form-urlencoded`.

**Le contraste avec l'éditeur de roster est instructif** : là-bas, la phase 4 a
choisi le JSON parce que la structure descendait sur deux niveaux — un roster,
ses postes, et trois listes par poste. Ici il n'y a rien à imbriquer, et le
formulaire natif dispense d'un état Alpine à sérialiser.

```rust
// Création — POST /app/{space_id}/admin/skills
#[derive(Deserialize)]
pub struct CreateCustomSkillDto {
    pub name:        String,
    pub category:    String,
    pub skill_type:  String,
    pub activation:  String,
    pub description: String,
}
```

```rust
// Modification — PUT /app/{space_id}/admin/skills/{skill_id}
#[derive(Deserialize)]
pub struct UpdateCustomSkillDto {
    pub name:        String,
    pub description: String,
    pub activation:  String,
    /// Absents quand la compétence est employée : l'écran les rend en texte,
    /// pas en champ, donc le navigateur n'envoie rien (phase 2).
    pub category:    Option<String>,
    pub skill_type:  Option<String>,
}
```

### Pourquoi `Option` et non deux points d'entrée

On pourrait router le cas verrouillé vers un second endpoint qui n'accepterait
que le libellé. **Ce serait faire dépendre le verrou de la route empruntée.**

Un `Option` absent veut dire « inchangé » ; un `Option` **présent et différent**
de la valeur en base, sur une compétence employée, est refusé par le use case
(phase 3). Le formulaire qui omet le champ est une commodité d'écran ; le
contrôle serveur est le garde. Les deux disent la même chose, mais **seul le
second tient face à un POST écrit à la main.**

## Les value objects

### Ce qui manque

```rust
// references/domain/value_objects.rs — le fichier n'existe pas encore

#[nutype(sanitize(trim), validate(not_empty, len_char_max = 50, regex = TEXTE_SAISI), …)]
pub struct SkillName(String);

#[nutype(sanitize(trim), validate(not_empty, len_char_max = 600, regex = TEXTE_SAISI), …)]
pub struct SkillDescription(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillType { Standard, Elite }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillActivation { Active, Passive }
```

`600` est la borne de la phase 2, et `TEXTE_SAISI` est la constante unique du
`shared_kernel::identity::charset` — **aucun charset local**, c'est la règle
posée par le commit `542bdfd`.

### Le piège qui coûte 10 kPo — `"Élite"` s'écrit avec son accent

Le corpus stocke le type en texte, et **un seul site décide de l'élitisme dans
toute l'application** :

```rust
// infrastructure/players/skill_catalog_adapter.rs:39
is_elite: skill.skill_type == "Élite",
```

Une comparaison de chaînes, exacte, accent compris. Un `SkillType::Elite`
sérialisé en `"Elite"` produirait donc des compétences Élite **que tout le
système traiterait comme Standard** — 10 kPo de moins à l'achat en SPP, un
barème de coût faux, et pas la moindre erreur.

```rust
#[serde(rename = "Élite")]
Elite,
```

C'est le même genre de silence que le `MUTATION` au singulier documenté plus
bas : **rien ne casse, la valeur ment.** Un test doit fixer la sérialisation,
comme `roster_tier_se_serialise_comme_le_corpus` le fait pour le tier.

### L'identifiant porte le préfixe dans son type

```rust
#[nutype(validate(predicate = |s| s.starts_with("CUSTOM_")), …)]
pub struct CustomSkillUid(String);
```

**Le préfixe est un invariant, pas une convention à vérifier.** Les commandes de
modification et de suppression n'acceptent que ce type : « on ne modifie pas une
compétence du règlement » devient alors une impossibilité de construction, et
non un contrôle qu'un troisième use case oublierait un jour.

### Ce qui est réutilisé de l'épic E10

`SkillCategoryUid` et `SkillUid` sont posés par l'éditeur de roster
(`docs/specs/roster-personnalise/editeur-de-roster/04-dtos.md`). **Même BC, même
fichier** — il n'y a rien à déplacer, seulement à ne pas redéclarer.

## Les commandes

```rust
pub struct CreateCustomSkillCommand {
    pub space_id:    SpaceId,
    pub created_by:  CoachId,
    pub name:        SkillName,
    pub category:    SkillCategoryUid,
    pub skill_type:  SkillType,
    pub activation:  SkillActivation,
    pub description: SkillDescription,
}

pub struct UpdateCustomSkillCommand {
    pub uid:         CustomSkillUid,
    pub space_id:    SpaceId,
    pub name:        SkillName,
    pub description: SkillDescription,
    pub activation:  SkillActivation,
    pub category:    Option<SkillCategoryUid>,
    pub skill_type:  Option<SkillType>,
}

pub struct DeleteCustomSkillCommand {
    pub uid:      CustomSkillUid,
    pub space_id: SpaceId,
}
```

`SpaceId` et `CoachId` sont des alias d'`EntityId`
(`shared_kernel/identity/ids.rs:9`) — disponibles, et sans emprunt à un autre BC.

### `space_id` sur la modification et la suppression, alors que l'uid suffit

L'uid est une clef primaire globale : il désigne la compétence sans ambiguïté.
**Le `space_id` n'est pas là pour désigner, il est là pour autoriser.**

`space_scope` ne résout que des paramètres de **chemin**, et il compte six
résolveurs — `competition_id`, `season_id`, `team_id`, `player_id`,
`match_report_id`, `article_id`. **Aucun ne connaît une compétence.** Sans le
`space_id` dans la commande et sa vérification dans le use case, l'administrateur
de l'espace A modifierait une compétence de l'espace B en connaissant son uid.

C'est la seule voie ; la nommer ici évite qu'elle se perde en phase 5.

## Sortie : trois templates

### La page hôte

```rust
#[derive(Template)]
#[template(path = "references-custom-skills.html")]
pub struct CustomSkillsPageTemplate {
    pub routes:   Routes,
    pub space_id: String,
}
```

Elle ne porte **que l'assemblage** — deux conteneurs `hx-get`, comme la page des
points manuels. Aucune donnée de compétence n'y transite.

### Le formulaire — trois états dans un enum

```rust
#[derive(Template)]
#[template(path = "references-custom-skill-form.html")]
pub struct CustomSkillFormTemplate {
    pub routes:     Routes,
    pub space_id:   String,
    pub mode:       FormMode,
    pub categories: Vec<CategoryOptionVm>,
}

pub enum FormMode {
    Create,
    /// Inemployée : tout est modifiable.
    Edit { skill: SkillFormVm },
    /// Employée : catégorie et type deviennent des faits affichés.
    /// `usage_count` accompagne le verrou — l'écran dit la cause, pas
    /// seulement l'interdit.
    EditLocked { skill: SkillFormVm, usage_count: u32 },
}

pub struct SkillFormVm {
    pub uid:            String,
    pub name:           String,
    pub description:    String,
    pub category_uid:   String,
    pub category_label: String,
    pub category_css:   String,
    pub is_elite:       bool,
    pub is_active:      bool,
}
```

`FormMode` et non deux booléens `is_edit` / `is_locked` : les trois états sont
exclusifs, et deux booléens en autorisent quatre — dont « création
verrouillée », qui n'existe pas. C'est l'argument exact d'`EditorMode` pour
l'éditeur de roster.

### Les catégories — statiques, sans endpoint JSON

```rust
pub struct CategoryOptionVm { pub uid: String, pub label: String, pub css: String }
```

La maquette pose un `<kreek-select name="category">`. Le composant accepte des
`<option>` **statiques** en alternative à son attribut `url` — documenté dans
`assets/static/js/kreek-select.js`, ligne 23 :

> Options statiques — alternative à `url`, pour une liste figée qui ne justifie…

**Sept catégories, immuables, chargées au démarrage.** Un endpoint JSON leur
coûterait une route, un contrôleur et un aller-retour, pour une liste qu'Askama
rend en même temps que le formulaire.

### La liste

```rust
#[derive(Template)]
#[template(path = "references-custom-skill-list.html")]
pub struct CustomSkillListTemplate {
    pub routes: Routes,
    pub space_id: String,
    pub rows: Vec<CustomSkillRowVm>,
}

pub struct CustomSkillRowVm {
    pub uid:            String,
    pub name:           String,
    pub category_label: String,
    pub category_css:   String,
    pub is_elite:       bool,
    pub activation:     String,
    pub description:    String,
    /// Zéro autorise la suppression ; au-delà, seul le libellé se modifie.
    pub usage_count:    u32,
}
```

Pas de `can_manage` : la page entière est réservée à l'administrateur d'espace,
contrairement à la liste des rosters que tout le monde consulte. Un booléen qui
vaut toujours `true` est une invitation à croire qu'il peut valoir `false`.

## La pastille — le champ qui mérite le plus d'attention

`category_css` apparaît trois fois ci-dessus. Sa valeur vient aujourd'hui de :

```rust
// players/io/app_events/team_created_listener.rs:49
pub fn skill_category_css(category: &str) -> &'static str {
    match category {
        "GENERAL" => "type-general",
        …
        "MUTATIONS" => "type-mutation",   // pluriel à l'entrée, singulier en sortie
        _ => "type-general",
    }
}
```

**`references` n'a pas le droit d'importer `players`.** Trois voies, et la
première est un piège que ce fichier documente lui-même.

| Voie | Ce qu'elle coûte |
|---|---|
| `references` redéclare la table | deux tables qui dérivent — **c'est déjà arrivé** |
| La monter dans `shared_kernel::bloodbowl` | un déplacement, `players` suit par un `use` |
| **`SkillCatalogEntryDto` porte `category_css`** | `references` devient le seul propriétaire, `players` le consomme |

### Pourquoi la duplication est exclue, preuve à l'appui

Le commentaire de la fonction raconte le précédent :

> `MUTATION` au singulier quand le corpus dit `MUTATIONS`, si bien que mutations
> et retors portaient la couleur du général. Personne ne l'avait vu : **une
> couleur fausse ne casse rien, elle ment simplement.**

Et la classe **est figée à l'écriture**, dans `players_proj.acquired_skills` —
pas résolue à l'affichage. Une seconde table fausse ne se corrigerait donc pas
en la corrigeant : les compétences déjà acquises garderaient leur teinte
erronée, comme les anciennes l'ont gardée.

### La voie recommandée : la troisième

**`references` possède les catégories ; il doit posséder leur teinte.** Le DTO
porte déjà `category_label`, résolu par le même adapter, à la ligne d'à côté —
`category_css` y est chez lui, et `players` n'a plus de table du tout.

La seconde voie marche aussi, mais elle laisse la table dans le noyau partagé
alors qu'un seul BC en connaît les valeurs d'entrée.

### Le test à emporter

`aucune_categorie_du_corpus_ne_retombe_sur_le_repli` relie la table au corpus
plutôt qu'à une liste écrite de mémoire — c'est lui qui aurait attrapé
`MUTATIONS`. **Il suit la fonction où qu'elle aille**, et une compétence
personnalisée le satisfait par construction : sa catégorie est l'une des sept.

## Les DTOs de port

```rust
// references/ports.rs — créé par l'épic E10, complété ici
#[async_trait]
pub trait ISkillUsagePort: Send + Sync {
    /// Joueurs qui l'ont acquise **plus** postes qui la posent en compétence de
    /// base (phase 3). Zéro autorise la suppression.
    async fn count_usages(&self, skill_uid: &str) -> Result<u32, String>;
}
```

`IReferencesSpaceAdminPort` est déjà déclaré par l'éditeur de roster, **dans le
même BC** : on le réutilise tel quel. C'est la seule différence avec
`match_report::ISpaceAdminPort`, qu'on ne partage pas parce qu'il est ailleurs.

## Interfaces d'utilisation

| DTO / VM | Émis par | Consommé par |
|---|---|---|
| `CreateCustomSkillDto` | le navigateur (formulaire) | `create_custom_skill_controller` |
| `UpdateCustomSkillDto` | le navigateur (formulaire) | `update_custom_skill_controller` |
| `CreateCustomSkillCommand` | le contrôleur | `create_custom_skill_use_case` |
| `UpdateCustomSkillCommand` | le contrôleur | `update_custom_skill_use_case` |
| `DeleteCustomSkillCommand` | le contrôleur | `delete_custom_skill_use_case` |
| `CustomSkillsPageTemplate` | `custom_skills_page_controller` | le navigateur (page) |
| `CustomSkillFormTemplate` / `FormMode` | `custom_skill_form_controller` | gabarit `references-custom-skill-form.html` |
| `SkillFormVm` | le contrôleur du formulaire | le gabarit, et lui seul |
| `CategoryOptionVm` | le contrôleur du formulaire | le `<kreek-select>` du gabarit |
| `CustomSkillListTemplate` / `CustomSkillRowVm` | `custom_skill_list_controller` | gabarit `references-custom-skill-list.html` |
| `Skill` (modèle du corpus) | `custom_skill_repository` | le cache mémoire, puis tous les lecteurs du catalogue |
| `count_usages` | `SkillUsageAdapter` (infrastructure) | les trois use cases, et le contrôleur de liste |
| `HX-Trigger: customSkillsChanged` | les trois contrôleurs de mutation | le conteneur `#cs-list` |
| `customSkillSelected { skill_id }` | la liste (clic « Modifier ») | le conteneur `#cs-form` |

**Aucun DTO de port n'atteint un gabarit** : `count_usages` rend un `u32` que le
contrôleur pose dans `usage_count`. C'est le seul port de lecture de l'écran, et
il ne rend pas de structure — la règle des domain services n'a rien à arbitrer
ici.

## Règles métier à préciser

**Une seule, et c'est un oubli de la phase 2.**

Le tableau du verrou partiel énumère quatre champs — nom, description,
catégorie, type — mais la maquette en porte **cinq** : l'**activation**, un
segmenté « Passive / Active ».

Je la pose **modifiable**, comme le nom et la description :

- elle ne décide d'aucun prix, contrairement au type et à la catégorie ;
- aucune compétence n'a d'effet mécanique dans kreek — l'activation est un
  qualificatif affiché, que les coachs appliquent sur table ;
- et c'est donc, exactement comme une faute de frappe, **une chose qui se
  corrige**.

Le tableau de la phase 2 devient :

| Mode | Nom | Description | **Activation** | Catégorie | Type |
|---|---|---|---|---|---|
| Création | libre | libre | **libre** | libre | libre |
| Édition, inemployée | libre | libre | **libre** | libre | libre |
| Édition, employée | libre | libre | **libre** | figée | figé |

Si l'activation devait être figée, `UpdateCustomSkillDto` la passerait en
`Option` avec les deux autres — c'est le seul changement de contrat qu'entraîne
la réponse inverse.
