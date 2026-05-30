# Widget de sélection de roster avec tiers

**Priorité : haute**
**Contexte :** `team_creation` (affichage) + `references` (données)

## Objectif

Remplacer le `<select>` hardcodé dans `build-team.html` par un widget dynamique qui :
- liste les rosters autorisés filtrés par les `CreationRules` de l'équipe
- les regroupe par tier avec le budget associé
- émet les infos complètes du roster sélectionné en JSON lors de la confirmation

---

## État de l'existant

| Élément | Fichier | Remarque |
|---|---|---|
| Fragment plat (sans tiers) | `references/io/web/templates/roster-picker.html` | Chips sans filtrage, sans tiers |
| Builder d'items | `references/io/web/pickers.rs` → `build_roster_items()` | Liste complète, non filtrée |
| Données roster complètes | `references/domain/models.rs` → `Team { uid, name, tier, reroll_cost, … }` | `tier` est une `String` ("1", "2", "3") |
| Règles de création | `team_creation/domain/creation_rules.rs` → `CreationRules { tiers: Vec<CreationTier> }` | `CreationTier { name, budget, start_xp, rosters: Vec<String> }` — `rosters` = UIDs autorisés |
| Select hardcodé | `team_creation/io/web/templates/build-team.html` | À remplacer |

---

## Conception

### Vue model

Dans `references/io/web/pickers.rs`, ajouter :

```rust
pub struct RosterTierVm {
    pub name:   String,   // nom du tier (ex. "Débutants")
    pub budget: u32,      // budget kPo
    pub rosters: Vec<RosterPickerItem>,
}

pub fn build_roster_tiers(
    repo:  &dyn IReferenceRepository,
    rules: &CreationRules,
) -> Vec<RosterTierVm> {
    // Pour chaque CreationTier :
    //   1. filtrer repo.list_teams() sur les UIDs autorisés (tier.rosters)
    //   2. trier les rosters par nom
    //   3. construire le RosterTierVm
    // Conserver l'ordre des tiers tel que défini dans CreationRules
}
```

### Template fragment

Nouveau fichier : `references/io/web/templates/roster-picker-tiers.html`

```
{% for tier in tiers %}
  <div class="roster-tier">
    <div class="roster-tier-header">
      <span class="roster-tier-name">{{ tier.name }}</span>
      <span class="roster-tier-budget">Budget : {{ tier.budget }} kPo</span>
    </div>
    <div class="roster-grid">
      {% for item in tier.rosters %}
      <span class="roster-chip" data-uid="{{ item.uid }}" data-tier="{{ tier.name }}" data-budget="{{ tier.budget }}">
        {{ item.name }}
      </span>
      {% endfor %}
    </div>
  </div>
{% endfor %}
```

Le template existant `roster-picker.html` reste inchangé (utilisé ailleurs).

### Intégration dans `build-team.html`

- Injecter `tiers: Vec<RosterTierVm>` dans le template `BuildTeamTemplate`
- Alimenter via `build_roster_tiers(references_repo, draft_team.creation_rules())`
- Inclure le nouveau fragment à la place du `<select>` hardcodé

### Payload JSON en sortie

Au clic sur un chip, un script inline constitue l'objet et le stocke dans un champ `<input type="hidden" name="roster">` :

```json
{
  "uid":          "chaos",
  "name":         "Chaos Rénégats",
  "tier_name":    "Débutants",
  "budget":       1000,
  "reroll_cost":  60
}
```

Le champ `reroll_cost` vient de `Team.reroll_cost` dans le repo references (disponible au moment du clic via `data-reroll-cost` sur le chip).

Le formulaire de la page utilise `hx-ext="json-enc"` : le JSON est envoyé au POST suivant (étape sélection roster).

L'action de changer de roster va hydrater l'objet de domaine TeamRosterChosen et délcencher l'appel RosterChanged.

---

## Checklist

- [ ] `build_roster_tiers(repo, rules)` dans `references/io/web/pickers.rs`
- [ ] Template `roster-picker-tiers.html` dans `references/io/web/templates/`
- [ ] `BuildTeamTemplate` reçoit `Vec<RosterTierVm>` (import du contexte references dans team_creation)
- [ ] Handler `build_team` charge les données depuis `AppState` (references + draft_team)
- [ ] Script inline : clic chip → sélection visuelle + hydratation du champ hidden JSON
- [ ] Chip désactivé si le roster n'est pas dans les UIDs autorisés du tier (défense en profondeur)
- [ ] CSS : `.roster-tier`, `.roster-tier-header`, `.roster-tier-budget` (dans `team-build.css`)