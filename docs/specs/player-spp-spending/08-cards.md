# Player SPP spending — Phase 8 : Cartes kanban

9 cartes créées dans `kanban/ready_to_be_done/`, ordonnées par dépendance.

| # | Carte | Dépend de | Résumé |
|---|---|---|---|
| 175 | `players-domain-improvement` | — | Agrégat `Player` : `stat_increases`, `spp_remaining()`, `purchase_skill()`/`increase_stat()` |
| 176 | `references-improvement-value-data` | — | Table officielle de `value_delta` (JSON + domaine + port) |
| 177 | `players-skill-catalog-port` | 176 | `ISkillCatalogPort` + adaptateur vers `references` |
| 178 | `players-purchase-skill-use-case` | 175, 177 | Use case achat compétence + handler + route |
| 179 | `players-increase-stat-use-case` | 175, 177, 178 | Use case augmentation caractéristique + handler + route |
| 180 | `teams-player-improvement-app-event` | 178, 179 | App event `players → teams`, construit enfin `PlayerImprovementApplied` |
| 181 | `players-detail-page-widget-slot` | 175 | Slot unique sur la fiche joueur, widget journal extrait |
| 182 | `players-spp-spending-widget` | 177, 178, 179, 181 | Widget interactif (tabs, réutilisation `skill_picker`) |
| 183 | `players-spp-spending-e2e` | 180, 182 | Tests E2E |

Groupes parallélisables : {175, 176} démarrables immédiatement. {177} après
176. {178} après 175+177. {179} après 178 (réutilise `resolve_stat_cost`).
{180} après 178+179. {181} après 175 (indépendant de 176-180). {182} après
177+178+179+181. {183} en dernier.
