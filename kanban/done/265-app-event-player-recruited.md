# App event `PlayerRecruited` — créer le joueur dans `players`

**Priorité : haute**
**Dépend de :** 261
**Bloque :** 266
**Spec :** `docs/specs/phases-recrutement-renvois/recrutement/05-use-cases.md` §5-6
**Fichiers :** `src/app/teams/io/app_events/app_event_publisher.rs`,
`src/app/shared_kernel/app_events/`,
`src/app/players/io/app_events/player_recruited_listener.rs` (nouveau),
`src/app/players/io/app_events/team_created_listener.rs`

## Problème

Recruter dans `teams` **ne crée aucun joueur**. `PlayerRecruited` est un événement de
`teams` ; l'entité joueur vit dans `players`. Sans propagation, le coach paie un joueur
qui n'existe nulle part.

Le chemin symétrique existe déjà à la création d'équipe : `TeamCreated` traverse le bus
et `players/io/app_events/team_created_listener.rs` crée les joueurs, résout leurs
compétences de base, calcule leur valeur de départ et attribue les maillots.

## Action

### 1. Le flux

```
Lot appliqué (teams)
   └─► domain event PlayerRecruited        (event store teams)
         └─► publisher teams (couche IO)
               └─► app event PlayerRecruited
                     └─► listener players
                           └─► domain event PlayerCreated
                                 └─► players_proj
```

L'émission passe **obligatoirement par le publisher** : ni le use case ni le handler
n'accèdent à l'`app_event_bus`.

### 2. Le numéro de maillot est attribué par `players`

`PlayerRecruited` ne porte que `position_id`, `base_value_kpo` et `cost_kpo` :
**`teams` ne connaît pas les numéros de maillot**, et n'a aucune raison de les
connaître.

C'est `players` qui les possède (`Player.jersey`) et qui attribue **le premier numéro
disponible** à la réception.

Deux bénéfices : le port de `teams` n'a pas à les transporter, et deux recrutements
d'un même lot ne peuvent pas réserver le même numéro — le listener les traite
séquentiellement, chacun voyant l'état laissé par le précédent.

Un numéro libéré par un renvoi redevient disponible d'office, puisque le joueur
`Dismissed` n'est plus lu par `find_by_team_id` (carte 260).

### 3. Factoriser, ne pas dupliquer

Le nouveau listener refait ce que `team_created_listener` fait déjà : résolution des
compétences de base par ligne de roster, valeur de départ depuis le catalogue,
attribution du maillot.

**Extraire une fonction partagée** plutôt que de recopier. Le listener de création
traite N joueurs d'un payload, le nouveau en traite un — mais le corps est le même.

Attention au bug d'unité corrigé par la carte 249 : la valeur de départ est en **kPo**.

## Checklist

- [ ] App event déclaré dans `shared_kernel/app_events/`
- [ ] `to_app_event()` de `teams` mappe `PlayerRecruited`
- [ ] Aucune émission directe depuis un use case ou un handler
- [ ] Listener `players` en `init(app_event_bus: …)`
- [ ] Logique de création factorisée avec `team_created_listener`
- [ ] Premier maillot disponible attribué par `players`
- [ ] Test : 3 recrutements d'un même lot → 3 maillots distincts
- [ ] Test : un maillot libéré par un renvoi est réattribué
- [ ] `make check-arch` au vert, `make test` au vert
