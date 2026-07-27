# Phase 3 — Architecture back · page Recap

## Vue d'ensemble

```
POST /recap/unpublish
      │
      ▼
 post_unpublish (io/web)                    ← traduit HTTP, vérifie les droits
      │
      ▼
 unpublish_match_report_use_case            ← charge l'agrégat + l'éligibilité
      │           │
      │           └─► correction_eligibility_service (use_cases)
      │                     │
      │                     ├─► ITeamDataPort::find_game_phase
      │                     └─► IPlayerDataPort::has_spent_spp_since_match
      │
      ▼
 MatchReportPublished::unpublish(coach_id, eligibility)   ← LE DOMAINE DÉCIDE
      │
      ▼
 MatchReportUnpublished (bus interne du BC)
      │
      ▼
 app_event_publisher (io)
      │
      ├─► MatchReportAppEvent::MatchReportUnpublished        → competitions, ranking, teams
      └─► PlayerMatchImpactAppEvent::TeamMatchImpactReverted → players (×2, une par équipe)
```

## Où vit le garde-fou

Le garde-fou répond à « ce rapport peut-il être corrigé ? » — une question
domaine. Mais sa réponse dépend **entièrement** d'état externe (phase de jeu dans
`teams`, SPP dépensés dans `players`), et le domaine ne peut pas appeler de
ports. D'où un découpage en trois temps :

| Couche | Responsabilité |
|---|---|
| Use case | charge l'agrégat, et l'éligibilité via le domain service |
| Domain service (`use_cases/`) | compose les DTOs des 2 ports en un **value object domaine** |
| Méthode domaine | **décide** : refuse, ou produit `MatchReportUnpublished` |

La méthode domaine est mince — elle rejette si non éligible — mais c'est le bon
endroit : la règle reste testable unitairement sans port ni HTTP, et le use case
ne décide de rien (cf. CLAUDE.md, « Responsabilités des couches »).

**Tension assumée** : on pourrait faire vérifier le use case et lui faire
retourner l'erreur, ce qui économiserait le value object. Choix explicite de
placer la décision dans le domaine.

## Ports — extension, pas de nouveau port

| Port (dans `match_report/ports.rs`) | Ajout | Adapter (existant) |
|---|---|---|
| `ITeamDataPort` | `find_game_phase(team_id)` | `infrastructure/match_report/ref_team_data_adapter.rs` |
| `IPlayerDataPort` | `has_spent_spp_since_match(team_id, match_report_id)` | `infrastructure/match_report/player_data_adapter.rs` |

Les deux adapters existent déjà : extension pure, aucun câblage nouveau dans
`main.rs`.

Le nom d'équipe nécessaire au message de blocage (règle 3) est déjà fourni par
`ITeamDataPort::find_team_info`, que le recap consomme déjà. Aucun ajout.

**Pourquoi pas un port unique `ICorrectionEligibilityPort`** qui répondrait d'un
coup : ça placerait la composition « 2 BCs → 1 verdict » dans un adapter, donc
dans `infrastructure/`. C'est de la composition métier, pas de l'infrastructure.
CLAUDE.md (« Domain services pour données inter-BCs ») tranche : la
transformation des DTOs de port en objets domaine passe par un domain service
dans `use_cases/`.

## Fichiers — BC `match_report` (émetteur)

| Fichier | Nature | Contenu |
|---|---|---|
| `domain/events.rs` | modif | variant `MatchReportUnpublished { unpublished_by, unpublished_at }` |
| `domain/match_report_published.rs` | modif | `unpublish(coach_id, eligibility)` — symétrique de `MatchReportReadyToPublish::publish()` |
| `domain/match_report_state.rs` | modif | arête `Published + MatchReportUnpublished → ReadyToPublish` dans `rehydrate()` |
| `domain/value_objects.rs` | modif | VO d'éligibilité + sa raison de blocage |
| `domain/error.rs` | modif | erreur domaine de refus |
| `use_cases/correction_eligibility_service.rs` | **nouveau** | domain service, compose les 2 ports |
| `use_cases/unpublish_match_report_use_case.rs` | **nouveau** | orchestration |
| `io/web/recap_controller.rs` | modif | `post_unpublish` ; `get_recap` alimente le VM du garde-fou |
| `io/web/view_models.rs` | modif | VM de la zone de correction |
| `routes.rs` / `router.rs` | modif | route `recap_unpublish` |
| `io/app_events/app_event_publisher.rs` | modif | conversion `MatchReportUnpublished` → app events |

## Fichiers — BCs consommateurs (compensation)

| BC | Fichier | Compensation |
|---|---|---|
| `competitions` | `io/app_events/match_report_unpublished_listener.rs` (**nouveau**) | `match_status` → `in_progress`, scores et sorties à null, `match_report_url` vers l'édition. **Ne recrée aucun pairing** |
| `ranking` | `io/app_events/match_report_unpublished_listener.rs` (**nouveau**) | `DELETE` des 2 lignes du `match_report_id` |
| `teams` | `io/app_events/match_report_unpublished_listener.rs` (**nouveau**) | inverse de `PostMatchSequenceStarted` : phase → `MatchReporting`, trésorerie et fans restaurés |
| `players` | **dans** `io/app_events/player_match_impact_listener.rs` (modif) | inverse de l'impact du match sur tout l'effectif des 2 équipes |

### Pourquoi `players` n'a pas de listener dédié

C'est imposé par le code existant. Le commentaire de
`team_match_concluded_listener.rs` l'explique : les events d'impact et
`TeamMatchConcluded` touchent le même agrégat joueur avec une version optimiste,
et deux tâches concurrentes se disputeraient la même version — l'une des deux
perd la course et son event est silencieusement abandonné. La compensation doit
donc passer par la **même tâche séquentielle**.

## App events de compensation

| Event | Cardinalité | Consommé par |
|---|---|---|
| `MatchReportAppEvent::MatchReportUnpublished` | 1 | `competitions`, `ranking`, `teams` |
| `PlayerMatchImpactAppEvent::TeamMatchImpactReverted { team_id, match_report_id }` | 2 | `players` |

Deux propriétés structurantes :

**1. Le payload de compensation est léger** — identifiants seulement
(`match_report_id`, `space_id`, `competition_id`, `season_id`, `round_id`,
`pairing_id`, `home_team_id`, `away_team_id`), **pas les actions**. Chaque BC
défait ce qu'il a lui-même enregistré, via son instantané dérivé ; il ne
recalcule rien depuis le payload. C'est ce qui rend la compensation exacte même
si le payload était incohérent — et c'est la raison profonde pour laquelle cette
approche bat une propagation par deltas.

**2. Un seul event par équipe pour `players`**, pas un par action. L'agrégat
joueur porte son propre instantané « contribution du dernier match ». Symétrique
de `TeamMatchConcluded`, qui itère déjà sur tout l'effectif.

`TeamMatchImpactReverted` vit dans `player_match_impact_app_events.rs` et non
dans `match_report_app_events.rs` : le BC `players` ne connaît ni compétition ni
saison, son contrat doit rester étroit.

## Domain services

| Fichier | Rôle |
|---|---|
| `match_report/use_cases/correction_eligibility_service.rs` | compose `find_game_phase` (×2) et `has_spent_spp_since_match` (×2) en un value object domaine, incluant le nom de l'équipe qui bloque |

Aucun handler ni template ne manipule les DTOs de ces ports (cf. CLAUDE.md).

## Carte prérequis — sécurité

`post_publish` doit appeler `is_authorized()` comme le fait `get_recap`. Le
handler charge déjà l'état du rapport pour le use case, donc `RecapSource` est
constructible sans coût supplémentaire. **Carte à part, en amont de tout le
reste** : la règle 4 aligne les droits de correction sur ceux de la publication,
donc la correction hériterait du trou.

## Règle métier identifiée en phase 3

**Échec partiel de la compensation** (règle 11 du README). Le bus étant
best-effort, une compensation peut réussir dans un BC et échouer dans un autre.
Posture retenue : on l'accepte, en s'appuyant sur l'idempotence de chaque
compensation et sur une resynchronisation manuelle si nécessaire.

Ni ordonnancement des compensations par criticité, ni compensation synchrone
dans le use case — cette dernière violerait la souveraineté des données entre
BCs. C'est le seul endroit où cette feature peut laisser la base incohérente,
et c'est assumé.
