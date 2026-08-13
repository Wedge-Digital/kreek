# Phase 5 — Use cases — player-detail

## Correctif à la phase 4 : les commandes portent `expected_version`

Omission relevée en lisant `basket_mutation.rs`. Les cinq commandes de mutation
gagnent un champ :

```rust
pub expected_version: u32,
```

Il vient du **formulaire**, cuit dans les `hx-vals` du panneau au moment de son
rendu — et non d'un agrégat conservé côté serveur. Le panneau étant re-rendu
après chaque mutation, la version qu'il porte est toujours celle d'après la
dernière écriture.

C'est bien ce que met en garde `basket_mutation.rs`, et qu'il faut lire
précisément : le piège de la carte 264 n'est pas de faire circuler la version,
c'est de **reposer sur l'agrégat** celle que `save` vient de rendre. L'agrégat
garde alors la version d'avant écriture, et chaque second clic échoue.

---

## Les cinq mutations de panier

Même forme, et **elles ne décident de rien** : hydrater, appeler la méthode
domaine, persister sous garde de version.

```rust
// use_cases/customisation_basket_mutation.rs

pub async fn add_skill(
    cmd: AddCustomisationSkillCommand,
    space_id: &str,
    player_repo: &dyn IPlayerRepository,
    basket_repo: &dyn ICustomisationBasketRepository,
    catalog: &dyn ISkillCatalogPort,
) -> Result<(), CustomisationBasketError>

pub async fn add_stat(cmd: AddCustomisationStatCommand, …)     -> Result<(), CustomisationBasketError>
pub async fn adjust_price(cmd: AdjustCustomisationPriceCommand, …) -> Result<(), CustomisationBasketError>
pub async fn add_spp(cmd: AddCustomisationSppCommand, …)       -> Result<(), CustomisationBasketError>
pub async fn remove_line(cmd: RemoveCustomisationLineCommand, …) -> Result<(), CustomisationBasketError>
```

**Aucune ne rend l'agrégat muté**, pour la raison ci-dessus. Le handler relit
avant de rendre le panneau.

### Orchestration, identique aux cinq

1. Charger le joueur (`player_repo.find_by_id`) — absent → `PlayerNotFound`.
2. Charger les lignes du panier (`basket_repo.load`) — absent → panier vide,
   ce qui **ouvre** le mode : c'est le premier geste du commissaire.
3. Hydrater via le domain service : joueur + catalogue + lignes → agrégat.
4. Appeler la méthode domaine (`add_skill`, `add_stat`, …) → `DomainError` en
   cas de refus.
5. Persister les **lignes seules** avec `expected_version`.

L'étape 2 mérite d'être notée : **la première mutation crée le panier**. Il n'y
a pas d'endpoint « ouvrir » séparé — le `GET` du widget crée lui aussi un
panier vide, la phase 2 ayant fait de son existence le signal du mode.

### Erreurs

```rust
pub enum CustomisationBasketError {
    PlayerNotFound,
    /// Un autre onglet a modifié le panier entre son affichage et ce geste.
    ConcurrentWrite,
    Domain(DomainError),
    Hydration(HydrationError),
    Repository(RepositoryError),
}
```

`Domain` porte les refus métier de la phase 1 — borne dépassée, compétence déjà
présente, prix négatif, plafond de SPP. Ce sont eux que le handler traduit en
`RefusalVm`, affiché à côté de l'action refusée.

---

## Validation

```rust
// use_cases/validate_customisation_use_case.rs
pub async fn execute(
    cmd: ValidateCustomisationCommand,
    player_repo: &dyn IPlayerRepository,
    basket_repo: &dyn ICustomisationBasketRepository,
    catalog: &dyn ISkillCatalogPort,
    event_bus: &EventBus,
) -> Result<(), ValidateCustomisationError>
```

### Orchestration

1. Charger joueur et panier. Panier absent ou vide → `NothingToApply`.
2. Hydrater, et **revalider chaque ligne** contre l'état courant. Une ligne
   devenue invalide depuis son ajout — compétence acquise entre-temps, borne
   atteinte par une autre voie — doit être refusée ici, pas appliquée.
3. Pour chaque ligne, appeler la méthode domaine correspondante sur `Player` →
   **un événement par customisation** (phase 1). Rien n'est encore écrit.
4. **Supprimer le panier** — pas le vider (phase 2 : son existence commande
   l'affichage). Avant l'append, cf. « atomicité » ci-dessous.
5. Appendre le lot en une seule transaction (`append_batch`, carte 291).
6. Émettre les événements sur le bus interne.

### Erreurs

```rust
pub enum ValidateCustomisationError {
    PlayerNotFound,
    NothingToApply,
    /// Une ou plusieurs lignes ne sont plus applicables — rien n'est appliqué.
    LinesRejected(Vec<RejectedLine>),
    ConcurrentWrite,
    Domain(DomainError),
    Repository(RepositoryError),
}
```

**Tout ou rien.** Si une ligne est refusée à la revalidation, aucune n'est
appliquée. Appliquer les valides et rejeter le reste laisserait le commissaire
devant un panier partiellement consommé, sans savoir ce qui est passé — et le
`HX-Refresh` de la réponse effacerait la trace de ce qui a échoué.

`RejectedLine` existe déjà dans `teams::domain::basket` ; ici c'est un type
propre à `players`, même intention.

---

## Annulation

```rust
pub async fn cancel(
    cmd: CancelCustomisationCommand,
    basket_repo: &dyn ICustomisationBasketRepository,
) -> Result<(), CancelCustomisationError>
```

Supprime le panier. Ni joueur chargé, ni domaine appelé : rien n'a été engagé,
il n'y a rien à défaire. Un panier déjà absent n'est **pas** une erreur —
l'annulation est idempotente, et deux clics ne doivent pas produire un message
d'échec.

---

## Un point d'atomicité à trancher

Les événements et le panier vivent dans **deux tables**, écrites par **deux
transactions** : `append_batch` gère la sienne, la suppression du panier est un
`DELETE` séparé.

Si l'append réussit et que la suppression échoue, le panier survit à des
customisations déjà appliquées. Le commissaire le retrouve à l'écran, le
revalide, et **les applique une seconde fois**.

**Décision : supprimer le panier d'abord, appendre ensuite.**

L'ordre de l'orchestration devient donc — hydrater, revalider, **supprimer le
panier**, appendre le lot, émettre.

Ce que ça change en cas de panne entre les deux : le panier est perdu et rien
n'est appliqué. Le commissaire ressaisit. C'est un désagrément, pas une
corruption — là où l'ordre inverse écrirait deux fois des customisations sur
des données de jeu, ce qui ne se découvrirait que bien plus tard et sans moyen
simple de défaire.

Les deux autres issues sont écartées. **Une transaction commune** fermerait
vraiment le trou, mais elle élargit le port repository et fait manipuler une
transaction à un use case — ce que le projet évite partout ailleurs.
**Rendre la validation idempotente** par identifiant de ligne serait robuste,
mais supposerait de lire l'event store à chaque validation pour savoir ce qui a
déjà été appliqué.

Reste une conséquence à assumer : entre la suppression et l'append, un `GET`
concurrent verrait le mode fermé alors que rien n'est encore écrit. La fenêtre
est de quelques millisecondes, et la phase 2 a déjà écarté la concurrence entre
commissaires comme improbable au niveau métier.

---

## Règles métier (identifiées phase 5)

- **La première mutation crée le panier.** Pas d'endpoint d'ouverture ; le
  `GET` du widget crée lui aussi un panier vide.
- **Revalidation intégrale à la validation.** Une ligne valide à l'ajout peut
  ne plus l'être à l'application.
- **Tout ou rien à la validation.**
- **L'annulation est idempotente.**

## Points ouverts

- Durée de vie d'un panier abandonné, et sort d'un panier visant un joueur
  renvoyé entre-temps (hérités des phases 2 et 3).
