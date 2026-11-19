# Architecture — Axe 5 : atomicité `pairing_projection_listener`

**Priorité : haute**
**Dépend de :** `184-arch-false-positives-and-axe5-scope.md` (pour que `make check-arch` isole bien cette violation des faux positifs)
**Contexte :** `competitions` — event sourcing intra-BC

## Objectif

`pairing_projection_listener.rs` met à jour la projection `competition_match_display_proj` (`insert_projection`/`delete_projection`) de façon **asynchrone**, en réagissant à `PairingCreated`/`PairingDeleted` sur le bus interne du BC. L'append de l'événement dans l'event store et la mise à jour de la projection s'exécutent dans deux transactions séparées : si le process crashe entre les deux, la projection se désynchronise silencieusement de l'event store.

Contrairement à `match_report_published_listener.rs` (carte 184 — cross-BC, par nature asynchrone), ici l'émetteur et le consommateur sont le **même BC** (`competitions`). La règle "même transaction" s'applique pleinement.

## Action

1. Localiser le/les use case(s) ou repository qui appendent `PairingCreated` et `PairingDeleted` dans l'event store de `competitions` (probablement `io/repository/*.rs`, méthode d'append avec `tx.commit()`).
2. Déplacer l'appel à `insert_projection`/`delete_projection` **dans cette même transaction**, juste après l'append de l'événement, avant le `commit()` — signature `&mut PgConnection` ou `&mut Transaction`, pas `&PgPool` (cf. règle "Projections event sourcing" du CLAUDE.md).
3. Supprimer `pairing_projection_listener.rs` comme point d'entrée asynchrone pour ces deux événements (ou le conserver s'il traite d'autres cas qui restent légitimement asynchrones — à vérifier lors de l'implémentation ; probable qu'il devienne obsolète et soit supprimé, avec vérification exhaustive des appelants avant suppression, cf. règle 4 du CLAUDE.md).
4. Adapter `init()` / le câblage dans `main.rs` en conséquence si le listener disparaît.

## Checklist

- [ ] `insert_projection`/`delete_projection` s'exécutent dans la même transaction que l'append de `PairingCreated`/`PairingDeleted`
- [ ] Signatures des fonctions de projection : `&mut PgConnection`/`&mut Transaction`, plus de `&PgPool`
- [ ] `pairing_projection_listener.rs` supprimé ou réduit à ce qui reste légitimement asynchrone (vérification exhaustive des consommateurs avant suppression)
- [ ] `main.rs` mis à jour si le listener est retiré
- [ ] Test unitaire ou d'intégration : append + projection échouent/réussissent ensemble (atomicité)
- [ ] `make check-arch` : axe 5 passe entièrement
- [ ] `cargo test` + `make e2e` passent
