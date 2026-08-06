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

- [ ] Décision prise : port étendu, ou exemption documentée pour cette page de test
- [ ] Si port étendu : nouvelle méthode sur `ICompetitionSpaceMemberPort` + implémentation dans `space_member_adapter.rs`
- [ ] `get_competitions_widget_tester` n'accède plus à `state.spaces` directement
- [ ] `make check-arch` passe
