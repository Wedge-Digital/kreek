# Step 2 — Mercenaires — Architecture front

## Périmètre

Ajout d'un 4ème onglet "Mercenaires" à la page step 2 (Coups de pouce) existante.
Cette page est déjà servie deux fois : TopDog puis Underdog. Le tab mercenaires suit ce même cycle.

---

## Composition de la page après modification

```
Page hôte : inducements.html (BC match_report) — MODIFIÉE
├── Header + budget banner              ← inchangé
├── Barre 4 onglets                     ← MIGRÉ hors du widget inducement-selector
│   ├── [Communs] [Spéciaux] [Stars]    ← contrôlent le widget references via événement DOM
│   └── [Mercenaires]                   ← contrôle le widget mercenaires (nouveau)
├── Zone inducement-selector            ← BC références, x-show masquée si onglet Mercenaires
├── Zone mercenary-selector             ← BC match_report, chargée lazy au 1er clic
└── Cart footer                         ← MODIFIÉ : écoute les deux sélections
```

---

## Changement architectural : migration du tab bar

Le widget `inducement-selector` (BC références) gère aujourd'hui sa propre `<div class="mr-tabs">`.
Pour intégrer le 4ème onglet "Mercenaires" dans une barre unifiée, la barre d'onglets est **migrée dans la page hôte**.

### Modification minimale du widget `inducement-selector` (BC références)

- **Supprimer** la `<div class="mr-tabs">` du template `inducement-selector.html`
- **Ajouter** dans le composant Alpine : `@switch-inducement-tab.window="activeTab = $event.detail.tab"`
- L'`activeTab` initial reste `'common'` (ou le premier tab non vide)

### Page hôte — nouveau tab bar

```html
<div class="mr-tabs" x-data="{ activeSection: 'common' }">
  <div class="mr-tab" :class="{ active: activeSection === 'common' }"
       @click="activeSection = 'common'; htmx.trigger(document.body, 'switchInducementTab', { tab: 'common' })">
    Communs
  </div>
  <div class="mr-tab" :class="{ active: activeSection === 'special' }"
       @click="activeSection = 'special'; htmx.trigger(document.body, 'switchInducementTab', { tab: 'special' })">
    Spéciaux
  </div>
  <div class="mr-tab" :class="{ active: activeSection === 'stars' }"
       @click="activeSection = 'stars'; htmx.trigger(document.body, 'switchInducementTab', { tab: 'stars' })">
    Stars
  </div>
  <div class="mr-tab" :class="{ active: activeSection === 'merco' }"
       @click="activeSection = 'merco'">
    Mercenaires
  </div>
</div>

<div x-show="activeSection !== 'merco'" class="mr-selector-zone"
     hx-get="{{ inducement_selector_url }}" hx-trigger="load" hx-target="this" hx-swap="innerHTML">
</div>

<div x-show="activeSection === 'merco'" id="merco-zone"
     hx-get="{{ mercenary_selector_url }}" hx-trigger="mercenairesActivated from:body once"
     hx-target="this" hx-swap="innerHTML">
</div>
```

Le div merco écoute `mercenairesActivated` émis au clic de l'onglet (chargement lazy unique).

---

## Widget : mercenary-selector (nouveau — BC match_report)

**Endpoint** : `GET /app/{space_id}/match-report/{mr_id}/step2/{team_id}/mercenaires`

**Rendu serveur** : le handler croise les données de deux ports pour produire la grille :

| Donnée | Port | Méthode (à créer) |
|--------|------|-------------------|
| Positions du roster (nom, coût, max_qty, is_journalier) | `ITeamDataPort` | `find_roster_positions(team_id)` |
| Counts joueurs par position dans l'équipe | `IPlayerDataPort` | `find_player_counts_by_position(team_id)` |

Les journaliers (`is_journalier: true`) sont **exclus** de la grille — ils ne peuvent pas être recrutés comme mercenaires.

Une position est affichée comme `disabled` si `count_in_team >= max_qty`.

**Isolation** : `hx-disinherit="*"` sur l'élément racine.

### Comportement Alpine (`x-data="mercenarySelector()"`)

```js
{
  selectedPosition: null,  // { uid, name, base_cost }
  mercenaries: [],         // [{ position_uid, position_name, tier, price }]

  selectPosition(pos) {
    this.selectedPosition = pos;
  },
  addMerc(tier) {
    if (!this.selectedPosition || this.mercenaries.length >= 3) return;
    const extra = tier === 'lvl1' ? 80 : 30;
    this.mercenaries.push({
      position_uid:   this.selectedPosition.uid,
      position_name:  this.selectedPosition.name,
      tier,
      price: this.selectedPosition.base_cost + extra,
    });
    this.selectedPosition = null;
    this.emit();
  },
  removeMerc(idx) {
    this.mercenaries.splice(idx, 1);
    this.emit();
  },
  emit() {
    htmx.trigger(document.body, 'mercenarySelectionChanged', {
      mercenaries: this.mercenaries,
      total_cost:  this.mercenaries.reduce((s, m) => s + m.price, 0),
    });
  }
}
```

### Tableau des interactions

| Geste utilisateur | Couche | Effet |
|-------------------|--------|-------|
| Clic sur une position card | Front (Alpine) | `selectedPosition` mise à jour, hire panel visible via `x-show` |
| Clic "Recruter" (base ou Niv.1) | Front (Alpine) | `addMerc(tier)`, hire panel masqué, `mercenarySelectionChanged` émis |
| Clic ✕ sur un mercenaire | Front (Alpine) | `removeMerc(idx)`, `mercenarySelectionChanged` émis |
| Aucun appel serveur pendant la sélection | | |

La validation des limites (max 3, limites roster cumulées avec les mercenaires déjà sélectionnés)
est faite **côté domaine au submit** — le front ne fait pas confiance à son propre état.

---

## Événements DOM

| Événement | Payload | Émis par | Écouté par |
|-----------|---------|----------|------------|
| `switchInducementTab` | `{ tab: 'common'\|'special'\|'stars' }` | JS page hôte (clic onglet) | Widget inducement-selector (`@switch-inducement-tab.window`) |
| `mercenairesActivated` | — | JS page hôte (clic onglet Mercenaires) | Zone merco (`hx-trigger once` → chargement lazy) |
| `inducementSelectionChanged` | `{ items, total_cost }` | Widget inducement-selector | Cart footer (Alpine) |
| `mercenarySelectionChanged` | `{ mercenaries, total_cost }` | Widget mercenary-selector | Cart footer (Alpine) |

---

## Cart footer (modifié)

Le cart footer Alpine agrège les deux sélections indépendantes :

```js
x-data="{
  inducementTotal: 0,
  mercenaryTotal:  0,
  mercenaries:     [],
  budget:          {{ budget }},
  get totalCost()  { return this.inducementTotal + this.mercenaryTotal; },
  get overBudget() { return this.totalCost > this.budget; }
}"
@inducement-selection-changed.window="inducementTotal = $event.detail.total_cost"
@mercenary-selection-changed.window="
  mercenaries    = $event.detail.mercenaries;
  mercenaryTotal = $event.detail.total_cost
"
```

**Affichage cart mercenaires** : liste des mercenaires sélectionnés avec ✕.
Le ✕ du cart émet `removeMercenaire { idx }` écouté par le widget :

```js
// Cart → widget
htmx.trigger(document.body, 'removeMercenaire', { idx });

// Widget écoute :
@remove-mercenaire.window="removeMerc($event.detail.idx)"
```

---

## Soumission du formulaire (POST étendu)

Le formulaire existant soumet `selection` (JSON inducements classiques). On ajoute un second champ caché :

```html
<input type="hidden" name="mercenaries"
       :value="JSON.stringify(mercenaries.map(m => ({ position_uid: m.position_uid, tier: m.tier })))">
```

Le handler parse les deux champs. La logique existante de `selection` est inchangée.

---

## Ports à créer (résumé)

Ces deux méthodes s'ajoutent à des traits existants — aucun nouvel adapter.

| Trait | Méthode | Implémentation |
|-------|---------|----------------|
| `ITeamDataPort` | `find_roster_positions(team_id: &str) -> Vec<RosterPositionDto>` | `TeamDataAdapter` : `team_repo.find_by_id` → `roster_id` → `reference_repo.find_team_by_uid` → `available_players` |
| `IPlayerDataPort` | `find_player_counts_by_position(team_id: &str) -> Vec<PositionCountDto>` | `PlayerDataAdapter` : `find_by_team_id` → group by `roster_line_id` |

---

## Widgets réutilisables

Aucun widget existant n'est réutilisé tel quel pour le mercenary-selector. Les cartes de position
utilisent les classes CSS `mr-position-card` définies dans la maquette (à créer dans un fichier CSS dédié).

---

## Règles métier identifiées à cette étape

- Max 3 mercenaires par équipe par match (enforced front + back)
- Un mercenaire recruté compte dans la limite roster pour les recrutements suivants du même type
- Les journaliers sont exclus de la grille de sélection
- Prix : `position.base_cost + 30 kPo` (Mercenaire) / `+ 80 kPo` (Mercenaire Niv. 1)
- La grille est rendue disabled côté serveur si `count_in_team >= max_qty` au chargement du widget
- La validation finale des limites cumulées (joueurs équipe + mercenaires déjà sélectionnés) est faite côté domaine
- Budget mercenaires = budget inducement de l'équipe (trésorerie TopDog, budget underdog pour l'autre)
