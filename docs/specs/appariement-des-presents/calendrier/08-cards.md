# Calendrier · appariement des présents — cartes

Épic : `kanban/epics/en_cours/E18-appariement-des-presents.md`.

| Ordre | Carte | Contenu | Dépend de |
|---|---|---|---|
| 1 | `612-le-tirage-rejoint-le-domaine` | `domain/draw.rs` copié puis renommé, `draw_pools` (R10), tests copiés | — |
| 2 | `613-match-day-accepte-ou-refuse-un-tirage` | `can_receive_draw`, `accept_draw`, `DrawRules`, sept variants de `DomainError` | 612 |
| 3 | `614-ecrire-une-journee-d-un-bloc` | `save_pairings` port et dépôt, `write_pairing`, `test_save_pairings.rs` (R11, R14) | — |
| 4 | `615-generer-une-journee-complete-passe-au-nouveau-tirage` | `draw_input.rs`, `pairing_created.rs`, `generate_pairings` réécrit, `generate_round_pairings` retiré, e2e n°9 | 612, 613, 614 |
| 5 | `616-l-etape-presents` | service de lecture, `GET …/draw`, gabarit, CSS, bouton d'entrée | 615 |
| 6 | `617-l-apercu-du-tirage` | `preview_draw_use_case`, `POST …/draw/preview`, gabarit | 616 |
| 7 | `618-ecrire-le-tirage-au-calendrier` | `confirm_draw_use_case`, `POST …/draw/confirm`, toast de succès | 617 |
| 8 | `619-les-tests-e2e-de-l-appariement-des-presents` | scénarios 1 à 8, `impact-map.toml` | 618 |

**615 passe avant l'écran** : elle corrige seule le défaut de
`generate_round_pairings` et pose ce que 616 à 618 réutilisent.

Chaque carte de 612 à 618 porte ses propres tests unitaires ou d'intégration ;
619 réunit les tests e2e du parcours.
