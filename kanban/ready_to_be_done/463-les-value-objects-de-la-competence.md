# Les value objects de la compétence

**Épic :** E10 — Référentiels éditables · **Ordre :** 1 · **Dépend de :** 439
**Conception :** `docs/specs/competences-personnalisees/page-de-gestion/`
(`04-dtos.md`, `06-domaine.md`)

## Objectif

Donner au projet de quoi dire « ce nom de compétence est valide », « ce type est
Élite ». Aucun écran, aucune écriture.

## Six types

```rust
// references/domain/value_objects.rs
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

`SkillCategoryUid` vient de la carte 439 — **ne pas le redéclarer**.

## Le piège qui coûte 10 kPo

Un seul site décide de l'élitisme dans toute l'application :

```rust
// infrastructure/players/skill_catalog_adapter.rs:39
is_elite: skill.skill_type == "Élite",
```

Une comparaison de chaînes exacte, **accent compris**. `SkillType::Elite`
sérialisé `"Elite"` produirait des compétences Élite que tout le système
traiterait comme Standard — 10 kPo de moins à l'achat en SPP, un barème faux, et
pas la moindre erreur. D'où le `#[serde(rename)]`, et le test qui le fixe.

## Le préfixe est un invariant, pas une convention

`CustomSkillUid` n'accepte que `CUSTOM_…`. Les commandes de modification et de
suppression ne prennent que ce type : **« on ne modifie pas une compétence du
règlement » devient une impossibilité de construction**, et non un contrôle
qu'un troisième use case oublierait un jour.

## `Holders` est un type et non un `u32`

Trois signatures le prennent, et un `u32` nu y voisinerait avec les compteurs de
caractères, les prix, les quantités. Il dit ce qu'il est : **le nombre de
porteurs**, pas un nombre.

## Le piège nutype à ne pas oublier

Une expression passée par une **constante** n'est compilée qu'au premier usage :
une faute de syntaxe ne produit pas une erreur de `cargo build` mais un `panic`
en production. Les tests de `charset.rs` touchent les deux constantes — ces deux
types textuels doivent y être exercés.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `un_uid_sans_prefixe_est_refuse` | le préfixe est dans le type |
| `un_nom_avec_apostrophe_est_accepte` | « Capitaine d'équipe » — `TEXTE_SAISI` et non un charset recopié |
| `un_nom_de_51_caracteres_est_refuse` | la borne haute |
| `un_nom_vide_est_refuse` | la borne basse compte autant |
| `une_description_de_601_caracteres_est_refusee` | la borne de C2 |
| `elite_se_serialise_avec_son_accent` | **les 10 kPo** |
| `standard_et_elite_se_deserialisent_depuis_le_corpus` | C4 dans l'autre sens |

`un_nom_avec_apostrophe_est_accepte` n'est pas une politesse : c'est le cas exact
qui a coûté le commit `542bdfd`, et le seul moyen de vérifier que ces types ont
bien pris `TEXTE_SAISI`.

## Checklist

- [ ] `references/domain/value_objects.rs` avec les six types
- [ ] `SkillCategoryUid` **réutilisé** de la 439, pas redéclaré
- [ ] Les deux types textuels exercés dans les tests de `charset.rs`
- [ ] Les sept tests
- [ ] `make lint && make test && make check-arch`
