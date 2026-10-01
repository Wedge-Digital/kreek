# Panneau d'ajustement · Phase 4 : contrats de données

**Entrée** : `03-back.md` validé.

## Entrée — le formulaire

```rust
#[derive(Deserialize)]
pub struct AdjustTreasuryForm {
    pub direction:  String,   // "Credit" | "Debit"
    pub amount_kpo: u32,
    pub note:       String,
}
```

**Les primitives sont assumées ici** : c'est le format HTTP, pas la commande. Le
contrôleur les traduit en value objects par leurs smart constructors, ce que le
`CLAUDE.md` met explicitement à sa charge.

`direction` reçoit la chaîne que `MovementDirection::as_str()` produit, et
repasse par son `parse()` — sensible à la casse, délibérément. Le formulaire
n'invente pas un second vocabulaire pour le même concept.

## Deux value objects neufs

```rust
/// Montant d'un ajustement : multiple de 5, de 5 à 500 kPo.
#[nutype(
    validate(greater_or_equal = 5, less_or_equal = 500, predicate = |n| n % 5 == 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct AdjustmentAmount(u32);

/// Le motif d'un ajustement — obligatoire, donc jamais un `Option`.
#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 200, regex = TEXTE_SAISI),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct AdjustmentNote(String);
```

**Les bornes vivent dans le type, pas dans l'agrégat.** Multiple de 5, plancher
et plafond sont des propriétés de la valeur : un montant de 123 kPo n'existe
pas, quel que soit l'état de l'équipe. Reste au domaine la seule question qui
dépend de l'état — *le solde couvre-t-il ce retrait*.

**200 caractères et non 100.** `TeamName` est à 100 parce que c'est un nom ; un
motif est une phrase. 200 laisse la place d'expliquer sans qu'une ligne du
relevé devienne un paragraphe. Validé le 2026-10-01.

**`TEXTE_SAISI` est une liste noire** : il refuse les caractères de contrôle et
les overrides bidirectionnels, et laisse passer tout le reste. « Forfait des
Griffons d'Argent — journée 3 » passe, apostrophe et tiret cadratin compris.
C'est le point que l'ancienne liste blanche avait fait rater à onze value
objects.

**Le sens ne crée rien.** `MovementDirection` existe dans `domain/treasury.rs`
avec son `parse()`, et c'est lui qu'on prend. Un second type pour la même notion
divergerait.

## La commande

```rust
#[derive(Debug)]
pub struct AdjustTreasuryCommand {
    pub team_id:    TeamId,
    pub direction:  MovementDirection,
    pub amount:     AdjustmentAmount,
    pub note:       AdjustmentNote,
    pub admin_id:   UserId,
    pub admin_name: CoachName,
}
```

Aucune primitive nue. `CoachName` vit dans
`shared_kernel/identity/coach_name.rs` : pas d'import inter-BC, et `teams`
importe déjà `identity::ids` dans ce même fichier.

Le `Debug` sert la journalisation : le use case est instrumenté
`fields(cmd = ?cmd)`, et rien ici n'est un secret — un motif est fait pour être
lu.

## L'événement domaine

```rust
TeamDomainEvent::TreasuryAdjusted {
    direction:  MovementDirection,
    amount:     AdjustmentAmount,
    note:       AdjustmentNote,
    admin_id:   UserId,
    admin_name: CoachName,
}
```

**Le nom de l'admin est copié dans l'événement, pas résolu à la lecture.** C'est
le nom **au moment de l'acte**, ce qu'un event store doit figer. Le résoudre
plus tard demanderait un port neuf, et réécrirait l'histoire le jour où un coach
se renomme.

### Ce qu'il faut ajouter pour que ça compile

`MovementDirection` dérive aujourd'hui `Debug, Clone, Copy, PartialEq, Eq` —
**pas** `Serialize` ni `Deserialize`. L'événement étant persisté en JSON, il faut
les lui ajouter. Trois mots, repérés en phase 4 plutôt qu'à la compilation.

## Sortie — les view models

```rust
pub struct TreasuryVm {
    …,                                  // inchangé
    pub adjust: Option<AdjustPanelVm>,  // None = le visiteur n'est pas admin
}

pub struct AdjustPanelVm {
    pub post_url:    String,
    pub balance_kpo: u32,   // l'annonce vivante, côté Alpine
    pub min_kpo:     u32,   // 5
    pub step_kpo:    u32,   // 5
    pub max_kpo:     u32,   // 500
}

pub struct AdjustErrorVm {
    pub message: String,
}
```

**`Option` et non un booléen.** Pour un non-admin il n'y a pas de panneau, pas un
panneau vide : le gabarit fait `{% if let Some(adjust) = vm.adjust %}` et ne peut
pas rendre un formulaire sans son URL.

**Les bornes descendent dans le VM** plutôt que d'être écrites en dur dans le
gabarit — règle « un gabarit n'invente aucune valeur ». Sans ça, 500 vivrait à
deux endroits, et le jour où le plafond change l'un des deux serait oublié.

`balance_kpo` est déjà porté par `SummaryVm` ; il est repris ici parce que le
panneau s'en sert pour une autre raison — l'annonce du nouveau solde et le refus
d'un retrait non couvert — et qu'un gabarit qui pioche dans le VM du bandeau
pour alimenter le panneau lierait deux choses qui n'ont pas à l'être.

`AdjustPanelVm` est un **VM de pur domaine plus une route** : construit par
`build_treasury_vm` dans `builders.rs`, où vivent déjà les VMs de cet écran.

## Qui produit, qui consomme

| DTO | Produit par | Consommé par |
|---|---|---|
| `AdjustTreasuryForm` | le `<form>` du panneau, par `hx-post` | `adjust_treasury_controller` |
| `AdjustTreasuryCommand` | `adjust_treasury_controller` | `adjust_treasury_use_case` |
| `TreasuryAdjusted` | l'agrégat `Team` | le dépôt — event store **et** grand livre, même transaction — puis `detail_de()` à la lecture |
| `TreasuryMovementRow.payload` | `list_treasury_movements.sql` | `detail_de()`, qui en tire « Par Bagouze — <motif> » |
| `AdjustPanelVm` | `build_treasury_vm` | `teams-treasury-tab.html` |
| `AdjustErrorVm` | `adjust_treasury_controller` | `teams-treasury-adjust-error.html` |

## Aucun DTO de port

Pas de port neuf, donc pas de DTO de port. `ITeamAccessPort::is_space_admin`
rend un `bool`, qui ne traverse aucune couche : il sert sur place à décider si
`adjust` vaut `Some`.

## Règles métier

**Une seule à cette étape**, et elle est déjà appliquée ci-dessus : le motif est
borné à 200 caractères. Les neuf autres restent celles de `02-front.md`.

La question posée le 2026-10-01 — « vois-tu des règles métier à préciser ? » — n'a
rien fait apparaître d'autre : la phase 4 donne des types à des règles déjà
tranchées.
