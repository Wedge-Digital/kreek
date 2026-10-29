# Calendrier · appariement des présents — cartes

Épic : `kanban/epics/ready_to_be_done/E18-appariement-des-presents.md`.

| Ordre | Carte | Contenu | Dépend de |
|---|---|---|---|
| 1 | `595-le-tirage-rejoint-le-domaine` | `domain/draw.rs` copié puis renommé, `draw_pools` (R10), tests copiés | — |
| 2 | `596-match-day-accepte-ou-refuse-un-tirage` | `can_receive_draw`, `accept_draw`, `DrawRules`, sept variants de `DomainError` | 595 |
| 3 | `597-ecrire-une-journee-d-un-bloc` | `save_pairings` port et dépôt, `write_pairing`, `test_save_pairings.rs` (R11, R14) | — |
| 4 | `598-generer-une-journee-complete-passe-au-nouveau-tirage` | `draw_input.rs`, `pairing_created.rs`, `generate_pairings` réécrit, `generate_round_pairings` retiré, e2e n°9 | 595, 596, 597 |
| 5 | `599-l-etape-presents` | service de lecture, `GET …/draw`, gabarit, CSS, bouton d'entrée | 598 |
| 6 | `600-l-apercu-du-tirage` | `preview_draw_use_case`, `POST …/draw/preview`, gabarit | 599 |
| 7 | `601-ecrire-le-tirage-au-calendrier` | `confirm_draw_use_case`, `POST …/draw/confirm`, toast de succès | 600 |
| 8 | `602-les-tests-e2e-de-l-appariement-des-presents` | scénarios 1 à 8, `impact-map.toml` | 601 |

**598 passe avant l'écran** : elle corrige seule le défaut de
`generate_round_pairings` et pose ce que 599 à 601 réutilisent.

Chaque carte de 595 à 601 porte ses propres tests unitaires ou d'intégration ;
602 réunit les tests e2e du parcours.
