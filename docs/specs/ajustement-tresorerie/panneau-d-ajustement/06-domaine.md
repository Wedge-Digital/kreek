# Panneau d'ajustement · Phase 6 : domaine

**Entrée** : `05-use-cases.md` validé.

## Les règles métier, récapitulées — phases 1 à 5

Validées le 2026-10-01.

### Qui

1. **Admin d'espace seul**, par `ITeamAccessPort::is_space_admin`. Le
   **propriétaire de l'équipe est exclu** — c'est la règle du commissaire, pas
   celle du coach. Le relevé, lui, reste lisible par tous.

### Le montant

2. Le **sens porte le signe** ; le montant saisi est positif.
3. Il est un **multiple de 5 kPo**, comme tous les prix du jeu.
4. Il va de **5 à 500 kPo**. Le plafond est un garde-fou de frappe.
5. Un **retrait supérieur au solde est refusé** — `DomainError::InsufficientTreasury`.

### Le motif

6. **Obligatoire.**
7. **200 caractères au plus**, charset `TEXTE_SAISI`.

### La trace

8. L'ajustement est **public**, avec le nom de son auteur **figé au moment de
   l'acte**.
9. **Rien ne se défait** : un ajustement de sens opposé corrige une erreur, et
   les deux lignes restent lisibles.

### Quand

10. **À tout moment.** Aucune garde de phase de jeu, et aucune garde de statut de
    participation : une équipe renvoyée reste ajustable. Un commissaire corrige
    souvent *parce que* l'équipe est bloquée, et une garde l'en empêcherait.
11. Une **écriture concurrente est refusée**, jamais rejouée.

### Les effets

12. Un crédit grossit « Encaissé », un débit « Dépensé ». Pas de cinquième terme
    dans l'équation du bandeau.
13. **Aucun effet sur la valeur d'équipe** — `team_value.rs` somme des joueurs et
    du staff, la trésorerie n'y entre pas.

## L'agrégat

### Il ne gagne aucun champ

`treasury: Kpo` existe, et c'est la seule chose que l'ajustement touche.

### Une méthode

À ranger près d'`apply_costly_mistakes`, dont elle partage la forme :

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

**Une seule règle** : un débit supérieur à `self.treasury` rend
`InsufficientTreasury` ; sinon elle rend l'événement.

Ni `expect_phase`, ni garde de statut de participation. C'est la **seule méthode
de commande du BC dans ce cas**, et c'est délibéré — règle 10. Écrit ici pour
qu'une relecture ne prenne pas l'absence pour un oubli.

Les bornes du montant ne sont pas vérifiées ici : `AdjustmentAmount` ne peut pas
exister hors de ses bornes. Les revérifier donnerait deux endroits à tenir
d'accord, et le second finirait par diverger.

### Deux branches ailleurs, dont une vide

**`treasury_movement()`** reçoit un bras qui rend `credit` ou `debit` selon le
sens, avec `MovementReason::AdminAdjustment`.

**`apply()`** reçoit un bras **sans corps**. La trésorerie est posée en tête
d'`apply` par `treasury_movement()` —

> « La trésorerie est traitée une fois pour toutes, ici : `apply` et le grand
> livre lisent le même `treasury_movement()`, donc ils ne peuvent pas
> diverger. »

— et un ajustement ne change rien d'autre dans l'agrégat. Le bras existe parce
que le `match` est exhaustif, et c'est tout.

C'est ce qui rend l'ajout si mince : le mécanisme de solde existe, on y branche
un neuvième motif.

### Le neuvième motif

`MovementReason::AdminAdjustment`, à poser **à trois endroits** de
`treasury.rs` : `ALL`, `as_str`, et la `garde_d_exhaustivite` du module de
tests. Le fichier le dit lui-même — ajouter une variante sans l'ajouter à `ALL`
compile sans un mot, et le relevé s'arrête en production sur un `UnknownReason`.

`tous_les_motifs_font_l_aller_retour` passe alors de huit à neuf, assertion
finale comprise.

## Ce que cette carte ne garde pas

**`Team` expose ses 25 champs en `pub`, `treasury` compris.** L'invariant ajouté
par `adjust_treasury` est gardé par la méthode, et reste **contournable** par qui
écrirait `team.treasury = …` directement.

Ce n'est pas une faiblesse introduite ici : c'est l'état de l'agrégat pour ses
vingt méthodes, mesuré à 139 lectures de champ dans 33 fichiers. Le corriger est
un chantier à part — **carte 556** — et le faire à moitié, un champ privé sur
vingt-cinq, donnerait l'illusion d'un agrégat gardé sans en être un.

Écrit ici pour qu'on ne lise pas dans cette phase une garantie qu'elle n'offre
pas.

## Les tests

### Sur l'agrégat

| Test | Règle |
|---|---|
| un crédit produit l'événement, et `apply` monte le solde | 2, 12 |
| un débit couvert produit l'événement, et `apply` baisse le solde | 2, 12 |
| un débit **égal** au solde passe | 5, la borne |
| un débit supérieur au solde rend `InsufficientTreasury` et **ne produit rien** | 5 |
| le même ajustement passe dans deux phases de jeu différentes | 10 |
| il passe sur une équipe `Dismissed` | 10 |
| `treasury_movement()` rend le bon sens et le motif `AdminAdjustment` | 12 |

Le test des deux phases est le seul qui vérifie une **absence**. Sans lui, un
`expect_phase` ajouté par réflexe plus tard ne casserait rien.

### Sur les value objects

`AdjustmentAmount` : 5 accepté, 500 accepté, 0 refusé, 4 refusé, 123 refusé
(multiple), 505 refusé (plafond).

`AdjustmentNote` : vide refusé, 200 caractères accepté, 201 refusé, un motif à
apostrophe et tiret cadratin accepté — c'est le cas que l'ancienne liste blanche
faisait échouer.

### Sur le motif

`tous_les_motifs_font_l_aller_retour`, étendu. Et `garde_d_exhaustivite`, qui
**casse la compilation** si la variante n'est pas nommée : c'est elle, et non le
test, qui ferme la boucle.
