# BC `references` — Barème SPP par type d'action

**Priorité : haute**
**Dépend de :** rien
**Contexte :** `references` — donnée de règle fixe, pas de nouveau fichier JSON

## Objectif

Exposer le barème SPP (Blood Bowl standard) par type d'action, pour que BC `players`
puisse résoudre le SPP gagné par une action de match sans jamais le calculer/coder
en dur lui-même. Fondation de la feature « player report events »
(`docs/specs/player-match-impact/`).

---

## Conception

### Méthodes ajoutées à `IReferenceRepository`

Fichier : `src/app/references/domain/port.rs`

```rust
pub trait IReferenceRepository: Send + Sync {
    // ... méthodes existantes inchangées ...

    fn touchdown_spp(&self) -> u8;
    fn pass_spp(&self) -> u8;
    fn interception_spp(&self) -> u8;
    fn casualty_spp(&self) -> u8;
    fn mvp_spp(&self) -> u8;
}
```

Méthodes nommées explicitement par type d'action plutôt qu'une méthode générique
paramétrée — cohérent avec le style `find_x`/`list_x` déjà en place sur ce trait.

### Implémentation

Valeurs fixes retournées en dur dans l'implémentation in-memory du repository
(`src/app/references/io/repository/...`) — ce barème n'est ni roster-spécifique ni
compétition-spécifique, pas besoin d'entrée dans `assets/references/*.json`.

| Méthode | Valeur |
|---|---|
| `touchdown_spp()` | 3 |
| `pass_spp()` | 1 |
| `interception_spp()` | 2 |
| `casualty_spp()` | 2 |
| `mvp_spp()` | 4 |

Pas de valeur pour l'agression (aucun SPP, cf. carte 154) ni pour les blessures
(idem).

---

## Checklist

- [ ] 5 nouvelles méthodes sur `IReferenceRepository`
- [ ] Implémentation dans le repository in-memory existant
- [ ] Tests unitaires : chaque méthode retourne la valeur attendue
