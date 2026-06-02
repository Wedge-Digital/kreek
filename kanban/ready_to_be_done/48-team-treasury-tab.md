# BC `teams` — Onglet Trésorerie de la fiche d'équipe

**Priorité : moyenne**
**Dépend de :** `29-teams-repository.md`
**Contexte :** `teams` — lecture depuis l'event store

## Objectif

Afficher l'historique des mouvements financiers d'une équipe dans l'onglet "Trésorerie" de la fiche d'équipe. Les données sont extraites directement de l'event store de BC `teams` — aucun BC externe nécessaire.

---

## Conception

### Source des données

Chaque événement financier est déjà daté et persisté dans `team_event_store`. Il suffit de filtrer les events qui touchent `treasury` et de les projeter en lignes de journal.

| `event_type` | Libellé affiché | Signe |
|---|---|---|
| `MatchPlayedReceived` | Revenus du match | + |
| `PlayerRecruited` | Recrutement joueur | − |
| `StaffBought` | Achat staff / relances | − |
| `CostlyMistakesApplied` | Erreur couteuse | − |

### Endpoint

```
GET /app/{space_id}/teams/{team_id}/treasury-tab
```

Chargé via HTMX au clic sur l'onglet :

```html
<!-- Dans team-detail.html -->
<div id="treasury-tab"
     hx-get="{{ team_routes.treasury_tab(space_id, team_id) }}"
     hx-trigger="click from:#tab-treasury"
     hx-target="#tab-content"
     hx-swap="innerHTML">
</div>
```

### View model

```rust
pub struct TreasuryTabVm {
    pub current_balance_kpo: u32,
    pub movements:           Vec<TreasuryMovementVm>,
}

pub struct TreasuryMovementVm {
    pub date:        String,
    pub label:       String,   // ex. "Revenus du match vs Nantes Undead"
    pub amount_kpo:  i32,      // positif ou négatif
    pub balance_kpo: u32,      // balance après ce mouvement (running total)
}
```

### Handler

```rust
pub async fn treasury_tab(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    // Charge uniquement les events financiers depuis team_event_store
    // WHERE team_id = $1 AND event_type IN ('MatchPlayedReceived', 'PlayerRecruited',
    //                                        'StaffBought', 'CostlyMistakesApplied')
    // ORDER BY version ASC
    // Calcule le running total pour afficher la balance à chaque ligne
}
```

Requête ciblée sur `event_type` — pas de rejeu complet de l'agrégat.

---

## Checklist

- [ ] Requête SQL ciblée : `SELECT … FROM team_event_store WHERE event_type IN (…) ORDER BY version`
- [ ] `TreasuryMovementVm` + calcul du running total
- [ ] Handler `treasury_tab` → fragment HTML
- [ ] Template fragment onglet trésorerie : balance courante + tableau de mouvements
- [ ] Slot `hx-get` dans `team-detail.html` (carte 34)
- [ ] Route `TREASURY_TAB` dans `routes.rs` + `router.rs`
