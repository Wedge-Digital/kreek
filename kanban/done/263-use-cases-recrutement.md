# Use cases de recrutement — hydratation, panier, validation en lot

**Priorité : haute**
**Dépend de :** 256, 261, 262
**Bloque :** 264
**Spec :** `docs/specs/phases-recrutement-renvois/recrutement/05-use-cases.md`
**Fichiers :** `src/app/teams/use_cases/basket_hydration_service.rs` (nouveau),
`src/app/teams/use_cases/add_basket_player_use_case.rs`,
`add_basket_staff_use_case.rs`, `remove_basket_line_use_case.rs`,
`validate_recruitment_phase_use_case.rs`, `src/app/teams/use_cases/commands.rs`

## Problème

Les mutations du panier et l'application du lot demandent une orchestration : charger
l'agrégat `Team`, charger le panier, l'hydrater depuis deux ports, appeler le
domaine, persister avec garde de version.

Aucun de ces use cases n'existe, et `validate_recruitment_phase_use_case` — qui existe
— ne fait aujourd'hui que valider la transition de phase.

## Action

### 1. Le domain service d'hydratation

```rust
pub async fn hydrate_recruitment_basket(
    team:       &Team,
    basket_repo: &dyn IPhaseBasketRepository,
    catalog:    &dyn IRosterCatalogPort,
    squad:      &dyn ISquadPort,
) -> Result<RecruitmentBasket, HydrationError>;
```

**Le seul endroit où les DTOs de port sont manipulés.** Au-delà, tout est domaine —
aucun handler, aucun template ne voit un DTO.

Un panier absent n'est pas une erreur : on hydrate un panier vide.

### 2. Les trois use cases de mutation

Même forme : charger `Team` → vérifier la phase → hydrater → appeler la méthode
domaine → `basket_repo.save(&basket, expected_version)` → retourner le panier.

**Le use case ne décide de rien.** Les quotas, les limites croisées, le plafond de 16
et la trésorerie sont évalués par l'agrégat.

`remove_basket_line_use_case` est **partagé avec les renvois** : retirer une ligne d'un
panier par son identifiant est la même opération, quelle que soit la phase.

### 3. La validation — le seul use case complexe

1. charger `Team`, vérifier la phase
2. hydrater **contre l'état du jour** — prix, effectif et trésorerie rechargés, jamais
   ceux de la constitution du panier
3. `basket.validate_all()` → **refus en bloc** si une seule ligne est invalide
4. construire le lot : un `PlayerRecruited` **par joueur**, un `StaffBought` par ligne
   de staff, `RecruitmentPhaseValidated` en dernier
5. `team_repo.append_batch(&team_id, &events, team.version)` (carte 256)
6. `basket_repo.delete(...)` — **hors transaction**, voir ci-dessous

### 4. Pourquoi la suppression du panier peut sortir de la transaction

L'y inclure obligerait le use case à porter la transaction, donc à exposer des types
`sqlx` dans `ports.rs`.

**Ce n'est pas nécessaire : l'agrégat se protège déjà.** Le dernier événement du lot
fait passer l'équipe en `Dismissals` ; une revalidation appelle
`expect_phase(GamePhase::Recruitment)` et **échoue**. La double application est
impossible.

Un panier résiduel est alors inatteignable et sera purgé à l'entrée suivante en
`ReadyToPlay` (carte 257).

### 5. Un événement par ligne, jamais de lot

L'event store reste lisible — « ce joueur a été recruté tel jour » — et le grand livre
de trésorerie en découle directement. Un événement de lot obligerait à le déplier à
chaque rejeu et à chaque projection.

L'ordre d'application est libre : la trésorerie ayant été vérifiée **en total**, aucune
ligne ne peut échouer en cours de lot par manque d'argent.

### 6. Erreurs applicatives

`TeamNotFound`, `WrongPhase`, `ConcurrentWrite`,
`BasketNoLongerValid(Vec<RejectedLine>)`, `Domain`, `Repository`.

`RejectedLine` porte un **motif structuré** (`BlockCause`), pas un message : la couche
web formule. Une seule énumération des causes, pas deux.

## Checklist

- [x] `hydrate_recruitment_basket` est le seul consommateur des DTOs de port
- [x] Les 3 use cases de mutation ne contiennent aucune logique métier
- [x] `remove_basket_line_use_case` réutilisable par les renvois
- [x] Validation : refus en bloc, jamais de succès partiel
- [x] Un événement par ligne + transition en dernier, via `append_batch`
- [x] Test : revalider après succès → `WrongPhase`, aucune double application
- [x] Test : panier vide → seul `RecruitmentPhaseValidated` est appendu
- [x] Test : ligne devenue invalide → rien n'est appliqué, les lignes fautives sont nommées
- [x] `make check-arch` au vert, `make test` au vert
