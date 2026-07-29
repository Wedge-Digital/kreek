# Recrutement — Phase 6 : domaine

**Entrée** : récapitulatif des 48 règles métier **validé** (README, section
« Règles métier transverses »).

Ce document couvre le domaine **commun aux deux pages** — value objects, erreurs,
trésorerie — puis l'agrégat du recrutement. `renvois/06-domaine.md` consigne ses
écarts.

## 1. Value objects

```rust
// domain/value_objects.rs — ajouts
pub struct DraftLineId(String);        // ULID ; identifiant technique, sans invariant
pub struct DraftVersion(u32);
pub struct RosterLineId(String);       // smart constructor : non vide
pub struct PositionQuota(u8);
pub struct CrossLimit { max: u8, position_uids: Vec<RosterLineId> }
```

`Kpo`, `StaffType`, `StaffQuantity` existent déjà et ne changent pas.

**Pas de `Jersey` ici** : `teams` ignore les numéros de maillot (règle 44). Le value
object vit dans `players`.

## 2. Erreurs domaine

`DomainError` (`domain/error.rs`) gagne huit variantes. Les cinq existantes utiles —
`WrongGamePhase`, `StaffTypeNotBuyable`, `StaffTypeNotDismissable`,
`InsufficientStaff`, `InsufficientTreasury` — sont conservées telles quelles.

| Variante | Règle |
|---|---|
| `MaxPlayersReached` | 13 |
| `PositionQuotaReached` | 14 |
| `CrossLimitExceeded` | 15 |
| `PositionNotInRoster` | 16 |
| `StaffNotAllowedForRoster` | 20 |
| `StaffQuotaReached` | 21 |
| `EligibleFloorReached` | 24 |
| `DraftLineNotFound` | 34 |

## 3. L'agrégat `RecruitmentDraft`

```rust
// domain/recruitment_draft.rs
pub struct RecruitmentDraft {
    team_id:  TeamId,
    version:  DraftVersion,
    lines:    Vec<DraftLine>,      // ← seul état persisté
    catalog:  RosterCatalog,       // hydraté à chaque chargement
    squad:    SquadSnapshot,       // hydraté
    treasury: Kpo,                 // hydraté depuis Team
}

pub enum DraftLine {
    Player { id: DraftLineId, roster_line: RosterLineId, price: Kpo },
    Staff  { id: DraftLineId, staff_type: StaffType,     price: Kpo },
}
```

**Aucun `async`, aucun port, aucune dépendance framework.** Tout est évaluable
synchroniquement une fois l'agrégat hydraté — c'est ce que la phase 3 a acheté.

### Méthodes de commande

```rust
pub fn add_player(&mut self, line: RosterLineId) -> Result<DraftLineId, DomainError>
pub fn add_staff(&mut self, staff: StaffType)    -> Result<DraftLineId, DomainError>
pub fn remove_line(&mut self, id: &DraftLineId)  -> Result<(), DomainError>
pub fn validate_all(&self) -> Result<Vec<AppliedLine>, Vec<RejectedLine>>
```

### Gardes privées, une par règle

```rust
fn check_position_in_roster(&self, line: &RosterLineId) -> Result<(), DomainError>  // 16
fn check_squad_max(&self)                               -> Result<(), DomainError>  // 13
fn check_position_quota(&self, line: &RosterLineId)     -> Result<(), DomainError>  // 14
fn check_cross_limits(&self, line: &RosterLineId)       -> Result<(), DomainError>  // 15
fn check_treasury(&self, additional: Kpo)               -> Result<(), DomainError>  // 7, 8
fn check_staff_buyable(&self, staff: StaffType)         -> Result<(), DomainError>  // 18, 19
fn check_staff_allowed(&self, staff: StaffType)         -> Result<(), DomainError>  // 20
fn check_staff_quota(&self, staff: StaffType)           -> Result<(), DomainError>  // 21
```

Chaque garde compte **possédés + en attente** : c'est ce qui fait qu'un brouillon
respecte les quotas au lieu de les contourner.

### Le prix du staff est une règle domaine

```rust
fn price_for(&self, staff: StaffType) -> Kpo
```

Le doublement de la relance hors création (règle 22) vit ici, pas dans le catalogue :
c'est une règle de saison, pas une donnée de référence. Le catalogue fournit le prix
de base, l'agrégat applique le facteur.

### Méthodes de lecture pour les VMs

```rust
pub fn action_for_position(&self, line: &RosterLineId) -> ActionState
pub fn action_for_staff(&self, staff: StaffType)       -> ActionState
pub fn projected_squad_size(&self) -> u8
pub fn remaining_treasury(&self)   -> Kpo
pub fn pending_for_position(&self, line: &RosterLineId) -> u8
```

`ActionState` est l'équivalent domaine de `ActionVm` : c'est **le domaine qui décide
de la raison du blocage**, la couche web ne fait que la formuler. C'est la traduction
en code de la décision D1 — les règles écrites une seule fois.

```rust
pub enum ActionState {
    Allowed,
    Blocked   { cause: BlockCause },   // quota, trésorerie, effectif complet
    Forbidden { cause: ForbidCause },  // roster sans apothicaire
}
```

`Blocked` et `Forbidden` sont distincts parce qu'un quota se libère et qu'un roster
n'acquiert jamais le droit à un apothicaire.

## 4. Méthodes ajoutées à `Team`

```rust
pub fn recruit_player(&self, position: PositionId, base_value: Kpo, cost: Kpo)
    -> Result<TeamDomainEvent, DomainError>          // PlayerRecruited
```

Garde : phase `Recruitment` et trésorerie suffisante. La vérification par ligne est
**redondante** avec le contrôle en total du brouillon — le total garantit que les
débits successifs ne passent jamais sous zéro — mais elle protège l'invariant propre à
`Team` : sa trésorerie n'est jamais négative. On la garde comme filet.

`buy_staff` est **corrigée** : l'apothicaire devient achetable (règle 18), le facteur
fans reste refusé (19). La condition `allowed_staff` **reste dans le brouillon** —
`Team` ne connaît pas son roster.

`dismiss_staff` est **corrigée** : la relance devient renvoyable (règle 29), et
`refund_kpo` **disparaît de la signature et de l'événement** (règles 10, 27, 32).

## 5. Trésorerie

```rust
pub struct TreasuryMovement {
    pub direction: MovementDirection,   // Credit | Debit
    pub amount:    Kpo,
    pub reason:    MovementReason,
}

impl TeamDomainEvent {
    /// `match` exhaustif SANS joker : ajouter une variante casse la compilation
    /// tant que son effet sur la trésorerie n'est pas déclaré (règle 12).
    pub fn treasury_movement(&self) -> Option<TreasuryMovement>
}
```

L'idiome est déjà celui d'`apply()`, qui liste explicitement ses événements sans effet
plutôt que de les balayer par `_ =>`.

Retournent `None` : tous les événements de renvoi (règle 10), les transitions de
phase, les événements d'identité.

## 6. Tests unitaires prévus

Un test par règle, plus les cas limites. Tous sont des tests de domaine pur — aucune
base, aucun HTTP.

### `RecruitmentDraft`

| # | Test | Règle |
|---|---|---|
| 1 | 16 joueurs possédés → `add_player` refuse | 13 |
| 2 | 15 possédés + 1 en attente → refuse | 13, brouillon compté |
| 3 | quota de poste atteint par les possédés → refuse | 14 |
| 4 | quota atteint par un mélange possédés/en attente → refuse | 14 |
| 5 | limite croisée atteinte sur deux postes différents → refuse | 15 |
| 6 | poste absent du roster → refuse | 16 |
| 7 | trésorerie insuffisante pour **le total**, chaque ligne passant seule → refuse | 8 |
| 8 | facteur fans → `StaffTypeNotBuyable` | 19 |
| 9 | apothicaire sur roster non autorisé → `StaffNotAllowedForRoster` | 20 |
| 10 | apothicaire sur roster autorisé → accepté | 18, 20 |
| 11 | 8 relances → la neuvième refuse | 21 |
| 12 | prix de relance = 2 × prix de base du roster | 22 |
| 13 | `remove_line` libère quota et trésorerie | 33, 34 |
| 14 | `remove_line` sur un identifiant inconnu → `DraftLineNotFound` | 34 |
| 15 | `validate_all` : une ligne invalide → **rien** n'est appliqué | 36 |
| 16 | `validate_all` sur brouillon vide → lot vide, pas d'erreur | — |
| 17 | `action_for_position` retourne la cause exacte du blocage | D1 |

### `Team`

| # | Test | Règle |
|---|---|---|
| 18 | `recruit_player` hors phase `Recruitment` → `WrongGamePhase` | 3 |
| 19 | `recruit_player` débite la trésorerie du coût | 9 |
| 20 | `buy_staff` accepte l'apothicaire | 18 |
| 21 | `buy_staff` refuse le facteur fans | 19 |
| 22 | `dismiss_staff` accepte la relance | 29 |
| 23 | `dismiss_staff` ne crédite plus la trésorerie | 10, 32 |
| 24 | `treasury_movement` : `StaffBought` → débit du coût | 11 |
| 25 | `treasury_movement` : `StaffDismissed` → `None` | 10 |
| 26 | rejeu d'un flux complet → trésorerie et compteurs corrects | 11 |

Les tests 2, 4 et 7 sont les plus importants : ils vérifient que **le brouillon compte
ses propres lignes en attente**, sans quoi toutes les gardes seraient contournables en
empilant des ajouts.

## 7. Points tranchés à cette étape

- **`RejectedLine` porte un motif structuré**, pas un message : `BlockCause` est déjà
  une énumération domaine, la couche web la formule. Une seule énumération des causes,
  pas deux.
- **`append_batch` accepte un lot vide** : le coach peut terminer sa phase sans rien
  acheter (test 16). Seul `RecruitmentPhaseValidated` est alors appendu.

## 8. Point ouvert pour la phase 7

- La course entre le recalcul de valeur d'équipe et la sortie d'effectif — traitée
  dans `renvois/06-domaine.md`, puisqu'elle ne concerne que les renvois.
