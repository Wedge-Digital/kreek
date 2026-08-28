# Page de gestion · Phase 6 : domaine

**Phase 5** : `05-use-cases.md`

## D'abord, une correction : le verrou était au mauvais endroit

La phase 3 posait le verrou partiel dans le use case, et le motivait ainsi :

> Il ne peut pas être dans le domaine : il compare l'état persisté au compte
> d'usage, qui vient d'un port.

**Le raisonnement ne tient pas.** Ce qui vient du port, c'est le *chiffre* ; la
*décision* n'en vient pas. La grille du `CLAUDE.md` tranche exactement ce cas :

| Question | Couche |
|---|---|
| « Quel coût SPP pour ce skill ? » | Use case (via port) |
| « **Le pool SPP est-il suffisant ?** » | **Domaine** |

Le pool est chargé par le use case, et pourtant la décision est domaine. « Cette
catégorie peut-elle encore changer ? » a la même forme, à l'identique : le use
case va chercher le compte, l'agrégat tranche.

Laissé dans le use case, ce contrôle est un `if` — et un `if` que le prochain
use case oubliera. C'est le seul invariant véritable de la fonctionnalité ; il
mérite d'être gardé par un type.

**Conséquence sur la phase 5** : `update_custom_skill_use_case` ne compare plus
rien lui-même. Il charge, il compte, il appelle `amend`, il persiste.

## Les règles — vingt et une, en cinq familles

### Portée et accès

| | Règle |
|---|---|
| P1 | Seul un **admin d'espace** crée, modifie ou supprime |
| P2 | Une compétence d'un autre espace est **introuvable**, pas interdite |
| P3 | La **résolution** par uid ne filtre pas par espace ; seule la **liste** filtre |

P3 mérite son motif, le même que pour les rosters : un joueur déjà pourvu doit
résoudre sa compétence où qu'on le regarde — sur sa fiche, dans un rapport, dans
une page consultée depuis un autre espace. **Ce qui se garde par l'espace, c'est
le choix, pas la lecture.**

### Identité

| | Règle |
|---|---|
| I1 | L'uid est **engendré côté serveur**, préfixe `CUSTOM_` |
| I2 | Le préfixe est un **invariant du type**, pas une convention vérifiée |
| I3 | L'uid est **conservé** à la modification |

I2 est ce qui rend « on ne modifie pas une compétence du règlement » impossible
plutôt qu'interdit : les commandes de modification et de suppression n'acceptent
que `CustomSkillUid`, dont le smart constructor exige le préfixe.

### Contenu

| | Règle |
|---|---|
| C1 | Nom : rogné, non vide, ≤ 50 caractères, `TEXTE_SAISI` |
| C2 | Description : rognée, non vide, ≤ 600 caractères, `TEXTE_SAISI` |
| C3 | Catégorie : l'une des **sept** du corpus, `TRAITS` comprise |
| C4 | Type : `Standard` ou `Élite`, **sérialisé avec son accent** |
| C5 | Activation : `Active` ou `Passive` |
| C6 | Le nom est **unique dans la liste fusionnée** corpus + espace |

### Verrou d'usage

| | Règle |
|---|---|
| U1 | Une compétence portée **n'est pas supprimable** |
| U2 | Nom, description et activation restent modifiables **quel que soit l'usage** |
| U3 | Catégorie et type sont **figés dès qu'un porteur existe** |
| U4 | Le compte est **re-vérifié à l'écriture**, jamais celui de l'écran |
| U5 | Le compte additionne joueurs porteurs **et** postes qui la posent en base |
| U6 | Le compte n'est demandé que si un champ risqué **change réellement** |
| U7 | Port indisponible → refus, **mais seulement de ce qu'il concerne** |
| U8 | Un joueur licencié **compte** comme porteur |

### Effets

| | Règle |
|---|---|
| E1 | La suppression n'émet **aucun événement** — plus rien ne cite l'uid |
| E2 | Le renommage ne réécrit rien dans `players_proj` — résolution par uid |

## La ligne : ce que le domaine juge, et ce qu'il ne peut pas

| Contrôle | Couche | Pourquoi |
|---|---|---|
| C1, C2, C4, C5, I2 | **value objects** | des bornes, pas des règles |
| U1, U2, U3, U6 | **domaine** | l'état courant plus un chiffre, rien d'autre |
| C3, C6 | **use case** | il faut le corpus, ou la liste de l'espace |
| P1, P2, U4, U5, U7 | **use case** | il faut un port |

Le domaine ne peut pas vérifier qu'une catégorie existe sans connaître
`IReferenceRepository` — ce qui lui ferait connaître un port, que le `CLAUDE.md`
lui interdit. Il ne peut pas davantage vérifier l'unicité d'un nom, qui demande
la liste de l'espace.

**Le domaine garantit qu'une compétence ne change pas sous les pieds de ceux qui
l'ont payée. Le use case garantit qu'elle ne référence rien d'inexistant et
qu'elle ne fait double emploi avec rien.**

## `CustomSkill` — la forme

```rust
// references/domain/custom_skill.rs
pub struct CustomSkill {
    uid:         CustomSkillUid,
    space_id:    SpaceId,
    name:        SkillName,
    category:    SkillCategoryUid,
    skill_type:  SkillType,
    activation:  SkillActivation,
    description: SkillDescription,
}
```

**Champs privés.** Un invariant gardé par `amend` reste contournable tant que
`category` est `pub` — et `CustomSkill` n'est jamais désérialisé depuis le web,
donc rien ne pousse à les ouvrir : c'est la commande qu'on désérialise, et elle
porte déjà ses value objects (phase 4).

### Le constructeur est total

```rust
pub struct CustomSkillDraft {
    pub uid:         CustomSkillUid,
    pub space_id:    SpaceId,
    pub name:        SkillName,
    pub category:    SkillCategoryUid,
    pub skill_type:  SkillType,
    pub activation:  SkillActivation,
    pub description: SkillDescription,
}

impl CustomSkill {
    pub fn new(draft: CustomSkillDraft) -> Self;   // pas de Result
}
```

`CustomRoster::try_new` rend un `Result` parce qu'il a sept règles structurelles
— au moins un poste, exactement un journalier, des noms distincts. **Celui-ci
n'en a aucune.** Une fois les sept value objects construits, il ne reste rien à
vérifier qui ne demande le corpus ou le compte d'usage.

Un `Result` qui ne peut pas échouer oblige chaque appelant à traiter un cas qui
n'arrive pas, et laisse croire à une garde qui n'existe pas. **Mieux vaut le
dire que le mimer.**

Le `Draft` reste, malgré sept arguments seulement : cinq d'entre eux sont des
enveloppes de `String` que le compilateur laisserait volontiers s'intervertir.
`name` et `description` voisins, tous deux textuels — c'est exactement
l'inversion qu'une structure nommée rend impossible.

### `amend` — le seul vrai geste métier

```rust
pub struct SkillAmendment {
    pub name:        SkillName,
    pub description: SkillDescription,
    pub activation:  SkillActivation,
    /// `None` = le champ n'était pas à l'écran (compétence portée).
    pub category:    Option<SkillCategoryUid>,
    pub skill_type:  Option<SkillType>,
}

impl CustomSkill {
    /// `holders` est ce que le use case est allé chercher. La décision, elle,
    /// est ici — c'est « le pool SPP est-il suffisant ? » de la grille.
    pub fn amend(
        &mut self,
        amendment: SkillAmendment,
        holders: Holders,
    ) -> Result<(), DomainError>;
}
```

Elle porte U2, U3 et U6 à elle seule :

```
si holders > 0 et category  = Some(c) et c ≠ self.category   → SkillCategoryFrozen
si holders > 0 et skill_type = Some(t) et t ≠ self.skill_type → SkillTypeFrozen
sinon : nom, description, activation écrasés ; category et type écrasés si Some
```

**`Some` ne suffit pas — il faut `Some` et différent.** Un écran déverrouillé
renvoie toujours les deux champs, à leur valeur d'origine dans le cas courant.
Traiter leur présence comme une demande de changement ferait échouer une
correction de faute de frappe dès qu'un joueur porte la compétence, **alors que
rien de risqué n'était demandé**. C'est le piège de cette méthode, et le test qui
le couvre compte autant que les refus.

**Le refus est total.** `amend` ne mute rien avant d'avoir tout vérifié : pas de
nom enregistré en écartant la catégorie. Une écriture partielle silencieuse est
pire qu'un refus — c'est ce qui a valu la carte 427.

**La catégorie est examinée avant le type**, et l'ordre est arbitraire : les deux
se ferment ensemble, donc un appelant honnête ne peut pas en heurter un seul.
Seul un POST écrit à la main atteint ce cas, et le premier refus lui suffit.

### `ensure_deletable` — mince, et assumée

```rust
pub fn ensure_deletable(&self, holders: Holders) -> Result<(), DomainError>;
```

Elle n'utilise presque pas `self` : c'est une comparaison à zéro, et un lecteur
pointilleux dirait que c'est une fonction libre déguisée.

**Elle reste sur l'agrégat quand même**, pour deux raisons. C'est là qu'on
cherche « quand peut-on supprimer une compétence ? » — mise ailleurs, la réponse
se trouverait dans le use case pour la suppression et dans le domaine pour la
modification, ce qui est précisément le désordre que la correction en tête de
cette phase répare. Et son erreur porte le compte, donc l'écran nomme la cause.

Le jour où U1 se nuancera — « supprimable si seuls des joueurs licenciés la
portent », par exemple — la méthode aura besoin de `self`, et elle sera déjà au
bon endroit.

### La conversion vers le format de stockage

```rust
/// Le `Skill` du corpus, tel qu'il sera stocké et servi par
/// `find_skill_by_uid`. Total : les sept champs sont déjà valides.
pub fn to_reference_skill(&self) -> Skill;
```

Même argument que `to_reference_team()` : un `CustomSkill` construit est valide,
sa conversion vers un type moins strict ne peut pas échouer.

Le sens inverse n'existe pas. Pour l'édition, c'est **la base** qu'on relit —
`find_custom_skill` rend un enregistrement qui porte le `space_id` (phase 5), et
c'est de lui qu'on reconstruit l'agrégat, pas d'un `Skill` amputé.

### Les lecteurs

```rust
pub fn uid(&self)      -> &CustomSkillUid;
pub fn space_id(&self) -> &SpaceId;
pub fn belongs_to(&self, space: &SpaceId) -> bool;
```

`belongs_to` plutôt que de comparer `space_id()` chez l'appelant : P2 se lit
alors dans le use case comme une question, et non comme une égalité qu'on peut
écrire à l'envers.

## Les value objects

### Ceux qui naissent ici

```rust
#[nutype(sanitize(trim), validate(not_empty, len_char_max = 50,  regex = TEXTE_SAISI), …)]
pub struct SkillName(String);

#[nutype(sanitize(trim), validate(not_empty, len_char_max = 600, regex = TEXTE_SAISI), …)]
pub struct SkillDescription(String);

#[nutype(validate(predicate = |s| s.starts_with("CUSTOM_")), …)]
pub struct CustomSkillUid(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillType { Standard, #[serde(rename = "Élite")] Elite }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillActivation { Active, Passive }

#[nutype(derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord))]
pub struct Holders(u32);
```

**`TEXTE_SAISI` et aucun charset local** — la règle du commit `542bdfd`. Onze
charsets avaient coexisté, et neuf refusaient l'apostrophe ; une compétence
nommée « Capitaine d'équipe » échouait à la validation sur un `UnknownSkill` qui
accusait le catalogue. Cette fonctionnalité crée des compétences nommées par des
humains : c'est exactement le terrain où ça se reproduirait.

**Le piège nutype à ne pas oublier** : une expression passée par une constante
n'est compilée qu'au premier usage. Une faute de syntaxe ne produit pas une
erreur de `cargo build` mais un `panic` en production. Les tests de `charset.rs`
touchent les deux constantes — c'est ce qui referme le trou, et ces deux types
doivent y être exercés.

### `Holders` est un type et non un `u32`

Trois signatures le prennent, et un `u32` nu y voisinerait avec les compteurs de
caractères, les prix, les quantités. C'est aussi ce qui permet à `amend` et
`ensure_deletable` de dire ce qu'elles attendent : pas un nombre, **le nombre de
porteurs**.

### Le piège des 10 kPo, redit parce qu'il se relit à l'implémentation

Un seul site décide de l'élitisme dans toute l'application :

```rust
// infrastructure/players/skill_catalog_adapter.rs:39
is_elite: skill.skill_type == "Élite",
```

Une comparaison de chaînes exacte, accent compris. `SkillType::Elite` sérialisé
`"Elite"` produirait des compétences Élite **que tout le système traiterait
comme Standard** — 10 kPo de moins à l'achat en SPP, un barème faux, et pas la
moindre erreur. D'où le `#[serde(rename = "Élite")]`, et le test qui le fixe.

## Les erreurs domaine

```rust
// references/domain/error.rs — créé par la carte 440
pub enum DomainError {
    …                                        // celles du roster personnalisé
    SkillInUse          { holders: u32 },    // U1
    SkillCategoryFrozen { holders: u32 },    // U3
    SkillTypeFrozen     { holders: u32 },    // U3
}
```

**Deux variantes gelées plutôt qu'un `ImmutableFieldChanged { field: &'static
str }`.** Une chaîne dans une erreur domaine dérive — elle se recopie, se
traduit, se faute — là où un `match` exhaustif oblige chaque nouveau lecteur à
traiter les deux cas. C'est l'argument qui a scindé `NoJourneymanPosition` et
`SeveralJourneymanPositions`.

**Chacune porte `holders`**, parce que l'écran doit nommer la cause : « la
catégorie ne peut plus changer, trois joueurs ont payé le barème de Force » se
comprend ; « modification refusée » envoie chercher.

## Tests

### `amend` — le verrou partiel

| Test | Règle |
|---|---|
| `une_competence_portee_accepte_un_nouveau_nom` | U2 |
| `une_competence_portee_accepte_une_nouvelle_description` | U2 |
| `une_competence_portee_accepte_une_nouvelle_activation` | U2 — le cas passant qu'on oublie |
| `une_competence_portee_refuse_un_changement_de_categorie` | U3 |
| `une_competence_portee_refuse_un_changement_de_type` | U3 |
| `une_competence_inemployee_accepte_les_cinq_champs` | U3 a contrario |
| `une_categorie_renvoyee_identique_ne_declenche_pas_le_verrou` | **U6 — le piège** |
| `un_type_renvoye_identique_ne_declenche_pas_le_verrou` | U6 |
| `un_refus_ne_mute_aucun_champ` | l'écriture partielle |

`une_categorie_renvoyee_identique_ne_declenche_pas_le_verrou` est le test qui
compte le plus : sans lui, la fonctionnalité *paraît* correcte — les refus
refusent, les acceptations acceptent — et un administrateur découvre en
production qu'il ne peut plus corriger une faute de frappe.

`une_competence_portee_accepte_une_nouvelle_activation` couvre la règle décidée
en phase 4, celle que le tableau de la phase 2 avait omise.

### `ensure_deletable`

| Test | Règle |
|---|---|
| `une_competence_portee_ne_se_supprime_pas` | U1 |
| `une_competence_inemployee_se_supprime` | U1 a contrario |
| `l_erreur_de_suppression_porte_le_nombre_de_porteurs` | le message de l'écran |

### Les value objects

| Test | Règle |
|---|---|
| `un_uid_sans_prefixe_est_refuse` | I2 |
| `un_nom_avec_apostrophe_est_accepte` | C1 — « Capitaine d'équipe » |
| `un_nom_de_51_caracteres_est_refuse` | C1 |
| `une_description_de_601_caracteres_est_refusee` | C2 |
| `une_description_vide_est_refusee` | C2 |
| `elite_se_serialise_avec_son_accent` | **C4 — les 10 kPo** |
| `standard_et_elite_se_deserialisent_depuis_le_corpus` | C4, dans l'autre sens |

`un_nom_avec_apostrophe_est_accepte` n'est pas une politesse : c'est le cas exact
qui a coûté le commit `542bdfd`, et le seul moyen de vérifier que ces deux types
ont bien pris `TEXTE_SAISI` et non un charset recopié.

### La conversion

| Test | Ce qu'il prouve |
|---|---|
| `to_reference_skill_conserve_les_sept_champs` | rien ne se perd au stockage |
| `to_reference_skill_porte_l_uid_prefixe` | I1 |

## Ce que la phase n'ajoute pas

- **Aucun agrégat événementiel.** `CustomSkill` n'a pas d'historique : il
  s'écrit, se relit, se remplace. Et contrairement au roster, il n'émet rien même
  à la suppression (E1).
- **Aucune méthode de mutation champ par champ.** Pas de `set_name`,
  `set_category`. `amend` prend l'amendement entier, et vérifie en bloc — une
  suite de mutateurs laisserait l'agrégat dans un état intermédiaire qu'aucun
  invariant ne garde.
- **Aucune connaissance du corpus.** L'existence d'une catégorie et l'unicité
  d'un nom restent au use case ; les y laisser est ce qui garde le domaine pur.
