# L'écran du jet · Phase 7 : effets de bord

**Entrée** : `06-domaine.md` validé. Conception, pas implémentation.

## 1. Persistance — aucune migration

L'event store sérialise les événements en JSON ; `CostlyMistakesApplied` y gagne
`damage_dice` **sans changer de schéma**, avec son `#[serde(default)]` pour que
d'éventuels événements existants se relisent en liste vide.

La projection `teams_proj` ne porte **rien de nouveau** : la phase y est déjà
stockée comme une chaîne, `CostlyMistakes` en est une de plus. Et la ligne du
grand livre s'écrit toute seule — `TreasuryMovement` est déjà branché sur cet
événement, avec le motif `CostlyMistake`, **dans la même transaction que
l'append**.

Aucune méthode de repository nouvelle.

## 2. Événements — ce qui se déclenche tout seul

`CostlyMistakesApplied` est **déjà** dans la liste des quatre événements que
`team_value_listener::ends_in_ready_to_play` surveille, et dans celle du
`phase_basket_purge_listener`. La valeur d'équipe sera donc recalculée et les
paniers purgés sans qu'on touche à un listener.

C'est le bénéfice d'un événement défini avant son producteur : tout l'aval était
déjà câblé, et attendait.

`CostlyMistakesPhaseStarted`, lui, ne déclenche rien — il ne fait que changer de
phase. **À vérifier au moment de le poser** : les deux listeners réagissent à des
listes explicites d'événements, donc l'ajout d'un variant ne les perturbe pas.

## 3. Handlers

```rust
// GET — la page
pub async fn costly_mistakes_page(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse
```

Il vérifie la phase comme le fait déjà `dismissals.rs:70` — `game_phase !=
Some(CostlyMistakes)` rend un **422**, et l'écran ne s'affiche pas. Même geste,
même code de retour : cette famille de pages n'a pas de sens hors de sa phase.

```rust
// POST — le jet
pub async fn post_roll_costly_mistakes(
    auth_session: AuthSession,
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse
```

Pas de `Form`, pas de `Query` : **le POST n'a pas de corps**.

| Étape | Échec |
|---|---|
| session | 401 |
| équipe introuvable | 404 |
| droit — propriétaire ou admin (`ITeamAccessPort`) | 403 |
| use case → `Domain` (mauvaise phase, second jet) | **409** |
| use case → `Repository` | 500 |
| succès | fragment de résultat |

Le **409 et non 422** : la requête est bien formée, c'est l'état qui a changé.
`edit_match_report` répond déjà ainsi sur un rapport publié.

## 4. Templates

| Template | Rôle |
|---|---|
| `teams-costly-mistakes.html` | la page : en-tête, table, zone de jet |
| `teams-costly-mistakes-result.html` | le fragment : verdict, calcul, table avec la case touchée |

Le fragment **ne répète pas l'`id` de son conteneur** — le `CLAUDE.md` le
rappelle pour les injections `innerHTML`, et c'est exactement ce montage.

Le CSS rejoint `assets/static/css/pages/costly-mistakes.css`, **inscrit au
bundle** (`css_bundle.rs`) et scopé sous la racine `.cm-page` : l'axe 14 de
`check-arch` refuse toute feuille absente de la liste.

L'animation vit dans un `x-data` Alpine, avec sa durée plancher. Pas de
`<script>` nu, pas d'`id` global — conventions 6 et 7.

## 5. Le bandeau de la fiche d'équipe

```rust
(Enrolled, Some(CostlyMistakes)) => Some(BannerVm { … ctas: vec![Navigate { … }] })
```

Une branche de plus dans `BannerVm::from_domain`, avec le type de CTA existant.
Rien d'autre : la fiche d'équipe ne connaît pas le contenu de la phase, elle
sait seulement y mener.

## 6. Tests E2E prévus

| Scénario | Ce qu'il vérifie |
|---|---|
| Renvois validés à **99 kPo** → l'équipe est directement prête à jouer, aucun écran | R1 |
| Renvois validés à **150 kPo** → le bandeau propose « Lancer le dé » | R1, accès |
| Lancer → le résultat s'affiche, la trésorerie de la fiche a baissé du montant annoncé | chemin nominal |
| Relancer (bouton désactivé contourné) → **409**, la trésorerie n'a pas rebougé | R6 |
| Un coach tiers ouvre l'URL du jet → **403**, aucun événement | R10 |
| Ouvrir la page hors phase → 422 | garde de page |

**Le quatrième est le seul qui compte vraiment.** Un double jet retirerait de
l'argent deux fois, et c'est le genre de défaut qu'un utilisateur découvre avant
nous. Il se teste en postant deux fois, sans passer par le bouton.

Le premier vérifie une **absence d'écran** — seul un test de bout en bout peut le
voir.

## 7. Ce qui reste hors périmètre

- **L'onglet Trésorerie** (carte 48), qui rendra le mouvement consultable après
  coup. La ligne existe déjà au grand livre ; il manque l'écran.
- **La retraite temporaire** (carte 39), toujours hors du chemin.
- **La consultation du jet passé**, écartée en phase 2.
