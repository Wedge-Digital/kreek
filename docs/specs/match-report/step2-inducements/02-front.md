# Step 2.1 — Coups de pouce — Architecture front

## Composition de la page

Assemblage d'un widget sur une page hôte fournie par le BC MatchReport.

```
Page hôte : BC MatchReport
├── Header + budget banner       ← rendu serveur (MatchReport)
├── Widget : inducement-selector ← BC References, chargé via hx-get au load
└── Cart (sticky footer)         ← rendu serveur (MatchReport), réactif via Alpine
```

La page hôte ne porte aucune logique métier. Elle assemble les fragments, transmet le budget au cart via Alpine `x-data`.

---

## URL

```
GET /app/{space_id}/match-report/{mr_id}/step2/inducements/{team_id}
```

La page est servie deux fois : une fois pour l'équipe TopDog, une fois pour l'équipe Underdog. L'identité de l'équipe courante est portée par `{team_id}`.

---

## Widget : inducement-selector (BC References)

**Endpoint** : `GET /references/inducement-selector`

**Params** :

| Param | Obligatoire | Description |
|---|---|---|
| `competition_id` | oui | Charge les inducements + star players autorisés par le tier de la compétition |
| `roster_id` | oui | Filtre les spéciaux (`restrictedTo`) et star players (`availableForRosters`) |
| `instance_id` | oui | Isolation multi-instance (convention des pickers existants) |
| `selected` | non | UIDs pré-sélectionnés (format `uid:qty,uid:qty`) pour retour en arrière |

**Rendu serveur** : 3 listes pré-rendues (Communs, Spéciaux, Star Players), filtrées par competition + roster. Les listes vides sont cachées ; l'onglet correspondant est désactivé.

**Interactions Alpine (front only)** :

- `activeCategory` : tab actif (`common` / `special` / `stars`)
- `quantities: {}` : map `uid → qty`
- `openStars: Set` : star players avec fiche dépliée
- Boutons `+` : incrément de qty, désactivé si `qty === maxQty`
- Boutons `−` : décrément de qty, désactivé si `qty === 0`
- Star player card click : toggle `openStars` (expand/collapse fiche stats)

**Événement DOM émis** (à chaque changement de qty, via `htmx.trigger`) :

```js
htmx.trigger(document.body, 'inducementSelectionChanged', {
  instanceId: '...',
  items: [{ uid, name, qty, unit_cost }],  // uniquement les items qty > 0
  total_cost: N
})
```

---

## Cart (BC MatchReport — rendu serveur, réactif Alpine)

Sticky footer. HTML Askama dans la page hôte, pas d'endpoint dédié.

**Données initiales rendues serveur (dans Alpine `x-data`)** :

- `budget` : montant en kPo (calculé par le domaine, injecté dans le template)
- `items` : `[]` (vide au chargement)
- `totalCost` : `0`

**Écoute DOM** :

```js
@inducement-selection-changed.window="
  items = $event.detail.items;
  totalCost = $event.detail.total_cost
"
```

**Affichage réactif** :

- Compteur : `X coup(s) de pouce sélectionné(s)`
- Total : `totalCost / budget kPo` — rouge si `totalCost > budget`
- Warning visible si `totalCost > budget` : "⚠ Budget dépassé de X kPo"
- Bouton "Valider les achats" : désactivé (`disabled`) si `totalCost > budget`

**Soumission** : un `<input type="hidden" name="selection">` est mis à jour via Alpine avec le JSON sérialisé des items sélectionnés. Le form POST s'appuie sur ce champ.

**Boutons** :

- **"Passer"** : lien `GET` vers l'URL de l'étape suivante (déterminée par le serveur, zéro achat enregistré pour l'équipe courante)
- **"Valider les achats →"** : `POST` sur `/app/{space_id}/match-report/{mr_id}/step2/inducements/{team_id}`

---

## Navigation

```
step2-avant-match POST
  → redirect → /step2/inducements/{topdog_team_id}

"Passer" depuis TopDog (GET)
  → /step2/inducements/{underdog_team_id}  (dépenses adverses = 0)

POST step2/inducements/{topdog_team_id}
  → redirect → /step2/inducements/{underdog_team_id}

"Passer" depuis Underdog (GET)
  → /step3/{mr_id}

POST step2/inducements/{underdog_team_id}
  → redirect → /step3/{mr_id}
```

**Cas spécial — aucun inducement disponible pour la compétition** :
Le serveur détecte ce cas au moment du redirect post-avant-match et saute directement vers `/step3`. Aucune page inducements n'est affichée.

---

## Règles métier relevées

- Budget dépassé → bouton "Valider" désactivé ; soumission impossible
- `maxQty` par inducement enforced côté widget (bouton `+` désactivé) ET côté back (use case)
- Max 2 star players par équipe (enforced front + back)
- Un star player ne peut pas être sélectionné par les deux équipes du même match (enforced back, au POST de la deuxième équipe)
- Aucun inducement disponible pour la compétition → phase 2.1 entièrement sautée pour les deux équipes
- TeamValue égale → home team = TopDog (achète en premier)
- Budget Underdog = différence TeamValue + dépenses TopDog + trésorerie Underdog
- Budget TopDog = trésorerie TopDog

---

## Pas d'autres événements DOM

Page formulaire — pas de communication inter-widgets au-delà de `inducementSelectionChanged`.
