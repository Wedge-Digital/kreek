# BC `teams` — Listener app event `PlayerValueChanged` → domain event `PlayerValueAdjusted`

**Priorité : moyenne**
**Dépend de :** `29-teams-repository.md`, `42-teams-projection.md`, BC `players` (non encore créé)
**Contexte :** `teams` — couche IO (listener app event)

## Objectif

Recevoir l'app event `PlayerValueChanged` du BC `players` dans la **couche IO**, et appendre le domain event `PlayerValueAdjusted` dans l'event store de BC `teams` pour mettre à jour la TV.

Le domaine `teams` ignore que cet event vient de `players` — il ne connaît que `PlayerValueAdjusted`.

---

## Pourquoi cet event et pas une projection locale

BC `teams` ne consomme aucune donnée SQL de BC `players`. Mais la TV est un état de l'équipe qui peut changer suite à des événements extérieurs. La consommation d'un app event pour une **transition d'état métier** est explicitement autorisée par la règle de souveraineté des données (CLAUDE.md).

---

## App event attendu de BC `players`

```rust
// Publié par BC players sur l'app event bus
PlayerValueChanged {
    event_id:  String,
    player_id: String,
    team_id:   String,
    delta_kpo: i32,   // positif = augmentation, négatif = diminution
}
```

## Traitement dans BC `teams`

Le listener traduit l'app event en domain event interne et l'appende dans l'event store :

```rust
// io/app_events/player_value_changed_listener.rs
// Reçoit PlayerValueChanged →
//   1. Construit TeamDomainEvent::PlayerValueUpdated { player_id, delta_kpo }
//   2. Charge la version courante de l'équipe (find_by_id ou version depuis projection)
//   3. team_repo.append(&team_id, &event, current_version)
//   → Team::apply() met à jour self.team_value
//   → update_projection_in_tx() met à jour team_value_kpo
```

### Optimisation : version sans rejeu complet

Pour ce listener, rejouer tous les événements juste pour obtenir la `version` courante est inutile. On peut lire la `version` directement depuis `teams_projection` :

```rust
let current_version = projection_repo.get_version(&team_id).await?;
team_repo.append(&team_id, &domain_event, current_version).await?;
```

Cela implique d'exposer `get_version()` sur le port, ou de l'intégrer dans `ITeamRepository`.

---

## Ce qui reste à définir

- Quels cas côté BC `players` déclenchent un `PlayerValueChanged` ? Uniquement les améliorations via SPP, ou aussi d'autres événements (blessures qui réduisent la valeur ?) ?
- Comment BC `players` connaît-il le `team_id` d'un joueur ? Il faut que BC `players` maintienne cette association.
- Faut-il un `get_version()` dédié sur `ITeamRepository`, ou lire depuis `teams_projection` suffit ?

---

## Checklist (à compléter après création de BC `players`)

- [ ] Payload `PlayerValueChanged` figé avec BC `players`
- [ ] `player_value_changed_listener::init()` dans BC `teams`
- [ ] `TeamDomainEvent::PlayerValueUpdated` déjà défini (carte 28)
- [ ] `update_projection_in_tx()` pour `PlayerValueUpdated` déjà défini (carte 42)
- [ ] `get_version()` sur `ITeamRepository` ou lecture depuis `teams_projection`
- [ ] Gestion idempotente (doublon d'event → `ConcurrentWrite` ignoré)
- [ ] Test d'intégration : app event → TV mise à jour en base

---

## Annulée — 2026-07-29 (carte 251)

Cette carte décrivait le producteur de `TeamDomainEvent::PlayerValueAdjusted`,
supprimé par la carte 251 : la TV n'est plus une accumulation de deltas mais une
somme recalculée, appendue en valeur absolue par `TeamValueRecomputed`.

Un listener qui ajusterait la TV joueur par joueur n'a donc plus d'objet — et il
n'aurait de toute façon pas su voir un joueur devenu indisponible, puisque cet
état vit dans `players` sans produire d'événement dans `teams`. C'est
précisément ce qui a motivé la bascule.
