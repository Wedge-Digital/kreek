# Trésorerie en mouvements — grand livre et tag dérivé

**Priorité : haute**
**Dépend de :** 251 (bus interne de `teams` et publication depuis l'append)
**Bloque :** 256, 261
**Spec :** `docs/specs/phases-recrutement-renvois/recrutement/07-integration.md` §1-2
**Fichiers :** `src/app/teams/domain/team.rs`, `src/app/teams/domain/treasury.rs` (nouveau),
`src/app/teams/io/repository/team_repository.rs`, migrations

## Problème

La trésorerie est **déjà** dérivée des événements — `apply()` la mute en sept endroits
(`TeamCreated`, `PostMatchSequenceStarted`, `PostMatchSequenceReverted`,
`CostlyMistakesApplied`, `PlayerRecruited`, `StaffBought`, `StaffDismissed`). Mais
**aucun historique n'est consultable** : pas d'événement de mouvement nommé, pas de
motif, pas de projection. On connaît le solde, jamais son chemin.

Second problème : `refund_kpo` crédite la trésorerie à chaque `StaffDismissed`
(`team.rs:477`), alors qu'**un renvoi ne rembourse rien**. Vérifié : à la création
d'équipe non plus il n'existe aucun remboursement — `remaining_budget()` recalcule
`budget − dépensé` depuis les listes courantes (`team_roster_selected.rs:92-95`) et
`remove_staff()` retire simplement l'élément. Le paramètre ne sert nulle part.

## Action

### 1. La méthode domaine décide

```rust
// domain/treasury.rs
pub enum MovementDirection { Credit, Debit }
pub enum MovementReason { StaffPurchase, PlayerRecruitment, MatchIncome, CostlyMistake, … }
pub struct TreasuryMovement {
    pub direction: MovementDirection,
    pub amount:    Kpo,
    pub reason:    MovementReason,
}
```

```rust
// domain/team.rs
impl TeamDomainEvent {
    pub fn treasury_movement(&self) -> Option<TreasuryMovement> { … }
}
```

**`match` exhaustif SANS joker.** Ajouter une variante doit casser la compilation tant
que son effet sur la trésorerie n'est pas déclaré. C'est l'idiome déjà utilisé par
`apply()`, qui liste explicitement ses événements sans effet
(`PlayerRetiredTemporarily | PlayerReEngaged => {}`) plutôt que de les balayer.

C'est cette garantie — et non le tag — qui protège des oublis, notamment quand la
carte 46 (customisation admin) ajoutera ses propres mouvements.

### 2. Le tag est dérivé, jamais saisi

`team_event_store` gagne `tags JSONB NOT NULL DEFAULT '[]'` et un index GIN, sur le
modèle exact d'`event_log` (`20260501102608_event_log.sql`).

À l'append, le tag `"treasury"` est posé **à partir du résultat de
`treasury_movement()`**. Jamais à la main événement par événement : ce serait recréer
l'endroit qu'on peut oublier.

### 3. Le grand livre

```sql
CREATE TABLE teams__treasury_ledger (
    id                BIGSERIAL   PRIMARY KEY,
    team_id           TEXT        NOT NULL,
    event_version     BIGINT      NOT NULL,
    direction         TEXT        NOT NULL,
    amount_kpo        INT         NOT NULL,
    reason            TEXT        NOT NULL,
    balance_after_kpo INT         NOT NULL,
    occurred_at       TIMESTAMPTZ NOT NULL
);
CREATE INDEX teams__treasury_ledger_team ON teams__treasury_ledger (team_id, id);
CREATE UNIQUE INDEX teams__treasury_ledger_source
    ON teams__treasury_ledger (team_id, event_version);
```

Alimenté **dans la même transaction que l'append**, comme toute projection
event-sourcée. L'unicité sur `(team_id, event_version)` rend l'alimentation
idempotente : rejouer un événement ne duplique pas sa ligne.

Reconstruction complète par `WHERE tags @> '["treasury"]'`.

### 4. Supprimer `refund_kpo`

Retirer le champ de `StaffDismissed`, la ligne `self.treasury.0 += refund_kpo.0`
(`team.rs:477`) et le paramètre de `dismiss_staff`.

## Checklist

- [ ] `treasury_movement()` en `match` exhaustif, **sans `_ =>`**
- [ ] Un test qui échoue à la compilation si une variante est ajoutée sans décision
- [ ] `tags` + index GIN sur `team_event_store`, tag posé depuis `treasury_movement()`
- [ ] `teams__treasury_ledger` alimentée dans la même transaction que l'append
- [ ] Index unique `(team_id, event_version)` — alimentation idempotente
- [ ] `refund_kpo` supprimé de l'événement, de `apply()` et de `dismiss_staff`
- [ ] Test : `StaffDismissed` ne crédite plus la trésorerie
- [ ] Test : rejeu d'un flux complet → solde et grand livre cohérents
- [ ] `make check-arch` au vert, `make test` au vert
