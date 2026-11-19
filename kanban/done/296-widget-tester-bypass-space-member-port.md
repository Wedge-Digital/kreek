# `widget_tester_controller.rs` contourne le port ACL vers `spaces`

**Priorité : basse**
**Fichier :** `src/app/competitions/io/web/widget_tester_controller.rs:28-31`

## Problème

`get_competitions_widget_tester` (page de test dev, route `/competitions/widget/tester`) accède directement au contexte du BC `spaces` :

```rust
let spaces = state
    .spaces
    .space_repository
    .find_all()
    .await
    .unwrap_or_default()
    ...
```

Même violation de souveraineté que la carte 277 (`resultats_view.rs`), découverte lors de la recherche exhaustive demandée par cette carte. Contrairement à 277, ce cas ne peut pas être corrigé en réutilisant `space_member_port` tel quel : ce port n'expose que `find_member_profile`, pas de méthode pour lister tous les espaces. Traité en carte séparée pour cette raison.

## Action

Étendre `ICompetitionSpaceMemberPort` (`src/app/competitions/ports.rs`) avec une méthode de listing (ex. `find_all(&self) -> Vec<SpaceDto>` ou équivalent), l'implémenter dans `src/infrastructure/competitions/space_member_adapter.rs` en délégant à `ISpaceRepository::find_all`, puis faire pointer `get_competitions_widget_tester` dessus au lieu de `state.spaces` directement.

Alternative à trancher en démarrant la carte : cette page est un outil de test dev (comme `/kreek-select-tester`, déjà exempté de certaines règles de template) — vérifier si un contournement documenté est plus approprié qu'un port dédié pour un usage aussi marginal, avant d'étendre le port pour un seul appelant.

## Checklist

- [x] Décision prise : port étendu, ou exemption documentée pour cette page de test
- [x] Si port étendu : nouvelle méthode sur `ICompetitionSpaceMemberPort` + implémentation dans `space_member_adapter.rs`
- [x] `get_competitions_widget_tester` n'accède plus à `state.spaces` directement
- [x] `make check-arch` passe

---

## Notes d'implémentation

**Décision : port étendu**, et non exemption documentée. La carte laissait le
choix, mais l'axe 3 est **bloquant** : une exemption aurait dû être encodée dans
`check-arch.sh`, c'est-à-dire percer un trou nominatif dans un garde-fou global
pour une page de test. Le port coûtait dix lignes, l'adapter détenant déjà
l'`Arc<dyn ISpaceRepository>`.

`find_all_spaces()` retourne des `SpaceDefinition` — un type d'identité partagé,
déjà connu des deux côtés — plutôt qu'un DTO de plus. Le nom du port parle
d'appartenance et cette méthode n'en relève pas : dette de nommage assumée,
préférée à un second port pour un appelant unique.

**Les deux `expect("")` ont disparu.** C'étaient des panics sans message, sur des
données venant de la base. Les espaces dont l'identifiant ou le nom sont refusés
par leur value object sont désormais **écartés** : faire tomber tout le
sélecteur sur une ligne douteuse rendait les autres widgets intestables.

**Une jumelle exacte existe côté `teams`** (`team_selection_tester.rs`), révélée
par la carte 297. Elle n'a pas pu être corrigée ici : `teams` n'a aucun port
vers `spaces`, il faudrait en créer un de toutes pièces. Traitée en carte 301.
