# BC `players`/`teams` — App event d'achat → `team_value`

**Priorité : moyenne**
**Dépend de :** `178-players-purchase-skill-use-case.md`, `179-players-increase-stat-use-case.md`
**Contexte :** `players/io/app_events` (nouveau, 1er publisher du BC) + `teams/io/app_events`

## Objectif

Informer `teams` de chaque achat pour que `team_value` reflète la valeur
ajoutée — via app event, jamais de lecture croisée. Construit enfin
`TeamDomainEvent::PlayerImprovementApplied`, déjà défini mais jamais
produit. Spec complète : `docs/specs/player-spp-spending/README.md`.

---

## Conception

### App event (`shared_kernel/app_events/player_improvement_app_events.rs`, nouveau)

```rust
pub enum PlayerImprovementAppEvent {
    Purchased { team_id: String, player_id: String, value_delta_po: u32 },
}
```
Nommé en termes domaine (le fait : une amélioration a été achetée), pas en
termes d'origine technique. `value_delta_po` explicitement suffixé `_po`
dans le payload pour éviter toute ambiguïté à la frontière (voir point
d'attention ci-dessous).

### ⚠️ Point d'attention critique — unités différentes entre BCs

`players::domain::player::ValueKpo` stocke en réalité des **Po bruts**
malgré son nom (`ValueKpo(100_000)` → affiché "100 000 Po") — c'est l'unité
de `PlayerSkillPurchased.value_delta`/`PlayerStatIncreased.value_delta`.

`teams::domain::value_objects::Kpo` stocke lui de vrais **kPo**
(`Kpo(873)` → affiché "873 kPo").

**Le listener `teams` doit diviser par 1000** avant de construire
`Kpo` à partir du `value_delta_po` reçu dans l'app event — sinon
`team_value` serait incrémentée 1000× trop. Toutes les valeurs de la table
officielle (10 000/20 000/30 000/40 000/60 000 Po) sont des multiples exacts
de 1000, donc la division est toujours entière, sans arrondi.

### Publisher (`players/io/app_events/app_event_publisher.rs`, nouveau)

Souscrit au bus interne `players`, transforme `PlayerSkillPurchased`/
`PlayerStatIncreased` (domain events) en `PlayerImprovementAppEvent::Purchased`
sur l'`app_event_bus`. Premier publisher du BC `players` — suit le patron
déjà utilisé par `match_report`/`competitions`.

### Listener (`teams/io/app_events/player_improvement_listener.rs`, nouveau)

```rust
pub fn init(app_event_bus: &EventBus, team_repo: Arc<dyn ITeamRepository>) {
    // souscrit PlayerImprovementAppEvent::Purchased
    // team_repo.find_by_id(team_id) → team.apply_player_improvement(value_delta) [nouvelle méthode domaine teams]
    //   → TeamDomainEvent::PlayerImprovementApplied { player_id, value_delta: Kpo, ... } (event déjà défini, apply() déjà géré ligne 412-414)
}
```

Note : l'event `TeamDomainEvent::PlayerImprovementApplied` a une forme
existante (`player_id`, `improvement: PlayerImprovement`, `value_delta`) —
vérifier si `improvement: PlayerImprovement` (enum `NewSkill(String)`/
`StatBoost(Stat)`, `teams/domain/value_objects.rs:81-93`) doit être simplifié
ou conservé tel quel pour cette carte (à trancher en ouvrant la carte).

---

## Checklist

- [ ] `PlayerImprovementAppEvent` dans `shared_kernel/app_events/`
- [ ] `players::io::app_events::app_event_publisher.rs` (nouveau, souscrit domain events, publie app event)
- [ ] `teams::io::app_events::player_improvement_listener.rs` — construit `PlayerImprovementApplied`
- [ ] Nouvelle méthode domaine `Team::apply_player_improvement(...)` si nécessaire pour respecter la forme existante de l'event
- [ ] Câblage dans `players::context.rs` (publisher) et `teams::context.rs` (listener)
- [ ] Test : achat côté players → team_value incrémenté côté teams (test d'intégration bus événementiel, cf. `test_player_match_impact_pipeline.rs` comme précédent)
