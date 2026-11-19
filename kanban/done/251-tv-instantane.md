# TV — bascule en instantané et retrait de l'incrémental

**Priorité : haute**
**Dépend de :** 249, 250
**À livrer avec :** 253 (voir « Incohérence transitoire »)
**Fichiers :** `src/app/teams/domain/team.rs`,
`src/app/teams/io/repository/team_repository.rs`,
`src/app/teams/io/listeners/team_value_listener.rs` (nouveau),
`src/app/teams/use_cases/recompute_team_value_use_case.rs` (nouveau),
`src/app/teams/context.rs`, `src/main.rs`,
suppressions : `src/app/teams/io/app_events/player_improvement_listener.rs`,
`src/app/teams/io/app_events/tests/test_player_improvement_pipeline.rs`,
`src/app/shared_kernel/app_events/player_improvement_app_events.rs`,
`src/app/players/domain/events.rs`

## Problème

`Team.team_value` n'est jamais calculée : elle est **mutée par deltas** dans
`apply()` (`domain/team.rs:429-499`), sur sept événements — `PlayerRecruited`,
`StaffBought`, `StaffDismissed`, `PlayerImprovementApplied`, `PlayerFired`,
`PlayerNotReEngaged`, `PlayerValueAdjusted`.

Ce modèle ne peut pas porter la définition retenue. La disponibilité d'un joueur
vit dans `players` : quand un joueur passe `MissingNextGame`, **aucun événement
de `teams` ne se produit**, et le compteur n'a aucun moyen de le savoir. Un
modèle incrémental est structurellement incapable de suivre un état détenu
ailleurs.

Deuxième trou, indépendant : **`TeamCreated` ne pose aucune valeur**. L'app event
porte pourtant `players: Vec<PlayerPayload>`, que le listener de `teams` ignore.
Une équipe naît donc à `team_value = 0` (traité par la carte 252).

### Décisions prises

- **Instantané seul** : plus aucune mutation incrémentale. La TV se fige pendant
  les cinq phases d'après-match et ne bouge qu'au retour en `ReadyToPlay`.
  Assumé — c'est aussi ce qui rend la valeur lue par `match_report` exactement
  celle du moment où l'équipe s'est déclarée prête.
- **Recalcul à toute entrée en `ReadyToPlay`**, quel que soit le chemin, avec
  append systématique.
- **Déclenchement par un listener intra-BC**, pour que la règle soit écrite une
  seule fois.

## Action

### 1. Nouvel événement

```rust
TeamValueRecomputed { value: Kpo }
```

Valeur **absolue**, pas un delta. `apply()` écrase le champ :
`self.team_value = *value;`

Appendu systématiquement, même si la valeur n'a pas changé : l'event store
devient l'historique de progression de la TV sur la saison, information qu'on n'a
nulle part aujourd'hui.

### 2. Retirer les sept mutations d'`apply()`

Les bras d'événements restent — ils ont d'autres effets (trésorerie, compteurs de
staff) — seules les lignes touchant `self.team_value` disparaissent.

**Trois de ces sept événements ne sont en réalité jamais émis.** Aucune méthode
de `Team` ne construit `PlayerRecruited`, `PlayerFired` ni `PlayerNotReEngaged` :
l'agrégat n'expose que `enroll`, `dismiss`, `start/cancel_match_reporting`,
`start/revert_post_match_sequence`, `buy_staff`, `dismiss_staff`, les quatre
`validate_*_phase`, `apply_player_improvement` et `override_phase`. Recruter ou
licencier un joueur **n'existe pas** dans `teams`.

Conséquence : une fois la mutation de `team_value` retirée, les bras de
`PlayerFired` et `PlayerNotReEngaged` deviennent **vides** — ils ne faisaient que
ça. `PlayerRecruited` garde son effet sur la trésorerie.

À trancher au moment de coder : ces trois événements sont des jalons d'une
fonctionnalité non construite, pas des reliquats comme `PlayerValueAdjusted`. Les
supprimer par cohérence avec « on supprime le code mort », ou les garder comme
contrat déjà écrit du futur recrutement — les deux se défendent, mais il faut
choisir explicitement plutôt que de laisser deux bras vides.

Les champs `base_value_kpo`, `value_kpo_at_firing` et `value_kpo_at_release`
**sont conservés** : ils voyagent sur des événements qui existent de toute façon
et documentent ce que valait le joueur à cet instant — une information non
reconstructible.

### 3. Supprimer le code devenu mort

Vérification faite, aucun de ces éléments n'a d'autre consommateur :

| À supprimer | Justification |
|---|---|
| `TeamDomainEvent::PlayerImprovementApplied` | son bras `apply()` fait une seule ligne : `team_value += value_delta` |
| `teams/io/app_events/player_improvement_listener.rs` | unique producteur de l'événement ci-dessus |
| `shared_kernel/app_events/player_improvement_app_events.rs` | plus aucun abonné |
| le mapping `to_app_event()` de `players/domain/events.rs:164-190` | ne produisait que cet app event |
| `teams/io/app_events/tests/test_player_improvement_pipeline.rs` | teste la chaîne supprimée |
| `TeamDomainEvent::PlayerValueAdjusted` | **jamais émis** — défini, bras `apply()` écrit, aucun producteur |

Le publisher de `players` **n'est pas supprimé** : c'est lui qui portera l'app
event de la carte 252.

### 4. Publier les domain events de `teams` sur le bus interne

`teams` n'a aujourd'hui **aucune publication interne** : ses six listeners sont
tous sur l'`app_event_bus`, et `TeamsContext` ne reçoit même pas l'`event_bus`
(pourtant instancié à `main.rs:137` et partagé par les autres BCs).

Publier depuis `TeamRepository::append()`, et non depuis chaque use case comme le
font `players` et `match_report`. Raison : deux des quatre chemins vers
`ReadyToPlay` passent par des listeners (`match_report_cancelled_listener`), pas
par des use cases. Publier au point d'append est le seul endroit qui les couvre
tous — c'est exactement la propriété « aucun chemin oublié » qu'on a cherchée.

**Deux conséquences à valider :** c'est une déviation du pattern des autres BCs,
et l'`event_log_feeder` (abonné à `event_bus`) se mettra à journaliser les
événements de `teams`, ce qu'il ne fait pas aujourd'hui. Les deux me semblent
souhaitables, mais ce sont des effets de bord, pas des non-événements.

### 5. Listener de recalcul

`src/app/teams/io/listeners/team_value_listener.rs`, signature
`init(event_bus: &EventBus, ...)` — c'est cette convention que `check-arch`
(axe 5) utilise pour reconnaître un listener intra-BC.

Réagit à `TeamEnrolled`, `DismissalsPhaseValidated`, `MatchReportingCancelled`,
`CostlyMistakesApplied` — les quatre événements dont `apply()` pose
`game_phase = ReadyToPlay`.

**Ignorer `TeamValueRecomputed`**, sous peine de boucle infinie : le recalcul
appende, l'append publie, le listener recevrait son propre événement.

### 6. Use case et projection

`recompute_team_value_use_case` : charge l'agrégat, appelle le domain service de
la carte 250, appende `TeamValueRecomputed`.

`team_repository.rs` : la mise à jour de `team_proj.team_value` se fait **dans la
même transaction** que l'append (règle des projections event-sourcées).

### 7. Cartes impactées

- **Carte 47** (`to_be_refined`, listener `PlayerValueChanged` →
  `PlayerValueAdjusted`) : à déplacer en `cancelled/`. Elle décrivait le
  producteur de l'événement qu'on supprime, et avait déjà été doublée par
  `player_improvement_listener`.
- **Carte 46** (`to_be_refined`, customisation admin) : ajouter une note. Son
  `GamePhaseOverridden` vers `ReadyToPlay` sera couvert d'office par le listener,
  à condition d'ajouter l'événement à la liste des déclencheurs.

## Incohérence transitoire avec la carte 253

Livrée seule, cette carte fait compter les indisponibles à zéro et ajoute des
journaliers que `match_report` ne créera pas — il compte encore l'effectif total
(`count_available_players`). La TV annoncerait une équipe renforcée qui n'existe
pas sur le terrain.

**Livrer 253 avant ou avec celle-ci.** La 253 est indépendante des autres, donc
c'est possible.

## Point ouvert

Pendant les cinq phases d'après-match, la fiche équipe affichera une TV figée
alors que le coach recrute et licencie. Rien n'est prévu dans cette carte pour le
signaler — un libellé « TV au dernier match » serait le minimum si ça gêne à
l'usage. À trancher avant de coder.

## Checklist

- [ ] `TeamValueRecomputed { value: Kpo }` défini, `apply()` écrase le champ
- [ ] Les sept mutations incrémentales retirées d'`apply()`
- [ ] Champs de valeur des payloads conservés
- [ ] Les six éléments du tableau « à supprimer » retirés, aucun consommateur oublié
- [ ] `TeamsContext` reçoit `event_bus`, `TeamRepository::append` publie
- [ ] `team_value_listener` en `init(event_bus: ...)`, ignore `TeamValueRecomputed`
- [ ] Les 4 événements déclencheurs couverts
- [ ] Projection mise à jour dans la même transaction que l'append
- [ ] Carte 47 déplacée en `cancelled/`, carte 46 annotée
- [ ] Test unitaire : rejeu d'un flux complet → `team_value` correcte
- [ ] E2E : après une séquence d'après-match, la TV reflète l'effectif réel
- [ ] `make check-arch` au vert, `make test` au vert
