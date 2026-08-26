# Panneau « Visibilité »

**Épic :** E14 · **Ordre :** 3 · **Dépend de :** 420
**Conception :** `docs/specs/modifier-une-competition/onglet-parametres/`
(`04-dtos.md`, `05-use-cases.md`)

## Objectif

Changer le mode d'accès d'une compétition et son mode de validation des
inscriptions. Le panneau le plus simple des cinq.

## Conception

### Le use case

```rust
pub struct UpdateVisibilitySettingsCommand {
    pub season_id: SeasonId,
    pub access_mode: AccessMode,
    pub requires_validation: RequiresValidation,
}
```

1. `find_invitations(&season_id)` → `SeasonNotFound`
2. remplacer les deux champs, **conserver `invited_coaches`,
   `registration_deadline` et `max_participants`**
3. `find_notifications(&season_id)`
4. `save_invitations(&season_id, &invitations, &notifications)`

**Le seul use case qui relit deux documents.** La signature de
`save_invitations` porte aussi les notifications — héritage de l'étape 4 du
magicien, où les deux se règlent ensemble. Ne pas les relire les remettrait à
leur valeur par défaut, et **les rappels d'échéance s'éteindraient sans que rien
ne le dise**.

`max_participants` traverse sans être touché : le panneau ne l'édite pas (il ne
règle rien, cf. carte 415), mais le réécrire à zéro éteindrait la ligne « il
reste N places » des relances.

### Le handler

```rust
GET  …/settings/visibility  → get_settings_visibility
POST …/settings/visibility  → post_settings_visibility   (axum::Form)
```

```rust
#[derive(Deserialize)]
pub struct VisibilitySettingsForm {
    pub access_mode: String,             // "invitation" | "open"
    pub requires_validation: String,     // "manual" | "automatic"
}
```

Deux chaînes et non deux booléens : ce sont des `<select>` à deux options, et un
libellé futur — « sur candidature » — ne doit pas exiger de changer le type.

**Une valeur inconnue est un `400`**, jamais un repli silencieux sur le défaut,
qui ouvrirait une compétition fermée.

### Le VM

```rust
pub struct VisibilityVm {
    pub access_mode: String,
    pub requires_validation: bool,
    pub invited_count: u32,      // affichage seul
}
```

`invited_count` existe pour une raison : le panneau réécrit `invitations`, dont
`invited_coaches` fait partie. Afficher « 12 coachs invités » rend visible ce que
le POST doit préserver — la relecture cesse d'être une précaution invisible.

## Tests

- Unitaires : les trois champs préservés, les notifications préservées, le refus
  d'une valeur d'énumération inconnue.
- E2E : basculer de « sur invitation » à « ouvert » et vérifier que les coachs
  invités sont toujours là.

## Checklist

- [ ] Le use case et ses tests
- [ ] Les deux handlers, `require_admin_access`
- [ ] Le VM, le template
- [ ] `make lint && make test && make check-arch`
