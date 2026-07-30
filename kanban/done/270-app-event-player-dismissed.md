# App event `PlayerDismissed` et second recalcul de valeur d'équipe

**Priorité : haute**
**Dépend de :** 251, 260, 261
**Bloque :** 271
**Spec :** `docs/specs/phases-recrutement-renvois/renvois/06-domaine.md` §4,
`07-integration.md` §2
**Fichiers :** `src/app/teams/io/app_events/app_event_publisher.rs`,
`src/app/shared_kernel/app_events/`,
`src/app/players/io/app_events/player_dismissed_listener.rs` (nouveau),
`src/app/teams/io/listeners/team_value_listener.rs` (carte 251)

## Problème

Renvoyer dans `teams` ne retire le joueur de nulle part : l'entité vit dans `players`.
Sans propagation, un joueur renvoyé reste `Active`, continue d'être compté, et le
renvoi n'a aucun effet réel.

Et il y a une **course**.

## Action

### 1. Le flux

```
Lot appliqué (teams)
   └─► domain event PlayerDismissed        (event store teams)
         └─► publisher teams (couche IO)
               └─► app event PlayerDismissed
                     └─► listener players
                           └─► domain event PlayerDismissed
                                 └─► players_proj.membership = 'Dismissed'
```

Émission **obligatoirement par le publisher** — ni use case ni handler n'accèdent à
l'`app_event_bus`.

Le même nom des deux côtés ne contrevient pas au CLAUDE.md : la règle interdit de
nommer un domain event d'après son origine externe, pas l'homonymie du même fait.

### 2. La course, et pourquoi elle compte

Le recalcul de valeur d'équipe (carte 251) est déclenché par
`DismissalsPhaseValidated` sur le **bus interne** de `teams`. La sortie d'effectif
transite par l'**app event bus** vers `players`. Rien ne garantit que `players` ait
traité le renvoi quand `teams` recalcule.

**Conséquence : la valeur d'équipe peut être recalculée en comptant encore les
renvoyés.**

### 3. Ce qui est retenu — un second recalcul

Le listener de valeur d'équipe gagne un **déclencheur supplémentaire** : la réception
de l'app event `PlayerDismissed` côté `teams`.

- `TeamValueRecomputed` porte une valeur **absolue** : un recalcul de plus est
  inoffensif, le dernier gagne
- l'opération est idempotente et peu coûteuse

**Écarté** : faire dépendre le recalcul d'un accusé de traitement de `players` —
couplage fort entre deux BCs pour un simple problème de fraîcheur.

### 4. Le numéro de maillot se libère seul

Un joueur `Dismissed` n'est plus lu par `find_by_team_id` (carte 260), donc son numéro
cesse d'être occupé. Aucun traitement spécifique.

L'ordre des phases fait qu'il ne sera réutilisable qu'à la **séquence suivante** —
recrutement puis renvois — ce qui est cohérent avec « on ne libère pas une place pour
recruter dans la même séquence ».

## Checklist

- [ ] App event déclaré, `to_app_event()` le mappe
- [ ] Aucune émission directe depuis un use case ou un handler
- [ ] Listener `players` en `init(app_event_bus: …)` → `membership = 'Dismissed'`
- [ ] Projection mise à jour dans la même transaction que l'append
- [ ] Listener de TV abonné **aussi** à `PlayerDismissed`
- [ ] Test : deux recalculs successifs → même valeur finale (idempotence)
- [ ] Test : après renvoi, la valeur d'équipe exclut le joueur — **sans intermittence**
- [ ] `make check-arch` au vert, `make test` au vert
