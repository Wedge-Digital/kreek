# Renvois — Phase 6 : domaine

**Entrée** : récapitulatif des 48 règles métier **validé**.

Les value objects, les erreurs, la trésorerie et l'idiome `ActionState` sont décrits
dans `recrutement/06-domaine.md`. Ce document consigne les écarts.

## 1. L'agrégat `DismissalsBasket`

```rust
// domain/dismissals_basket.rs
pub struct DismissalsBasket {
    team_id: TeamId,
    version: BasketVersion,
    lines:   Vec<DismissalBasketLine>,   // ← seul état persisté
    squad:   SquadSnapshot,        // hydraté
    catalog: RosterCatalog,        // hydraté — pour le staff possédé
}

pub enum DismissalBasketLine {
    Player { id: BasketLineId, player_id: PlayerId },
    Staff  { id: BasketLineId, staff_type: StaffType },
}
```

**Pas de trésorerie** : un renvoi ne rembourse rien (règle 27), l'agrégat n'a aucune
raison de la connaître.

### Méthodes de commande

```rust
pub fn mark_player(&mut self, id: PlayerId)    -> Result<BasketLineId, DomainError>
pub fn mark_staff(&mut self, staff: StaffType) -> Result<BasketLineId, DomainError>
pub fn remove_line(&mut self, id: &BasketLineId) -> Result<(), DomainError>
pub fn validate_all(&self) -> Result<Vec<AppliedLine>, Vec<RejectedLine>>
```

### Deux gardes seulement

```rust
fn check_eligible_floor(&self, id: &PlayerId) -> Result<(), DomainError>  // 24, 25
fn check_staff_owned(&self, staff: StaffType) -> Result<(), DomainError>  // 31
```

Toutes les gardes de composition du recrutement — plafond de 16, quota par poste,
limites croisées, trésorerie — sont **sans objet** : retirer ne peut violer aucune
borne haute.

### Le plancher, précisément

```rust
fn check_eligible_floor(&self, id: &PlayerId) -> Result<(), DomainError> {
    let player = self.squad.find(id).ok_or(DomainError::PlayerNotInSquad)?;
    // Un absent ne compte pas parmi les éligibles : le renvoyer n'entame
    // pas le plancher (règle 25).
    if !player.available_for_next_match { return Ok(()); }
    if self.eligible_after_basket() <= MIN_ELIGIBLE {
        return Err(DomainError::EligibleFloorReached);
    }
    Ok(())
}
```

`MIN_ELIGIBLE = 11`. `eligible_after_basket()` compte les membres actifs disponibles
**moins** les joueurs déjà marqués — c'est ce qui fait que le plancher se resserre à
chaque marquage (règle 28).

### `ActionState` gagne un troisième cas

```rust
pub enum DismissalActionState {
    Removable,
    Marked,                          // ← sans équivalent au recrutement
    Blocked { cause: BlockCause },   // EligibleFloor
}
```

`Marked` existe parce qu'une ligne s'annule ici depuis la ligne du joueur, pas
seulement depuis le panier.

## 2. Méthodes ajoutées à `Team`

```rust
pub fn dismiss_player(&self, player: PlayerId, value_at_dismissal: Kpo)
    -> Result<TeamDomainEvent, DomainError>          // PlayerDismissed
```

Garde : phase `Dismissals` uniquement. `Team` ne connaît ni l'effectif ni les
éligibles — le plancher est vérifié par le panier, qui les porte.

`value_at_dismissal` est conservée dans l'événement bien qu'elle ne serve à aucun
calcul : elle documente ce que valait le joueur au moment du renvoi, information non
reconstructible une fois qu'il a quitté l'effectif. C'est le même raisonnement que
pour `base_value_kpo` sur `PlayerRecruited` (cf. carte 251).

**`PlayerFired` est renommé `PlayerDismissed`** — jamais émis, renommage gratuit.

## 3. Trésorerie — ce que le compilateur va exiger

```rust
TeamDomainEvent::PlayerDismissed { .. }  => None,   // règle 27
TeamDomainEvent::StaffDismissed  { .. }  => None,   // règle 32
```

C'est la traduction en code, **vérifiée à la compilation**, de « un renvoi ne
rembourse rien ». Le `match` exhaustif sans joker force cette déclaration ; on ne peut
pas l'oublier.

## 4. La course entre recalcul de TV et sortie d'effectif — tranchée

Le recalcul de valeur d'équipe (carte 251) part du bus **interne** de `teams` sur
`DismissalsPhaseValidated` ; la sortie d'effectif transite par l'**app event bus** vers
`players`. Rien ne garantit l'ordre, donc la TV peut être recalculée en comptant
encore les renvoyés.

**Retenu : un second recalcul, déclenché côté `teams` à la réception de
`PlayerDismissed`.**

- `TeamValueRecomputed` porte une valeur **absolue** : un recalcul de plus est
  inoffensif, le dernier gagne.
- L'opération est idempotente et peu coûteuse.
- Écarté : faire dépendre le recalcul d'un accusé de traitement de `players` —
  couplage fort entre deux BCs pour un problème de fraîcheur.

À porter dans la carte 251, dont le listener gagne un déclencheur.

## 5. Tests unitaires prévus

### `DismissalsBasket`

| # | Test | Règle |
|---|---|---|
| 1 | 12 éligibles → marquer un disponible passe | 24 |
| 2 | 11 éligibles → marquer un disponible refuse | 24 |
| 3 | 12 éligibles, un marqué → le second refuse | 24, 28 |
| 4 | 9 éligibles → marquer un **absent** passe | 25 |
| 5 | 9 éligibles → marquer un disponible refuse | 24 |
| 6 | démarquer rend un éligible et rouvre le marquage | 28, 34 |
| 7 | marquer plus de staff que possédé → `InsufficientStaff` | 31 |
| 8 | staff possédé 2, un marqué → le second passe, le troisième refuse | 31 |
| 9 | joueur absent de l'effectif → `PlayerNotInSquad` | — |
| 10 | `validate_all` : une ligne invalide → **rien** n'est appliqué | 36 |
| 11 | panier vide → lot vide, pas d'erreur | — |

### `Team`

| # | Test | Règle |
|---|---|---|
| 12 | `dismiss_player` hors phase `Dismissals` → `WrongGamePhase` | 3 |
| 13 | `dismiss_player` ne touche pas la trésorerie | 27 |
| 14 | `treasury_movement(PlayerDismissed)` → `None` | 27 |
| 15 | `treasury_movement(StaffDismissed)` → `None` | 32 |
| 16 | `dismiss_staff` décrémente le compteur sans créditer | 32 |

Les tests 3, 4 et 6 sont les plus importants : ils couvrent l'interaction entre le
plancher et le contenu du panier, qui est la seule vraie subtilité de cette page.

## 6. Un cas qu'on croyait ouvert, et qui ne l'est pas

**Un rapport dépublié ne peut pas référencer un joueur `Dismissed`.** Vérifié dans
`correction_eligibility_service.rs:70-81` : la correction est bloquée dès que l'un des
deux camps n'est plus en phase `PlayerImprovement`.

```rust
if matches!(status.in_improvement, Ok(false)) {
    return Some(CorrectionBlocker::PhaseAdvanced { side });
}
```

Le garde-fou n'est donc pas « les SPP n'ont pas été dépensés » mais « les deux équipes
sont **encore** en phase d'amélioration » — strictement plus fort, puisque valider
cette phase sans rien dépenser bloque déjà la correction.

Or `Dismissals` vient après `Recruitment`, lui-même après `PlayerImprovement`. Une
équipe ayant renvoyé un joueur a nécessairement quitté la phase d'amélioration, donc
le rapport est déjà incorrigible.

**Le cas est impossible par construction**, et le garde-fou échoue fermé : un port qui
ne répond pas bloque aussi (`EligibilityUnknown`). Aucune interaction avec la série
227-236 n'est à prévoir.
