# Agrégat `RecruitmentDraft` — les gardes du recrutement

**Priorité : haute**
**Dépend de :** 257, 258, 259
**Bloque :** 263
**Spec :** `docs/specs/phases-recrutement-renvois/recrutement/06-domaine.md` §3
**Fichiers :** `src/app/teams/domain/recruitment_draft.rs` (nouveau),
`src/app/teams/domain/value_objects.rs`

## Problème

Le panier de recrutement porte des **invariants forts** : plafond de 16, quota par
poste, limites croisées, trésorerie. C'est donc un agrégat, pas un objet applicatif.

La difficulté apparente : ces gardes ont besoin de données d'un autre BC, et le domaine
n'a pas le droit d'appeler un port.

## La tension se résout par hydratation

C'est le pattern déjà en place dans `RosterSelectedTeam`, qui **porte** son
`roster: Roster` (avec `player_definitions`, `allowed_staff`, `cross_limits`). Le use
case hydrate, puis toutes les gardes sont **pures et synchrones** — comme
`check_cross_limits`, `check_player_budget` et `check_staff_limit`
(`team_creation/domain/team_roster_selected.rs`).

```rust
pub struct RecruitmentDraft {
    team_id:  TeamId,
    version:  DraftVersion,
    lines:    Vec<DraftLine>,      // ← seul état persisté
    catalog:  RosterCatalog,       // hydraté (carte 258)
    squad:    SquadSnapshot,       // hydraté (carte 259)
    treasury: Kpo,                 // hydraté depuis Team
}
```

**Aucun `async`, aucun port, aucune dépendance framework.**

## Action

### 1. Méthodes de commande

```rust
pub fn add_player(&mut self, line: RosterLineId) -> Result<DraftLineId, DomainError>
pub fn add_staff(&mut self, staff: StaffType)    -> Result<DraftLineId, DomainError>
pub fn remove_line(&mut self, id: &DraftLineId)  -> Result<(), DomainError>
pub fn validate_all(&self) -> Result<Vec<AppliedLine>, Vec<RejectedLine>>
```

### 2. Huit gardes privées, une par règle

`check_position_in_roster`, `check_squad_max`, `check_position_quota`,
`check_cross_limits`, `check_treasury`, `check_staff_buyable`, `check_staff_allowed`,
`check_staff_quota`.

**Chacune compte possédés + en attente.** C'est le point qui fait qu'un brouillon
respecte les quotas au lieu de les contourner par empilement.

### 3. Le prix du staff est une règle domaine

```rust
fn price_for(&self, staff: StaffType) -> Kpo
```

Le **doublement de la relance hors création** vit ici, pas dans le catalogue : c'est
une règle de saison, pas une donnée de référence. Le catalogue fournit le prix de base
(carte 258), l'agrégat applique le facteur.

Quota de relances : **8**.

### 4. `ActionState` — le domaine décide de la raison du blocage

```rust
pub enum ActionState {
    Allowed,
    Blocked   { cause: BlockCause },   // quota, trésorerie, effectif complet
    Forbidden { cause: ForbidCause },  // roster sans apothicaire
}
```

`Blocked` et `Forbidden` sont distincts : un quota se libère, un roster n'acquiert
jamais le droit à un apothicaire.

C'est la pièce qui matérialise la décision D1 — **les règles écrites une seule fois**.
La couche web ne fait que formuler la cause, elle ne la calcule pas.

### 5. Refus en bloc

`validate_all()` retourne soit **toutes** les lignes applicables, soit la liste des
lignes fautives avec leur `BlockCause`. Jamais un succès partiel.

## Tests unitaires — les 17 de la spec

Les plus importants sont les **2, 4 et 7** : ils vérifient que le brouillon compte ses
propres lignes en attente. Sans eux, toutes les gardes seraient contournables.

Le test 7 mérite une mention : trésorerie insuffisante **pour le total**, chaque ligne
passant seule. C'est la vérification en total, pas ligne par ligne.

## Checklist

- [ ] Agrégat sans `async`, sans port, sans dépendance framework
- [ ] Les 8 gardes comptent possédés + en attente
- [ ] Doublement du prix de relance dans l'agrégat, pas dans le catalogue
- [ ] Quota de relances = 8
- [ ] `ActionState` distingue `Blocked` et `Forbidden`
- [ ] `validate_all` refuse en bloc, jamais partiellement
- [ ] Les 17 tests de `recrutement/06-domaine.md` §6
- [ ] `make check-arch` au vert, `make test` au vert
