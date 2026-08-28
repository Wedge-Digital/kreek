# `CustomSkill` — le gardien du verrou partiel

**Épic :** E10 — Référentiels éditables · **Ordre :** 2 · **Dépend de :** 463, 440
**Conception :** `docs/specs/competences-personnalisees/page-de-gestion/06-domaine.md`

## Objectif

L'agrégat et sa seule vraie règle : **une compétence ne change pas de coût sous
les pieds de ceux qui l'ont payée.**

## La correction qui motive cette carte

Les phases 3 et 5 avaient mis ce verrou dans le use case, au motif qu'il compare
l'état persisté à un compte venu d'un port. **Ce qui vient du port, c'est le
chiffre ; la décision n'en vient pas.** La grille du `CLAUDE.md` tranche le cas à
l'identique : le pool SPP est chargé par le use case, et « le pool est-il
suffisant ? » reste une question de domaine.

Laissé dans le use case, ce contrôle est un `if` que le prochain use case
oubliera.

## La forme

```rust
// references/domain/custom_skill.rs
pub struct CustomSkill {
    uid: CustomSkillUid, space_id: SpaceId,
    name: SkillName, category: SkillCategoryUid,
    skill_type: SkillType, activation: SkillActivation,
    description: SkillDescription,
}

impl CustomSkill {
    pub fn new(draft: CustomSkillDraft) -> Self;                    // total
    pub fn amend(&mut self, a: SkillAmendment, h: Holders) -> Result<(), DomainError>;
    pub fn ensure_deletable(&self, h: Holders) -> Result<(), DomainError>;
    pub fn to_reference_skill(&self) -> Skill;                      // total
    pub fn belongs_to(&self, space: &SpaceId) -> bool;
}
```

**Champs privés** : un invariant gardé par `amend` reste contournable tant que
`category` est `pub`. L'agrégat n'est jamais désérialisé depuis le web — c'est la
commande qui l'est.

### `new` et non `try_new`

`CustomRoster` rend un `Result` parce qu'il a sept règles structurelles.
**Celui-ci n'en a aucune** une fois ses sept value objects construits. Un
`Result` qui ne peut pas échouer oblige chaque appelant à traiter un cas qui
n'arrive pas, et laisse croire à une garde qui n'existe pas.

Le `Draft` reste malgré tout : cinq des sept champs sont des enveloppes de
`String`, et `name` / `description` voisins sont exactement l'inversion qu'une
structure nommée rend impossible.

### `amend` — les trois règles du verrou

```rust
pub struct SkillAmendment {
    pub name: SkillName, pub description: SkillDescription,
    pub activation: SkillActivation,
    pub category: Option<SkillCategoryUid>,   // None = absent de l'écran
    pub skill_type: Option<SkillType>,
}
```

```
si holders > 0 et category   = Some(c) et c ≠ self.category    → SkillCategoryFrozen
si holders > 0 et skill_type = Some(t) et t ≠ self.skill_type  → SkillTypeFrozen
sinon : nom, description, activation écrasés ; category et type écrasés si Some
```

**`Some` ne suffit pas — il faut `Some` et différent.** Un écran déverrouillé
renvoie toujours les deux champs, à leur valeur d'origine dans le cas courant.
Traiter leur présence comme une demande de changement ferait échouer une
correction de faute de frappe dès qu'un joueur porte la compétence, **alors que
rien de risqué n'était demandé**. C'est le piège de cette carte.

**Le refus est total** : rien n'est muté avant que tout soit vérifié. Une
écriture partielle silencieuse est pire qu'un refus — c'est ce qui a valu la
carte 427.

### `ensure_deletable` est mince, et assumée

Elle n'utilise presque pas `self`. Elle reste sur l'agrégat pour que les deux
refus se lisent au même endroit, et parce que le jour où la règle se nuancera
— « supprimable si seuls des joueurs licenciés la portent » — elle aura besoin
de `self` et sera déjà au bon endroit.

## Les erreurs

```rust
// s'ajoutent au DomainError créé par la carte 440
SkillInUse          { holders: u32 },
SkillCategoryFrozen { holders: u32 },
SkillTypeFrozen     { holders: u32 },
```

**Deux variantes gelées et non un `field: &'static str`** : une chaîne dans une
erreur domaine dérive, un `match` exhaustif oblige chaque lecteur à traiter les
deux cas. Chacune porte `holders`, parce que l'écran doit nommer la cause.

## Tests

| Test | Règle |
|---|---|
| `une_competence_portee_accepte_un_nouveau_nom` | U2 |
| `une_competence_portee_accepte_une_nouvelle_description` | U2 |
| `une_competence_portee_accepte_une_nouvelle_activation` | U2 — le cas passant qu'on oublie |
| `une_competence_portee_refuse_un_changement_de_categorie` | U3 |
| `une_competence_portee_refuse_un_changement_de_type` | U3 |
| `une_competence_inemployee_accepte_les_cinq_champs` | U3 a contrario |
| `une_categorie_renvoyee_identique_ne_declenche_pas_le_verrou` | **U6** |
| `un_type_renvoye_identique_ne_declenche_pas_le_verrou` | U6 |
| `un_refus_ne_mute_aucun_champ` | l'écriture partielle |
| `une_competence_portee_ne_se_supprime_pas` | U1 |
| `une_competence_inemployee_se_supprime` | U1 a contrario |
| `to_reference_skill_conserve_les_sept_champs` | la conversion |

`une_categorie_renvoyee_identique_ne_declenche_pas_le_verrou` est **le test qui
compte le plus** : sans lui la fonctionnalité paraît correcte — les refus
refusent, les acceptations acceptent — et un administrateur découvre en
production qu'il ne peut plus corriger une faute de frappe.

## Checklist

- [ ] `references/domain/custom_skill.rs`
- [ ] Les trois variantes d'erreur ajoutées au `DomainError` de la 440
- [ ] Les douze tests
- [ ] Aucun champ `pub` sur l'agrégat
- [ ] `make lint && make test && make check-arch`
