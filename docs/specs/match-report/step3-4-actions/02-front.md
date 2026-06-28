# Step 3 & 4 — Actions match — Architecture front

## Composition de la page

Assemblage de 5 widgets sur une page hôte fournie par le BC MatchReport.
Step3 = actions équipe domicile. Step4 = actions équipe visiteur.
Même gabarit Askama, même architecture, données différentes.

```
Page hôte : BC MatchReport (step3 ou step4)
├── Header (nom équipe, étapes)        ← rendu serveur (MatchReport)
│
├── Widget : turn-selector             ← BC MatchReport, hx-get au load + actionRecorded
├── Widget : player-selector           ← BC Players, hx-get au load
├── Widget : temp-player-selector      ← BC MatchReport, hx-get au load
├── Widget : action-panel              ← BC MatchReport, hx-get au load
├── Widget : action-log                ← BC MatchReport, hx-get au load + actionRecorded
│
└── Bouton navigation (évent final)    ← rendu serveur (MatchReport)
```

La page hôte ne porte aucune logique métier. Elle assemble les widgets et fournit le bouton de navigation vers l'étape suivante.

---

## URLs

```
GET /app/{space_id}/match-report/{mr_id}/step3   ← actions équipe domicile
GET /app/{space_id}/match-report/{mr_id}/step4   ← actions équipe visiteur
```

Navigation libre entre step3 et step4 (aucun verrouillage).

---

## Machine d'état (inter-widgets via événements DOM sur `body`)

```
État 0 : Tour non sélectionné
  → player-selector    : désactivé
  → temp-player-selector : désactivé
  → action-panel       : désactivé

État 1 : Tour sélectionné  (après turnSelected)
  → player-selector    : actif, aucun joueur sélectionné
  → temp-player-selector : actif
  → action-panel       : désactivé

État 2 : Joueur sélectionné  (après playerSelected)
  → action-panel       : actif

État 3 : Action enregistrée  (après actionRecorded)
  → retour état 1 (tour conservé, joueur désélectionné)
```

---

## Événements DOM

| Événement | Payload | Émis par | Écouté par |
|---|---|---|---|
| `turnSelected` | `{ turn: number }` | turn-selector | player-selector, temp-player-selector, action-panel |
| `playerSelected` | `{ player_id: string, player_type: string }` | player-selector, temp-player-selector | action-panel |
| `actionRecorded` | `{ action_id: string }` | action-panel | turn-selector, player-selector, temp-player-selector, action-panel, action-log |

---

## Widget : turn-selector (BC MatchReport)

**Endpoint** : `GET /app/{space_id}/match-report/{mr_id}/step3/turn-selector`
(identique pour step4 avec `/step4/`)

**Trigger HTMX** : `load`, `actionRecorded from:body`

**Rendu serveur** : 16 boutons (8 par mi-temps). Chaque bouton reçoit la classe
`has-events` si au moins un événement a été enregistré pour ce tour.

**Interactions Alpine** :

```js
{
  selectedTurn: null,
  select(turn) {
    this.selectedTurn = turn;
    htmx.trigger(document.body, 'turnSelected', { turn });
  },
  reset() { this.selectedTurn = null; }
}
```

Écoute `actionRecorded` pour conserver le tour courant (pas de reset du tour).

---

## Widget : player-selector (BC Players)

**Endpoint** : `GET /app/{space_id}/players/teams/{team_id}/match-selector`

**Trigger HTMX** : `load`

**Params** : `team_id` baked dans l'URL par la page hôte au rendu Askama.

**Rendu serveur** : grille de chips pour les joueurs réguliers de l'équipe.
Un joueur indisponible (blessé, suspendu) est rendu avec la classe `disabled`
(grisé, non cliquable).

**Interactions Alpine** :

```js
{
  selectedPlayerId: null,
  enabled: false,   // passe à true sur turnSelected
  select(playerId) {
    this.selectedPlayerId = playerId;
    htmx.trigger(document.body, 'playerSelected',
      { player_id: playerId, player_type: 'regular' });
  },
  reset() { this.selectedPlayerId = null; }
}
```

Écoute :
- `turnSelected from:body` → `enabled = true`
- `actionRecorded from:body` → `reset()`

---

## Widget : temp-player-selector (BC MatchReport)

**Endpoint** : `GET /app/{space_id}/match-report/{mr_id}/step3/temp-players`
(identique pour step4 avec `/step4/`)

**Trigger HTMX** : `load`

**Rendu serveur** : grille de chips pour les joueurs temporaires de l'équipe
(star players, mercenaires, journaliers). Identifiés par leur `TempPlayerId`
(UUID match-scoped créé en fin d'étape 2). Pas de nom pour les mercenaires et
journaliers ; les star players affichent le nom du référentiel.

**Interactions Alpine** : même pattern que player-selector, avec
`player_type: 'star' | 'merc' | 'journalier'`.

---

## Widget : action-panel (BC MatchReport)

**Endpoint** : `GET /app/{space_id}/match-report/{mr_id}/step3/action-panel`

**Trigger HTMX** : `load`

**Rendu serveur** : les 7 boutons d'action + le panneau blessure (masqué par défaut).

### State Alpine

```js
{
  enabled: false,              // passe à true sur playerSelected
  currentTurn: null,
  currentPlayerId: null,
  currentPlayerType: null,
  showInjuryPanel: false,
  selectedInjuryType: null,   // null | 'commotion'|'amoche'|'serious'|'sequel'|'death'
  selectedSequel: null,        // null | '-1 AV'|'-1 MA'|'-1 PA'|'-1 AG'|'-1 ST'

  canAct()          { return this.enabled; },
  selectInjuryType(type) {
    this.selectedInjuryType = type;
    if (type !== 'sequel') {
      this.selectedSequel = null;
      this.submitAction('blesse');   // POST immédiat si pas séquelle
    }
  },
  selectSequel(stat) {
    this.selectedSequel = stat;
    this.submitAction('blesse');     // POST immédiat une fois la séquelle choisie
  },
  reset() {
    this.enabled = false;
    this.showInjuryPanel = false;
    this.selectedInjuryType = null;
    this.selectedSequel = null;
  }
}
```

Écoute :
- `turnSelected from:body` → `currentTurn = event.detail.turn`
- `playerSelected from:body` → `enabled = true`, `currentPlayerId = event.detail.player_id`,
  `currentPlayerType = event.detail.player_type`
- `actionRecorded from:body` → `reset()`

### Actions directes (TD, Passe, Interception, Agression, Lancer, Sortie, MVP)

Chaque bouton déclenche un `hx-post` immédiat avec `:hx-vals` injectant
`{ turn, player_id, player_type, action_type }`. Le bouton est `x-bind:disabled="!canAct()"`.

### Action Blessé (flux en 2 temps)

1. Clic "Blessé" → `showInjuryPanel = true`
2. Clic type ≠ Séquelle → `submitAction('blesse')` (POST immédiat)
3. Clic type = Séquelle → panneau séquelle visible → clic stat → `submitAction('blesse')` (POST immédiat)

Pas de bouton "Confirmer".

### Mutations

| Méthode | Route | Body | Retour |
|---|---|---|---|
| POST | `/step3/actions` | `{ turn, player_id, player_type, action_type }` | `HX-Trigger: actionRecorded` |
| POST | `/step3/actions` | `{ turn, player_id, player_type, action_type: "blesse", injury_type, sequel? }` | `HX-Trigger: actionRecorded` |
| POST | `/step4/actions` | idem | `HX-Trigger: actionRecorded` |

---

## Widget : action-log (BC MatchReport)

**Endpoint** : `GET /app/{space_id}/match-report/{mr_id}/step3/log`
(identique pour step4)

**Trigger HTMX** : `load`, `actionRecorded from:body`

**Rendu serveur** : liste des événements de l'équipe courante, ordre chronologique.
Chaque entrée : icône, tour, description, bouton suppression.

### Suppression

| Méthode | Route | Retour |
|---|---|---|
| DELETE | `/app/{space_id}/match-report/{mr_id}/actions/{action_id}` | `HX-Trigger: actionRecorded` |

La suppression émet `actionRecorded` → les widgets turn-selector et action-log se rafraîchissent.

---

## Navigation

```
← Retour   → GET /step2/inducements/{away_team_id}   (page hôte, lien statique)
→ Suivant  → GET /step4  (depuis step3)  ou  GET /step5  (depuis step4)
```

Le bouton "Suivant" est rendu par la page hôte. Il envoie l'« évent final »
(à préciser en phase 7 — potentiellement un POST marquant les actions de l'équipe
comme terminées avant la navigation).

---

## Règles métier relevées (Phase 1 + Phase 2)

- Joueur régulier indisponible → chip grisée, non sélectionnable (BC Players)
- Ordre de saisie obligatoire : tour → joueur → action
- Pour Blessé : tour → joueur → Blessé → type → [séquelle si Séquelle] → enregistrement automatique
- Blessure Sérieuse (11-12) = Niggling pour les joueurs réguliers uniquement
- Aucun maximum sur les MVP
- Événements indépendants (suppression d'un event n'entraîne pas la suppression d'un autre)
- Mercenaires et journaliers : pas de nom, identifiés par `TempPlayerId` (UUID match-scoped)
- Star players : identité connue du référentiel (nom affiché)
- Journaliers : nombre = `max(0, 11 - joueurs_disponibles)`, dérivé de BC Players
- Broadcast vers BC Players : tous les événements d'action → calcul SPP côté BC Players
