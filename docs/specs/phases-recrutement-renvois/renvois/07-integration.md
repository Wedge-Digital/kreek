# Renvois — Phase 7 : persistance, événements, réponses

**Entrée** : `06-domaine.md` validé.

Les migrations, `append_batch`, le grand livre, les conventions de handlers et de
templates sont décrits dans `recrutement/07-integration.md`. Ce document consigne les
écarts.

## 1. Persistance

**Aucune migration en propre.** La table `teams__phase_drafts` est partagée,
discriminée par `phase = 'Dismissals'` ; la colonne `players_proj.membership` est
créée par la migration commune.

Une modification côté `players` :

| Méthode | Changement |
|---|---|
| `IPlayerRepository::apply(PlayerDismissed)` | pose `membership = 'Dismissed'` dans la projection, **dans la même transaction** que l'append |

## 2. Événements et listeners

### Domain events de `teams`

| Événement | Émis par | Mouvement de trésorerie |
|---|---|---|
| `PlayerDismissed` | `Team::dismiss_player` | **aucun** — règle 27 |
| `StaffDismissed` | `Team::dismiss_staff`, sans `refund_kpo` | **aucun** — règle 32 |
| `DismissalsPhaseValidated` | `Team::validate_dismissals_phase` | aucun |

Ces trois `None` sont **imposés par le compilateur** via le `match` exhaustif de
`treasury_movement()`. C'est la traduction vérifiée de « un renvoi ne rembourse rien ».

### App events

| App event | Origine | Consommateur |
|---|---|---|
| `PlayerDismissed` | publisher `teams` | listener `players` → domain event `PlayerDismissed` → `membership = 'Dismissed'` |

### Le second recalcul de valeur d'équipe

Décision de la phase 6 : le listener de TV (carte 251) gagne un déclencheur
supplémentaire, la réception de l'app event `PlayerDismissed` côté `teams`.

Sans lui, la TV serait recalculée sur `DismissalsPhaseValidated` (bus interne)
potentiellement **avant** que `players` ait traité la sortie d'effectif (app event
bus), donc en comptant encore les renvoyés. `TeamValueRecomputed` portant une valeur
absolue, le second recalcul est idempotent et le dernier gagne.

**À porter dans la carte 251.**

## 3. Handlers

```rust
// io/web/dismissals.rs
pub async fn dismissals_page(Path(..), State(..)) -> Result<impl IntoResponse, AppError>

// io/web/widgets/dismissals_roster_widget.rs
pub async fn dismissals_roster(Path(..), State(..)) -> …                 // GET
pub async fn mark_player(Path(..), State(..), Form(MarkPlayerBody))      // POST
pub async fn mark_staff(Path(..), State(..), Form(MarkStaffBody))        // POST

// io/web/widgets/dismissals_cart_widget.rs
pub async fn dismissals_cart(Path(..), State(..)) -> …                   // GET
pub async fn unmark_line(Path(..), State(..), Form(RemoveLineBody))      // POST
```

Routes en `mark` / `unmark` et non `add` / `remove` : sur une page de renvois,
`players/add` se lirait « ajouter un joueur à l'équipe ».

Une erreur de plus à traduire : `EligibleFloorReached` → 422 avec le message
« Minimum 11 joueurs éligibles ». En pratique elle ne devrait pas survenir, le bouton
étant déjà désactivé — sauf en cas de version périmée, où elle fait office de
deuxième barrière.

## 4. Templates

| Template | VM |
|---|---|
| `templates/dismissals.html` | page d'assemblage |
| `templates/widgets/dismissals-roster.html` | `DismissalsRosterVm` — trois états par ligne |
| `templates/widgets/dismissals-cart.html` | `DismissalsCartVm` |
| `templates/widgets/draft-error.html` | **partagé** avec le recrutement |

## 5. Tests e2e prévus

Fichier `tests/e2e/test_dismissals_phase.py`, à déclarer dans `tests/impact-map.toml`
— mêmes BCs traversés qu'au recrutement.

| # | Scénario |
|---|---|
| 1 | « Gérer les renvois → » ouvre la page ; l'effectif est listé avec SPP, valeur et disponibilité |
| 2 | Marquer un joueur : la ligne se barre, le bouton devient « Annuler », le panier s'incrémente |
| 3 | Annuler depuis la ligne **et** depuis le panier : les deux chemins fonctionnent |
| 4 | **12 éligibles → un renvoi passe ; à 11, tous les disponibles affichent « Minimum 11 »** |
| 5 | À 11 éligibles, un joueur **absent** reste renvoyable |
| 6 | Valider : les joueurs disparaissent de l'effectif, la trésorerie est **inchangée** |
| 7 | Le grand livre de trésorerie ne gagne **aucune ligne** |
| 8 | Après validation, l'équipe est prête à jouer et la valeur d'équipe **exclut les renvoyés** |
| 9 | Le numéro de maillot libéré est réattribué au recrutement de la séquence suivante |
| 10 | Le brouillon survit à un aller-retour sur la fiche équipe |
| 11 | Mobile 390px : panier fixe repliable, avertissement en version courte |

Les scénarios 4 et 5 couvrent le plancher, seule vraie subtilité de la page. Le 8 est
le test de non-régression de la course décrite au §2 : sans le second recalcul, il
échoue de façon intermittente.

Le 7 mérite d'exister alors qu'il vérifie une absence : c'est le seul test qui prouve
que « un renvoi ne rembourse rien » tient de bout en bout, du domaine au grand livre.

## 6. Ce qui n'est pas testé ici

La purge des brouillons (D6) est déclenchée par les quatre entrées en `ReadyToPlay` et
concerne les deux phases. Son test appartient à la carte qui l'implémente, pas à l'une
des deux pages.
