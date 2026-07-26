# Départages — ACL du catalogue vers `competitions` (port + adapter)

**Priorité : haute**
**Dépend de :** carte 208 (catalogue dans `ranking`)
**Contexte :** `src/app/competitions/ports.rs`, `src/infrastructure/competitions/`, `src/app/competitions/context.rs`, `src/main.rs`
**Spec :** `docs/specs/ranking/tiebreakers/competition-rules-form/{03-back,04-dtos}.md`

## Objectif

Rendre le catalogue de `ranking` consultable par `competitions` via un port. **Purement
additif** : le port est injecté mais encore consommé par personne. Commit intermédiaire
sûr.

## Conception (cf. `03-back.md`, `04-dtos.md`)

### `app/competitions/ports.rs` — port + DTO

```rust
pub struct TiebreakCriterionDto {
    pub code:  String,
    pub label: String,
}

pub trait ITiebreakCatalogPort: Send + Sync {
    /// Catalogue complet, dans l'ordre canonique.
    fn all(&self) -> Vec<TiebreakCriterionDto>;
}
```

Port **synchrone** (le catalogue est statique, aucun IO) et primitives acceptées : c'est
un DTO de lecture.

### `infrastructure/competitions/tiebreak_catalog_adapter.rs` — nouveau

Implémente le port en appelant `ranking::domain::tiebreak` + `tiebreak_labels`. **Sans
état** — pas de repository à injecter, contrairement à `SkillCatalogAdapter`
(`infrastructure/players/skill_catalog_adapter.rs`), pris comme modèle pour le reste.

C'est le **seul** fichier autorisé à importer `ranking` depuis ce chemin : le BC
`competitions` ne l'importe jamais directement.

### Injection

- `CompetitionsContext` : champ `tiebreak_catalog_port: Arc<dyn ITiebreakCatalogPort>`,
  ajouté aux paramètres de `new()`.
- `main.rs` : instancie l'adapter et le passe à `CompetitionsContext::new`.

Attention : `CompetitionsContext::new` a d'autres appelants potentiels (tests) — les
compléter.

## Checklist

- [ ] `TiebreakCriterionDto` + `ITiebreakCatalogPort` dans `competitions/ports.rs`
- [ ] Adapter dans `infrastructure/competitions/`, seul importeur de `ranking`
- [ ] Champ ajouté à `CompetitionsContext` + injection dans `main.rs`
- [ ] Aucun import de `ranking` dans `app/competitions/` (vérifié par grep)
- [ ] Test de l'adapter : 7 entrées, codes stables, ordre canonique, libellé non vide
- [ ] `make test` + `make check-arch` passent
