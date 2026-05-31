# Câblage du cartouche "Règles de la compétition" — build-team

**Priorité : haute**
**Dépend de :** `20-build-team-roster-wiring.md`
**Contexte :** `team_creation`

## Objectif

Le cartouche "Compétition" dans la sidebar de `build-team.html` est entièrement
hardcodé. Le câbler avec les vraies données : nom de la compétition, nom de la
saison, et tiers disponibles (nom, budget, XP de départ).

---

## État de l'existant

Le panier temps réel (OOB cart) est déjà implémenté depuis les cartes 21–24.

Le cartouche hardcodé actuel dans `build-team.html` :

```html
<div class="rules-panel">
  <div class="rules-panel-title">Compétition</div>
  <div class="league-summary">
    <div class="rules-comp-name">Ligue de Condate</div>          <!-- hardcodé -->
    <div class="rules-season">Saison 3 · 2024–2025</div>         <!-- hardcodé -->
  </div>
  <div class="rules-section">
    <div class="rules-section-title">Tiers disponibles</div>
    <div class="rules-tier">
      <div class="rules-tier-name">Débutants</div>               <!-- hardcodé -->
      <div class="rules-tier-meta">Budget : 1 000 kpO · XP départ : 0</div>
    </div>
    ...
  </div>
</div>
```

| Source de données | Disponibilité |
|---|---|
| Nom de la compétition | `state.competitions.repository.find_with_seasons(&space_id)` → `CompetitionWithSeasons.competition_name` |
| Nom de la saison | Même appel → `SeasonOption.season_name` (filtrer par `draft.season_id()`) |
| Tiers (nom, budget, XP) | `DraftTeam.creation_rules().tiers` → `Vec<CreationTier>` |

---

## Conception

### Vue model

```rust
// build_team.rs
pub struct RulesTierVm {
    pub name:     String,
    pub budget:   u32,
    pub start_xp: u32,
}

pub struct RulesPanelVm {
    pub competition_name: String,
    pub season_name:      String,
    pub tiers:            Vec<RulesTierVm>,
}
```

### Builder

Dans le handler `build_team` :

```rust
// Après le chargement du DraftTeam
let competitions = state.competitions.repository
    .find_with_seasons(&SpaceId::try_new(&space_id)?)
    .await
    .ok()
    .unwrap_or_default();

let rules_panel = {
    let comp = competitions.iter().find(|c| c.competition_id == draft.competition_id());
    RulesPanelVm {
        competition_name: comp.map(|c| c.competition_name.clone()).unwrap_or_default(),
        season_name: comp
            .and_then(|c| c.seasons.iter().find(|s| s.season_id == draft.season_id()))
            .map(|s| s.season_name.clone())
            .unwrap_or_default(),
        tiers: draft.creation_rules().tiers.iter().map(|t| RulesTierVm {
            name:     t.name.clone(),
            budget:   t.budget / 1000,  // en kPo
            start_xp: t.start_xp,
        }).collect(),
    }
};
```

### Template

Remplacer le bloc hardcodé dans `build-team.html` par un rendu dynamique
depuis `rules_panel: RulesPanelVm` passé dans `BuildTeamTemplate` :

```html
<div class="rules-panel">
  <div class="rules-panel-title">Compétition</div>
  <div class="league-summary">
    <div class="rules-comp-name">{{ rules_panel.competition_name }}</div>
    <div class="rules-season">{{ rules_panel.season_name }}</div>
  </div>
  <div class="rules-section">
    <div class="rules-section-title">Tiers disponibles</div>
    {% for tier in rules_panel.tiers %}
    <div class="rules-tier">
      <div class="rules-tier-name">{{ tier.name }}</div>
      <div class="rules-tier-meta">Budget : {{ tier.budget }} kPo · XP départ : {{ tier.start_xp }}</div>
    </div>
    {% endfor %}
  </div>
</div>
```

---

## Checklist

- [ ] `RulesTierVm` + `RulesPanelVm` dans `build_team.rs`
- [ ] Builder dans `build_team` handler : `find_with_seasons` → filtrer par `competition_id` et `season_id`
- [ ] `BuildTeamTemplate` reçoit `rules_panel: RulesPanelVm`
- [ ] `build-team.html` : remplacer le bloc hardcodé par le rendu dynamique
