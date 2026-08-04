# `resultats_view.rs` contourne le port ACL vers `spaces`

**Priorité : moyenne**
**Fichier :** `src/app/competitions/io/web/resultats_view.rs:74-81`

## Problème

`compute_authorization` accède directement au contexte du BC `spaces` :

```rust
let is_space_admin = matches!(
    state
        .spaces
        .space_repository
        .find_member_profile(&user.id, space_id)
        .await,
    Ok(Some(SpaceProfile::SpaceAdmin))
);
```

C'est une violation de la souveraineté des BCs (cf. CLAUDE.md « Adapters inter-BCs ») : `competitions` ne doit jamais lire `state.spaces` directement, il doit passer par un port qu'il définit lui-même. Ce port **existe déjà** et n'est utilisé nulle part : `ICompetitionSpaceMemberPort` / `space_member_port` (`src/app/competitions/ports.rs:46-52`, déclaré dans `context.rs:29`, adapter `src/infrastructure/competitions/space_member_adapter.rs`).

La violation échappe à `make check-arch` (axe 3, qui grep `state\.spaces\b` par ligne) uniquement parce que `rustfmt` a coupé l'appel sur plusieurs lignes (`state` puis `.spaces` sur la ligne suivante) — le regex mono-ligne ne matche pas. Le contrôle doit rester tel quel (pas de justification à en faire un faux positif), mais ce cas précis mérite d'être corrigé plutôt que contourné.

Découvert en préparant la feature « Accueil — derniers résultats » (`docs/specs/accueil-derniers-resultats/`), qui a besoin du même port pour sa propre vérification d'autorisation et ne doit pas reproduire ce contournement.

## Action

Remplacer l'appel direct par `state.competitions.space_member_port.find_member_profile(&user.id, space_id)` (déjà branché sur `space_repository` de `spaces` via l'adapter — comportement identique, juste conforme à l'architecture). Vérifier qu'aucun autre appel direct à `state.spaces` ne s'est glissé ailleurs dans `src/app/competitions/` de la même manière (recherche multi-lignes, pas seulement le grep mono-ligne de `check-arch`).

## Checklist

- [ ] `resultats_view.rs::compute_authorization` utilise `space_member_port` au lieu de `state.spaces.space_repository`
- [ ] Recherche exhaustive d'autres occurrences de `state.spaces` (ou tout `state.<autre_bc>`) coupées sur plusieurs lignes dans `src/app/competitions/`
- [ ] `make check-arch` passe
- [ ] `make test` passe (tests existants de `resultats_view.rs`)
