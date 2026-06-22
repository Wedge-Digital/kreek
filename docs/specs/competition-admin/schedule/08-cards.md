# Calendrier — Phase 8 : Cartes kanban ✅

## Cartes produites

| # | Carte | Dépend de |
|---|---|---|
| 99 | Domaine MatchDay + Pairing + generate_round_pairings + tests unitaires | — |
| 100 | Migration tables + IMatchDayRepository | 99 |
| 101 | Use cases generate_pairings + generate_all_pairings | 99, 100 |
| 102 | Widget sidebar (sync structure + liste journées) | 100 |
| 103 | Widget round detail (config date + liste pairings + TomSelect) | 100, 102 |
| 104 | Actions handlers (10 routes POST/PUT/DELETE) | 100, 101 |
| 105 | Fragment onglet calendrier + câblage page hôte + CSS | 102, 103, 104 |
| 106 | Tests E2E calendrier | 105 |

## Ordre d'implémentation

```
99 → 100 → 101 → 102 → 103 → 104 → 105 → 106
```
