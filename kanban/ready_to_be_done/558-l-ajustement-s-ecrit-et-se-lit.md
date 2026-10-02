# L'ajustement s'écrit, et le relevé le raconte

**Ordre :** 2 · **Dépend de :** `557`
**Conception :** `docs/specs/ajustement-tresorerie/panneau-d-ajustement/`
(`05-use-cases.md`, `07-integration.md`)

## Objectif

Le use case qui écrit, et la ligne de relevé qui en sort lisible. Toujours aucun
écran : la carte se prouve par un test d'intégration sur une vraie base.

## Conception

### 1. Le use case — `teams/use_cases/adjust_treasury_use_case.rs`

```rust
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: AdjustTreasuryCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), AdjustTreasuryError>
```

Charger, appeler `adjust_treasury`, `append`. **Il n'émet rien** : dans `teams`
c'est le dépôt qui publie, après le commit — « le seul point qui les couvre
tous », deux des quatre chemins vers `ReadyToPlay` passant par des listeners.
C'est une déviation assumée du patron de `players` et `match_report`, et s'en
écarter ici produirait une double émission.

`skip_all` est indispensable : sans lui l'attribut tenterait d'enregistrer le
dépôt, qui n'implémente pas `Debug`.

**Il rend `()`**, pas le nouveau solde. La réponse HTTP relit de toute façon, et
rendre un solde créerait une seconde source pour un chiffre que le relevé lit de
la dernière ligne du grand livre.

```rust
pub enum AdjustTreasuryError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}
```

**`ConcurrentWrite` remonte tel quel et n'est jamais rejoué.** Le rejeu
silencieux sur un état non relu peut créditer une caisse que la recette d'un
match publié entre-temps a déjà remplie : le commissaire validerait un geste
qu'il n'aurait pas fait en voyant l'écran. Le refus coûte un clic.

### 2. La commande — `teams/use_cases/commands.rs`

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

Aucune primitive nue. `CoachName` vit dans `shared_kernel/identity/` : pas
d'import inter-BC.

### 3. La ligne lisible — `treasury_statement_service.rs`

Un bras dans `detail_de()` :

```rust
MovementReason::AdminAdjustment => ajustement(payload),
```

qui rend **« Par <admin_name> — <note> »**. Le `payload` est celui de
l'événement, joint par `list_treasury_movements.sql` : aucune colonne à ajouter
au grand livre, aucune migration.

Si le payload manque — ligne dont l'événement a disparu — la fonction rend un
libellé sobre plutôt que de faire échouer l'assemblage. Le `LEFT JOIN` existe
pour cette raison, et le fichier SQL l'explique : « un relevé à trou se lit comme
une erreur de calcul et se cherche du mauvais côté ».

### 4. Rien d'autre à toucher

- **Aucune méthode de dépôt neuve.** `append` écrit event store, projection et
  grand livre dans une transaction ; l'insertion au grand livre est pilotée par
  `treasury_movement()`.
- **Aucun bras de projection.** `team_proj` ne porte pas de colonne trésorerie,
  et son `match` finit par un joker.
- **Aucun listener.** L'événement ne rejoint pas `ends_in_ready_to_play()` : la
  valeur d'équipe ne dépend pas de la trésorerie. C'est l'inverse du cas de la
  carte 46 — ne pas faire le rapprochement.

## Checklist

- [ ] `AdjustTreasuryCommand`
- [ ] `adjust_treasury_use_case.rs`, instrumenté `skip_all`
- [ ] Bras `AdminAdjustment` dans `detail_de()`, avec son repli sans payload
- [ ] Test use case : équipe introuvable → `TeamNotFound`, **rien d'appendé**
- [ ] Test use case : refus domaine remonté tel quel, **rien d'appendé**
- [ ] Test use case : l'événement appendé porte le motif, le nom et le bon sens
- [ ] Test d'intégration sur vraie base : après l'écriture,
      `teams__treasury_ledger` porte une ligne `AdminAdjustment` dont
      `balance_after_kpo` est le solde attendu
- [ ] Test de `detail_de()` : « Par Bagouze — … », et le repli sans payload
