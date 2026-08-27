# La chaîne d'événements du roster supprimé

**Épic :** E10 · **Ordre :** 3 · **Dépend de :** 443
**Conception :** `docs/specs/roster-personnalise/editeur-de-roster/`
(`03-back.md`, `07-integration.md`)

## Objectif

Qu'un roster supprimé disparaisse aussi des tiers de compétition qui le citent.
Sans quoi un uid mort y reste, et le sélecteur de création d'équipe le laisse
tomber en silence.

## La chaîne, de bout en bout

```
delete_custom_roster_use_case                                    (carte 443)
    │  emettre()   ReferencesDomainEvent::CustomRosterDeleted
    ▼
references/io/app_events/app_event_publisher.rs                  ← à créer
    │  publier()   ReferencesAppEvent::CustomRosterDeleted { roster_uid }
    ▼
competitions/io/app_events/custom_roster_deleted_listener.rs     ← à créer
    │
    ▼  retire l'uid des tiers de toutes les saisons
```

**Rien de tout cela n'existe.** `references` n'a jamais publié.

| À créer | Où |
|---|---|
| `ReferencesDomainEvent` | `references/domain/domain_event.rs` |
| Le publisher | `references/io/app_events/app_event_publisher.rs` |
| `ReferencesAppEvent` et `to_app_event()` | `shared_kernel/app_events/references_app_events.rs` |
| Le listener | `competitions/io/app_events/custom_roster_deleted_listener.rs` |
| Le câblage | `main.rs`, à côté des cinq autres |

## Le publisher se copie, il ne se réécrit pas

`competitions/io/app_events/app_event_publisher.rs` est le modèle :
`spawn_listener`, désérialisation du domain event, `to_app_event()`, puis
`publier()` sous un span `app_event_publication` qui porte
`cause = %envelope.event_id`.

**Copier-coller** — règle 5 du `CLAUDE.md`. Le span n'est pas décoratif : c'est
lui qui permet de suivre un `grep <event_id>` du domain event jusqu'à toutes les
réactions, tous BCs confondus.

## Le listener

```rust
pub fn init(app_event_bus: &EventBus, pool: PgPool);
```

**`init(app_event_bus: …)` et non `event_bus`** : c'est la convention que l'axe 5
de `check-arch` reconnaît pour un listener **cross-BC**, exempté de la règle de
transaction unique. Un événement venu d'un autre BC est déjà committé ailleurs ;
partager sa transaction est impossible par construction.

Il parcourt les saisons, retire l'uid de chaque `tiers[].rosters`, et réécrit.

### Il journalise son passage

Combien de saisons parcourues, combien de tiers modifiés.

**Un listener silencieux qui échoue laisse une incohérence que rien ne
raconte** — et cette incohérence est précisément celle que le `filter_map` de
`builders.rs` avale sans un mot (carte 438). Les deux se répondent : le listener
dit ce qu'il a fait, le `filter_map` dira ce qu'il écarte.

### Il est idempotent

Rejoué sur une saison déjà nettoyée, il ne trouve rien et ne réécrit pas. Le bus
peut redélivrer ; un listener qui compterait sur l'unicité serait faux.

## Ce que la carte ne fait pas

- **Elle ne nettoie pas les brouillons.** `team_drafts.creation_rules` porte une
  **copie figée** de la liste des rosters d'un tier ; un uid mort y est sans
  effet, et le brouillon se refait.
- **Elle ne rattrape pas l'existant** : aucun roster personnalisé n'existe encore
  au moment où elle est livrée.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `le_publisher_convertit_le_domain_event` | la conversion |
| `le_listener_retire_l_uid_des_tiers` | intégration, vraie base |
| `le_listener_est_idempotent` | rejoué, il ne casse rien |
| `un_tier_sans_cet_uid_n_est_pas_reecrit` | pas d'écriture inutile |
| `le_listener_journalise_son_passage` | le compte de saisons et de tiers |

## Checklist

- [ ] `ReferencesDomainEvent` et son `to_app_event()`
- [ ] Le publisher, copié sur celui de `competitions`
- [ ] `ReferencesAppEvent` dans le `shared_kernel`
- [ ] Le listener, `init(app_event_bus: …)`, idempotent et journalisé
- [ ] Le câblage dans `main.rs`
- [ ] Les cinq tests
- [ ] `make lint && make test && make check-arch`
