# Récap — Phase 3 : Architecture back

## BC responsables

- **`match_report`** : fournit la page (handler unique, pas de widgets — cf. 02-front.md).
- **`spp_calculator`** (nouveau mini BC autonome) : calcule les SPP gagnés par joueur sur le match, à partir de ses propres règles internes.

Aucun widget — page GET classique + un seul POST de mutation (publication).

## Routes

```
GET  /app/{space_id}/match-report/{mr_id}/recap
POST /app/{space_id}/match-report/{mr_id}/recap/publish
```

## Fichiers

```
src/app/match_report/
├── io/web/
│   ├── recap_controller.rs             ← GET get_recap + POST post_publish
│   └── templates/
│       └── recap.html                  ← template Askama (page complète)
├── use_cases/
│   └── publish_match_report_use_case.rs ← orchestration POST publish
├── domain/
│   └── match_report_published.rs       ← struct MatchReportPublished + méthode publish()
└── ports.rs                            ← + méthode find_round_context sur ICompetitionDataPort
                                           + trait ISppCalculatorPort (match_report → spp_calculator)
                                           + trait ICoachDataPort (match_report → spaces, résolution created_by → nom)

assets/static/css/pages/
└── match-report-recap.css

src/app/spp_calculator/                 ← nouveau mini BC
├── mod.rs
├── domain/
│   ├── spp_rules.rs                    ← struct SppRules, enum SppRuleset { Normal, Brutal }
│   └── calculator.rs                   ← fonction pure calculate() + sélection de ruleset
├── io/repository/
│   └── in_memory_spp_rules_repository.rs ← charge assets/spp_calculator/spp_rules.json (include_str!, même pattern que references)
└── ports.rs                            ← trait IRosterSppPort (spp_calculator → references)

assets/spp_calculator/
└── spp_rules.json                      ← 2 entrées : Normale, Brutale (td_spp, sortie_spp)

src/infrastructure/
├── match_report/
│   ├── spp_calculator_adapter.rs       ← implémente ISppCalculatorPort en appelant spp_calculator
│   └── coach_data_adapter.rs           ← implémente ICoachDataPort en appelant ISpaceUserCacheRepository (BC spaces, existant)
└── spp_calculator/
    └── roster_spp_adapter.rs           ← implémente IRosterSppPort en appelant IReferenceRepository::find_team_by_uid (existant)

main.rs                                 ← instancie les 2 nouveaux adapters, injecte dans les contextes
```

## Ports nécessaires

| Port | Propriétaire | Appelé par | Méthode | Retour |
|---|---|---|---|---|
| `ICompetitionDataPort` (étendu) | `match_report` | `recap_controller::get_recap` | `find_round_context(season_id, round_id) -> Option<RoundContextDto>` | Noms compétition/saison/journée — dégradation gracieuse si absent (détail des champs en phase 4) |
| `ISppCalculatorPort` (nouveau) | `match_report` | `recap_controller::get_recap` | `calculate_match_spp(home_actions, away_actions, home_roster_id, away_roster_id) -> SppMatchResult` | Adapter appelle directement le mini BC `spp_calculator` (in-process) |
| `IRosterSppPort` (nouveau, réduit) | `spp_calculator` | `spp_calculator::domain::calculator` | `find_special_rules(roster_id) -> Vec<String>` | Réutilise `IReferenceRepository::find_team_by_uid(roster_id).special_rules` — **aucune nouvelle méthode côté `references`** |
| `ICoachDataPort` (nouveau) | `match_report` | `recap_controller::get_recap` (via `builders.rs`) | `find_coach_name(coach_id) -> Option<String>` | Réutilise `ISpaceUserCacheRepository::find_user_by_id` (BC `spaces`, existant) — pour la byline « Soumis par {coach} » (`created_by`) |

### Décision de conception — SppRules (validée)

Les 2 jeux de règles SPP (Normale / Brutale) sont des données propres au mini BC `spp_calculator`, pas à `references` :
- Chargées en interne par `spp_calculator` depuis `assets/spp_calculator/spp_rules.json` (pattern `include_str!`, identique à `InMemoryReferenceRepository`)
- `references` n'expose que ce qu'il a déjà (`special_rules: Vec<String>` du roster) via `IRosterSppPort` — pas de méthode ni de donnée SPP ajoutée côté `references`
- La logique de sélection (ex. présence de `BRAWLIN_BRUTES` → ruleset Brutal) vit dans `spp_calculator::domain::calculator` — point d'extension unique si une nouvelle variante doit être ajoutée demain (ex. un roster avec une autre special rule mappée vers un 3ᵉ ruleset)

## Domain services nécessaires

- **GET `/recap`** : composition pure lecture — `recap_controller.rs` assemble l'état local (`MatchReportReadyToPublish`/`MatchReportPublished`) + `ITeamDataPort::find_team_info` (existant) + `ICompetitionDataPort::find_round_context` (nouveau) + `ISppCalculatorPort::calculate_match_spp` (nouveau). Les VMs dépendant de ports vivent dans `builders.rs` (convention CLAUDE.md), pas dans `view_models.rs`.
- **POST `/recap/publish`** : `publish_match_report_use_case.rs` — charge `MatchReportReadyToPublish`, appelle la méthode domaine `publish()` (→ `MatchReportPublished` + `MatchReportDomainEvent::MatchReportPublished`), persiste, retourne l'outcome. Le controller traduit l'outcome en `Redirect` vers `GET /recap` (même état, redirigé — cf. décision ci-dessous).
- **`spp_calculator`** : `domain/calculator.rs` contient la fonction pure `calculate()` — reçoit les actions + le ruleset déjà résolu (pas de dépendance port dans le domaine, cf. règle CLAUDE.md « le domaine n'accède pas aux ports »). La résolution roster → ruleset (via `IRosterSppPort`) se fait en amont, côté use case/service applicatif du mini BC.

## Décisions validées à cette étape (récap Q1-Q3)

1. **SppRules** : fichier dédié dans `spp_calculator` avec 2 rulesets (Normal/Brutal), chargés et sélectionnés en interne par `spp_calculator` — `references` n'est interrogé que pour les `special_rules` déjà existantes du roster.
2. **`MatchReportPublished`** : reprend tous les champs de `MatchReportReadyToPublish` + `published_at: DateTime<Utc>` (cohérent avec le pattern `PreMatch` → `ReadyToPublish` déjà en place).
3. **Redirect post-publication** : `POST /recap/publish` redirige directement vers `GET /recap` (état `Published`) — pas d'écran de confirmation séparé.

## Règles d'architecture applicables (rappel CLAUDE.md)

- `app_event_bus` n'est jamais passé à un use case — le publisher du BC `match_report` convertit `MatchReportDomainEvent::MatchReportPublished` en `AppEvent` (structure validée en phase 2/hand-off).
- Aucun import direct de `references` en dehors de `src/infrastructure/spp_calculator/roster_spp_adapter.rs`.
- Aucun import direct de `spaces` en dehors de `src/infrastructure/match_report/coach_data_adapter.rs`.
- `spp_calculator` ne connaît jamais `match_report` — seul `src/infrastructure/match_report/spp_calculator_adapter.rs` importe les deux.
