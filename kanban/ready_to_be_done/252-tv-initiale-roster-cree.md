# TV initiale — app event « roster initial créé »

**Priorité : haute**
**Dépend de :** 250, 251
**Fichiers :** `src/app/players/io/app_events/team_created_listener.rs`,
`src/app/players/domain/events.rs`,
`src/app/players/io/app_events/app_event_publisher.rs`,
`src/app/shared_kernel/app_events/` (nouvel app event),
`src/app/teams/io/app_events/` (nouveau listener), `src/app/teams/context.rs`

## Problème

Une équipe fraîchement créée affiche une TV de **0**, et jouerait son premier
match avec des coups de pouce calculés sur cette valeur.

Après la carte 251, le recalcul se déclenche à toute entrée en `ReadyToPlay`, et
`TeamEnrolled` en fait partie. Mais **ça ne suffit pas, à cause d'une course**.

`teams` et `players` s'abonnent au **même** app event
`TeamCreationAppEvent::TeamCreated`, dans deux `tokio::spawn` indépendants :

- `players/io/app_events/team_created_listener.rs` crée les N joueurs, un
  `PlayerCreated` puis un `InitialSkillEarned` par compétence, chacun dans sa
  transaction.
- `teams/io/app_events/team_created_listener.rs` appende `TeamCreated` puis, si
  `auto_enroll` est vrai, enchaîne sur `approve_enrollment` → `TeamEnrolled` →
  `ReadyToPlay`.

Rien ne garantit l'ordre. `teams` peut atteindre `ReadyToPlay` avant que
`players` ait inséré le moindre joueur — le port renverrait une liste vide et la
TV vaudrait 0 (ou une valeur partielle, pire encore parce que plausible).

`auto_enroll` vaut vrai dès que la saison ne demande pas de validation
(`finalize_team.rs:96-105`) : c'est un cas normal, pas une exception.

### Pourquoi un app event et pas le payload

`TeamCreated` porte déjà `players: Vec<PlayerPayload>` avec les lignes de roster
et les compétences initiales. `teams` pourrait donc valoriser lui-même, sans
appeler `players` ni attendre personne.

**Écarté délibérément** : `teams` réimplémenterait « coût de la position + deltas
des compétences », qui est la règle de `players`. C'est exactement la duplication
qui a produit les deux tables divergentes corrigées par la carte 249. `players`
reste seul propriétaire de la valeur d'un joueur.

## Action

### 1. Domain event côté `players`

Le listener de `players` émet, **une fois sa boucle terminée**, un domain event
signalant que le roster initial de l'équipe est complet.

Entorse assumée : aucun agrégat de `players` ne porte le roster — la création
initiale est une boucle sur N joueurs dans un listener, pas une opération
d'agrégat. C'est la seule concession de la série, et elle est préférable aux deux
alternatives (course non résolue, ou duplication de la règle de valorisation).

Nommer en termes de fait domaine, pas d'origine externe :
`InitialRosterCompleted { team_id, player_count }` — surtout pas
`TeamCreatedReceived`.

### 2. App event et publisher

Le publisher de `players` (`app_event_publisher.rs`) convertit ce domain event en
app event via `to_app_event()`. C'est ce qui le maintient en vie après la
suppression de `PlayerImprovementAppEvent` par la carte 251.

Rappel du flux obligatoire : le listener **n'émet jamais** l'app event
directement — il émet le domain event, le publisher convertit.

### 3. Listener côté `teams`

Nouveau listener sur l'`app_event_bus`, signature
`init(app_event_bus: &EventBus, ...)` — c'est ce qui le distingue d'un listener
intra-BC pour l'axe 5 de `check-arch`, et qui l'exempte de la règle de
transaction unique (l'événement vient d'un commit distant).

Il appelle le même `recompute_team_value_use_case` que le listener intra-BC de la
carte 251.

### 4. Convergence

Si `TeamEnrolled` arrive avant l'app event du roster, la TV est recalculée deux
fois : une première à 0 (ou partielle), une seconde à la bonne valeur. Les deux
appends sont légitimes et l'historique reste lisible — `TeamValueRecomputed`
porte une valeur absolue, la dernière gagne.

Aucun ordre n'est donc à garantir, ce qui est le but.

## Checklist

- [ ] `InitialRosterCompleted` émis en fin de boucle du listener de `players`
- [ ] Nommage en termes de fait domaine, pas d'origine externe
- [ ] `to_app_event()` mappe le domain event, le publisher publie
- [ ] Le listener de `players` n'émet aucun app event directement
- [ ] Listener côté `teams` en `init(app_event_bus: ...)`
- [ ] Il réutilise `recompute_team_value_use_case`, sans logique dupliquée
- [ ] Test unitaire : la TV initiale somme joueurs + staff + relances, sans Facteur Fans
- [ ] E2E : une équipe fraîchement créée affiche une TV non nulle et exacte
- [ ] E2E : idem avec `auto_enroll` actif (le cas où la course se produit)
- [ ] `make check-arch` au vert, `make test` au vert
