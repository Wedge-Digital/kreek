# BC `references` — Table de valeur des améliorations (value_delta)

**Priorité : haute**
**Dépend de :** rien
**Contexte :** `references/domain` + données JSON

## Objectif

Ajouter la table officielle de valeur d'équipe (`value_delta`) associée à
chaque type d'amélioration achetée en SPP — distincte de la matrice de coût
SPP (`skill_cost.json`, déjà existante). Spec complète :
`docs/specs/player-spp-spending/README.md`.

---

## Conception

### Nouvelle donnée `assets/references/improvement_value.json`

```json
{
  "skill_primary_kpo": 20,
  "skill_secondary_kpo": 40,
  "stat_kpo": { "ma": 20, "st": 60, "ag": 30, "pa": 20, "av": 10 }
}
```

Table officielle fournie par l'utilisateur (remplace la table provisoire
non confirmée de la carte 36). Le mode (Choisie/Aléatoire) n'influence pas
cette valeur.

### Domaine (`references/domain/models.rs`)

```rust
pub struct ImprovementValueTable {
    pub skill_primary_kpo: u32,
    pub skill_secondary_kpo: u32,
    pub stat_kpo: StatValueKpo,   // struct { ma, st, ag, pa, av: u32 }
}
```

### Port (`references/domain/port.rs`)

```rust
fn improvement_value(&self) -> &ImprovementValueTable;
```

### Chargement (`InMemoryReferenceRepository::load()`)

Même patron que `skill_cost.json` — `include_str!`/lecture fichier +
désérialisation au démarrage, stocké à côté de `skill_cost_matrix`.

---

## Checklist

- [ ] `assets/references/improvement_value.json` créé avec la table officielle
- [ ] `ImprovementValueTable` + `StatValueKpo` dans `references/domain/models.rs`
- [ ] `IReferenceRepository::improvement_value()` + implémentation `InMemoryReferenceRepository`
- [ ] Test : chargement correct des 7 valeurs depuis le JSON
