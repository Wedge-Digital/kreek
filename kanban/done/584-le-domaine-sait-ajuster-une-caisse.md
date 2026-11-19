# Le domaine sait ajuster une caisse

> **Numérotée 557 jusqu'au 2026-10-04**, numéro qu'elle partageait avec une autre carte. Les commits déjà poussés portent `[557]` ; ils ne sont pas réécrits.

**Ordre :** 1 · **Dépend de :** rien
**Conception :** `docs/specs/ajustement-tresorerie/panneau-d-ajustement/`
(`04-dtos.md`, `06-domaine.md`)

## Objectif

Les deux value objects, le neuvième motif, l'événement et la méthode d'agrégat.
**Aucun écran, aucune route** — mais `cargo test` passe au vert sur toutes les
règles.

## Conception

### 1. Deux value objects — `teams/domain/value_objects.rs`

```rust
#[nutype(
    validate(greater_or_equal = 5, less_or_equal = 500, predicate = |n| n % 5 == 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct AdjustmentAmount(u32);

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 200, regex = TEXTE_SAISI),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct AdjustmentNote(String);
```

**Les bornes vivent ici et nulle part ailleurs.** L'agrégat ne les revérifie
pas : `AdjustmentAmount` ne peut pas exister hors d'elles, et un second contrôle
donnerait deux endroits à tenir d'accord.

Si nutype refuse de combiner `predicate` avec les deux bornes, replier les trois
dans un seul `predicate` — et le dire en commentaire, pour que le repli ne se
lise pas comme un choix de style.

### 2. Le neuvième motif — `teams/domain/treasury.rs`

`MovementReason::AdminAdjustment`, à poser **à trois endroits** :

| Endroit | Ce qui arrive si on l'oublie |
|---|---|
| `as_str` | le compilateur refuse — c'est le seul des trois qu'il garde |
| `ALL` | compile sans un mot, `parse` rend `None`, **le relevé s'arrête en production** sur un `UnknownReason` |
| `garde_d_exhaustivite` (tests) | le `match` casse la compilation des tests, ce qui ramène ici |

Porter `ALL` de `; 8]` à `; 9]`, et l'assertion finale de
`tous_les_motifs_font_l_aller_retour` de `8` à `9`.

### 3. `MovementDirection` gagne deux dérivations

Il porte `Debug, Clone, Copy, PartialEq, Eq` — **pas** `Serialize` ni
`Deserialize`. L'événement est persisté en JSON : sans elles, rien ne compile.

### 4. L'événement — `teams/domain/team.rs`

```rust
TreasuryAdjusted {
    direction:  MovementDirection,
    amount:     AdjustmentAmount,
    note:       AdjustmentNote,
    admin_id:   UserId,
    admin_name: CoachName,
}
```

**Le nom est copié dans l'événement**, pas résolu à la lecture : c'est le nom au
moment de l'acte. Le résoudre plus tard demanderait un port neuf et réécrirait
l'histoire le jour où un coach se renomme.

Ajouter la variante **cassera la compilation de `to_app_event()`**, qui n'a pas
de joker. C'est voulu : elle rejoint la liste nommée de celles qui ne sortent
pas du BC, aucun autre ne se souciant de la caisse d'une équipe.

### 5. La méthode d'agrégat

```rust
pub fn adjust_treasury(
    &self,
    direction:  MovementDirection,
    amount:     AdjustmentAmount,
    note:       AdjustmentNote,
    admin_id:   UserId,
    admin_name: CoachName,
) -> Result<TeamDomainEvent, DomainError>
```

Une seule règle : un débit supérieur à `self.treasury` rend
`DomainError::InsufficientTreasury`, qui **existe déjà**.

**Ni `expect_phase`, ni garde de statut de participation.** C'est la seule
méthode de commande du BC dans ce cas, et c'est délibéré — un commissaire
corrige souvent *parce que* l'équipe est bloquée. Une équipe `Dismissed` reste
ajustable.

### 6. Deux bras, dont un vide

`treasury_movement()` rend `credit` ou `debit` selon le sens, motif
`AdminAdjustment`.

`apply()` reçoit un bras **sans corps** : la trésorerie est posée en tête
d'`apply` par `treasury_movement()`, et l'ajustement ne change rien d'autre. Le
bras n'existe que parce que le `match` est exhaustif.

## Ce que cette carte ne garde pas

`Team` expose ses 25 champs en `pub`, `treasury` compris : l'invariant ajouté
reste contournable par `team.treasury = …`. Ce n'est pas introduit ici — c'est
l'état de l'agrégat pour ses vingt méthodes. Carte **583**.

## Checklist

- [ ] `AdjustmentAmount` + `AdjustmentNote`
- [ ] `MovementReason::AdminAdjustment` aux **trois** endroits, `ALL` à `; 9]`
- [ ] `Serialize`/`Deserialize` sur `MovementDirection`
- [ ] `TeamDomainEvent::TreasuryAdjusted`
- [ ] La variante nommée dans le bras « ne sort pas » de `to_app_event()`
- [ ] `Team::adjust_treasury()`
- [ ] Bras de `treasury_movement()` ; bras vide d'`apply()`
- [ ] Tests VO : 5 et 500 acceptés ; 0, 4, 123, 505 refusés
- [ ] Tests note : vide refusé, 200 accepté, 201 refusé, apostrophe et tiret cadratin acceptés
- [ ] Tests agrégat : crédit, débit couvert, débit **égal** au solde, débit non couvert refusé sans événement
- [ ] Test d'**absence** : le même ajustement passe dans deux phases, et sur une équipe `Dismissed`
- [ ] `tous_les_motifs_font_l_aller_retour` étendu à neuf
