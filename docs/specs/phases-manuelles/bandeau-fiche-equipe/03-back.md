# Bandeau de la fiche équipe · Phase 3 : architecture back

**Prérequis livré** : carte 570 — `is_admin`, `team_admin_guard`.

## Tout vit dans le BC `teams`

Aucun widget d'un autre BC, aucun port neuf. Le droit se demande au service
commun (`shared_kernel::bloodbowl::admin_access::is_admin`), que `teams` reçoit
déjà dans son contexte (`TeamsContext.admin_access`).

## Les fichiers neufs

| Fichier | Rôle |
|---|---|
| `teams/io/web/phase_override_controller.rs` | `post_phase_override` : lit le formulaire, exige le droit, appelle le use case, répond `HX-Refresh` ou le pied d'erreur |
| `teams/io/web/templates/phase-override-foot.html` | le pied du panneau — inclus par le gabarit de la page, rendu seul en cas d'erreur |
| `teams/use_cases/open_phase_override_use_case.rs` | charge l'agrégat, lui demande d'ouvrir la phase, persiste l'événement |

## Les fichiers modifiés

| Fichier | Ce qui change |
|---|---|
| `teams/routes.rs` | `PHASE_OVERRIDE = "/app/{space_id}/teams/{team_id}/phases/override"`, `Routes::phase_override()` |
| `teams/router.rs` | la route dans `routes_ouvertes()`, **hors** de `garde_action_equipe` qui admet le propriétaire ; le contrôleur se garde par `team_admin_guard::require_team_admin`, comme l'ajustement de trésorerie |
| `teams/io/web/team_detail.rs` | `BannerCtaVm::OpenPhaseOverride { post_url }` quand l'équipe est prête à jouer et le visiteur admin ; le droit est le booléen `is_team_admin` déjà calculé dans `rendre_fiche` pour la trésorerie — pas de requête de plus |
| `teams/io/web/templates/teams-team-detail.html` | le bouton et le panneau sous le bandeau, avec le `x-data` de la phase 2 |
| `assets/static/css/pages/team-page.css` | les règles du panneau, reprises de la maquette |
| `teams/domain/team.rs` | `PhaseEntry`, les trois événements d'ouverture et `ManualPhaseClosed`, les méthodes d'ouverture, le choix de sortie dans les trois `validate_*_phase`, et `TeamDomainEvent::returns_to_ready_to_play()` (phase 6) |
| `teams/io/repository/team_repository.rs` | quatre bras de projection ; le `_ => {}` de fin de `match` remplacé par une liste exhaustive |
| `teams/io/listeners/team_value_listener.rs`, `phase_basket_purge_listener.rs` | leurs deux copies identiques de `ends_in_ready_to_play()` cèdent la place à `TeamDomainEvent::returns_to_ready_to_play()` |
| `teams/io/app_events/app_event_publisher.rs` | les quatre événements rejoignent le groupe explicite qui ne sort pas du BC — `players` lit la phase en direct |
| `infrastructure/match_report/ref_team_data_adapter.rs` | `is_team_in_player_improvement` ne répond oui qu'à une entrée `PostMatch` |

## Pourquoi `returns_to_ready_to_play()` vit dans le domaine

Les deux listeners portaient chacun la même liste des événements qui ramènent
l'équipe à « prête à jouer ». Leur commentaire justifiait la duplication par la
crainte de **fusionner les listeners** — un listener à deux responsabilités.
L'argument tient pour les listeners, pas pour la **question** qu'ils posent :
elle est du domaine, et `apply()` y répond déjà en posant `ReadyToPlay`.

La méthode est un `match` **exhaustif, sans joker** : la règle de `to_app_event`
appliquée à une projection. Les quatre événements de cette fonction en sont la
raison — en oublier un dans une liste, c'est une valeur d'équipe non recalculée
et des paniers non purgés, sans un bruit. Le compilateur ferme ce cas.

Les deux listeners restent deux listeners.

## Ce qui ne change pas

- Les use cases et contrôleurs de sortie : tous trois demandent l'événement de
  sortie à l'agrégat, qui décide selon l'entrée de phase.
- Les pages de recrutement et de renvois.
- Le BC `players` : la dépense de SPP lit la phase en direct
  (`team_roster_adapter`).

## Identifiants

En anglais (règle 18) : `PhaseEntry::{PostMatch, Override}`,
`ManualImprovementPhaseOpened`, `ManualRecruitmentPhaseOpened`,
`ManualDismissalsPhaseOpened`, `ManualPhaseClosed`, `returns_to_ready_to_play`,
`phase_override_controller`, `open_phase_override_use_case`,
`phase-override-foot`, `BannerCtaVm::OpenPhaseOverride`.

## Règles métier

Question posée le 2026-10-03 : la phase 3 n'en ajoute aucune. Elle décide où
vit le code, pas ce qui est permis.
