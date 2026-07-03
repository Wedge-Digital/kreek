# Hand-off — Page Récap du match report

## État d'avancement

**Workflow new-feature terminé — phases 1 à 8 validées et écrites.** Les 7 cartes kanban
(144-150) sont dans `kanban/ready_to_be_done/`, prêtes à être implémentées carte par carte
(cf. `08-cards.md` pour l'ordre). Ce fichier reste comme mémoire des décisions prises pendant
la conception — plus besoin de reprendre une phase, seulement de suivre le protocole de
démarrage de carte (CLAUDE.md) pour chaque carte 144-150.

---

## Décisions validées

### Maquette (Phase 1)
- Maquette existante : `assets/rawpages/html/app-match-summary.html` — validée
- **Fan Factor avant/après supprimé** de la sidebar (pas info primordiale)
- **KO supprimé** du bilan sanitaire — `Sortie` n'est pas un KO trackable. Seuls les `Blesse { InjuryType }` : Commotion, Amoché, Blessure Sérieuse, Séquelle, Mort
- **Deux états CTAs** sur la même page :
  - `ReadyToPublish` → "Publier" + "← Modifier étape 5"
  - `Published` → "Retour compétition" + "Voir fiche [équipe home]"
- **Accès** : login requis (accès public = temps 2)

### Architecture (Phase 2)
- Page simple (pas de widgets multi-HTMX) — handler unique côté serveur
- Sources de données : repo local + `ITeamDataPort` (existant) + `ICompetitionDataPort` (nouvelle méthode) + `ISppCalculatorPort` (nouveau mini BC)
- **Mini BC `SppCalculator`** validé : BC autonome qui prend actions + roster_ids → SPP par joueur. Fetche les `SppRules` depuis BC References via `IRosterSppPort`. Fonction de calcul pure.
- **BC Players recalcule de son côté** après publication (n'utilise pas les SPP du récap)

### Modèle SPP
- `Sortie` = action **infligée** → l'acteur gagne des SPP
- `Blesse { injury }` = action **subie** → pas de SPP (bilan sanitaire uniquement)
- Multiplicateurs dépendent du roster : TD et Sortie ont des valeurs différentes selon équipe "normale" vs "brutale"
- `SppRules` viennent des règles spéciales de ligue dans la définition de référence des rosters (BC References)

### AppEvent `MatchReportPublished`
Structure validée :
```rust
pub struct MatchReportPublishedPayload {
    pub match_report_id: String,
    pub space_id:        String,
    pub competition_id:  String,
    pub season_id:       String,
    pub round_id:        String,
    pub pairing_id:      Option<String>,
    pub published_at:    DateTime<Utc>,
    pub home_team_id:    String,
    pub away_team_id:    String,
    pub home_score:      u8,
    pub away_score:      u8,
    pub home_gain_kpo:   u32,
    pub away_gain_kpo:   u32,
    pub home_fan_mod:    i8,
    pub away_fan_mod:    i8,
    pub home_actions:    Vec<MatchActionPublishedPayload>,
    pub away_actions:    Vec<MatchActionPublishedPayload>,
    // temp players inclus (pour BC Players)
    pub home_temp_players: Vec<TempPlayerPayload>,
    pub away_temp_players: Vec<TempPlayerPayload>,
}

pub struct MatchActionPublishedPayload {
    pub turn:   u8,
    pub player: PlayerRefPayload,
    pub action: ActionTypePayload,
}

pub enum PlayerRefPayload {
    Regular  { player_id: String },
    Star     { ref_uid: String, display_name: String },
    Mercenary,
    Journalier,
}
```
BC Teams et BC Players s'abonnent à cet AppEvent.

---

## Phase 3 — Proposition en attente de validation

### Nouveaux fichiers

**BC `match_report`**

| Fichier | Rôle |
|---|---|
| `io/web/recap_controller.rs` | GET + POST handlers |
| `io/web/templates/recap.html` | Template Askama |
| `assets/static/css/pages/match-report-recap.css` | Styles |
| `use_cases/publish_match_report_use_case.rs` | Orchestration publication |
| `domain/match_report_published.rs` | Struct + méthode domaine |

**Mini BC `spp_calculator`** (nouveau)

| Fichier | Rôle |
|---|---|
| `src/app/spp_calculator/mod.rs` | Module public |
| `src/app/spp_calculator/domain/spp_rules.rs` | `SppRules` struct |
| `src/app/spp_calculator/domain/calculator.rs` | Fonction pure `calculate()` |
| `src/app/spp_calculator/ports.rs` | `IRosterSppPort` trait |

**Infrastructure**

| Fichier | Rôle |
|---|---|
| `src/infrastructure/match_report/spp_calculator_adapter.rs` | Implémente `ISppCalculatorPort` |
| `src/infrastructure/spp_calculator/roster_spp_adapter.rs` | Implémente `IRosterSppPort` → BC References |

### Nouvelles routes
```
GET  /app/{space_id}/match-report/{match_report_id}/recap
POST /app/{space_id}/match-report/{match_report_id}/recap/publish
```

### Ports à créer / étendre
| Port | BC | Changement |
|---|---|---|
| `ICompetitionDataPort` | match_report | + `find_round_context(…) -> Option<RoundContextDto>` |
| `ISppCalculatorPort` | match_report | Nouveau |
| `IRosterSppPort` | spp_calculator | Nouveau |

### Domaine à créer
- `MatchReportState::Published(MatchReportPublished)` — nouveau variant
- `MatchReportDomainEvent::MatchReportPublished { … }` — nouveau event
- `MatchReportReadyToPublish::publish()` → `(MatchReportPublished, DomainEvent)`
- Nouveau bras de rehydratation dans `match_report_state.rs`

### Questions résolues (voir `03-back.md`)
1. **BC References port** — Pas de port/donnée SPP existant côté `references`. Décision : `IRosterSppPort` reste un simple accès à `special_rules` (déjà exposé par `find_team_by_uid`, sans nouvelle méthode côté `references`). Les 2 rulesets SPP (Normal/Brutal) sont des données propres au mini BC `spp_calculator` (fichier JSON dédié, chargé en interne), et la logique de sélection du ruleset vit dans `spp_calculator::domain::calculator` — point d'extension unique pour une variante future.
2. **`MatchReportPublished` struct** — validé : tous les champs de `ReadyToPublish` + `published_at: DateTime<Utc>`.
3. **Redirect post-publication** — validé : redirect direct vers `GET /recap` (état `Published`), pas d'écran de confirmation séparé.

---

## État d'avancement (mise à jour)

Phase 4 (DTOs) **validée et écrite** (`04-dtos.md`) — inclut un nouveau port `ICoachDataPort` (match_report → spaces) découvert pendant cette phase, pour résoudre la byline « Soumis par {coach} ».

Phase 5 (use cases) **validée et écrite** (`05-use-cases.md`) : le BC `match_report` n'avait pas l'infra bus-interne/publisher décrite par le CLAUDE.md (écart déjà présent dans `create_match_report_use_case.rs`, non corrigé, non touché). Décision validée : mettre `publish_match_report_use_case.rs` en conformité — nouveau bus interne + `app_event_publisher.rs` pour `match_report`, détaillés en phase 7.

## État d'avancement (mise à jour 2)

Phase 6 (domaine) **validée et écrite** (`06-domaine.md`). Décision notable : la règle SPP initialement proposée (« Sortie et Touchdown génèrent des SPP ») était **fausse** — corrigée par l'utilisateur (Passe, Lancer, Sortie, TD, Interception, MVP, actions bonus — règles réelles non figées). **Le calcul SPP réel est descopé de cette carte** : `spp_calculator::domain::calculator::calculate()` est un **stub** qui retourne 0 SPP partout. Seule règle figée : `Blesse{injury}` ne génère jamais de SPP (trivial avec un stub). Une carte dédiée traitera le calcul réel plus tard.

Nouveau domaine ajouté : `MatchReportPublished` (état terminal, copie exhaustive de `ReadyToPublish` + `published_by`/`published_at`), méthode `MatchReportReadyToPublish::publish()` (infaillible), variant `MatchReportDomainEvent::MatchReportPublished`, variant `MatchReportState::Published`.

## État d'avancement (mise à jour 3)

Phase 7 (intégration) **validée et écrite** (`07-integration.md`). Aucune migration SQL nécessaire. Confirmé : `IRosterSppPort`/`roster_spp_adapter.rs`/`spp_rules.json` ne sont **pas créés** dans cette carte (aucun appelant tant que le calcul SPP est stubbé) — seul `ISppCalculatorPort` + un adapter/`calculate()` stub existent.

**Point technique laissé ouvert pour la Phase 8** : le domain event `MatchReportPublished` (delta minimal : `published_by`/`published_at`) ne porte pas assez de données pour construire directement le riche `MatchReportPublishedPayload` de l'AppEvent (qui a besoin des actions, temp players, etc.). Deux options possibles à trancher en découpant les cartes : (a) le use case enrichit l'event avant publication sur le bus interne, ou (b) le publisher recharge l'état complet via `repo.find_by_id` avant de mapper. Aucune des deux ne change le contrat externe.

Stub `calculate()` ajusté sur demande : renvoie une valeur plausible (10 SPP) par acteur distinct plutôt que 0/vide, en excluant les acteurs n'ayant que des actions `Blesse{injury}` (BR5 respectée même par le stub) — permet à la carte "Performances (SPP)" de s'afficher avec des lignes pendant l'implémentation.

**Note pour la future carte « calcul SPP réel »** : `assets/references/spp_rules.json` existe déjà (créé en avance) avec les vraies valeurs Normal / Brawlin' Brutes (TD, CAS, REU, MVP, INT, TTM). Confirmé avec l'utilisateur que ce n'est que de la préparation — pas utilisé dans cette carte. À reprendre (et probablement déplacer vers `spp_calculator`, cf. décision phase 3) au moment de cette carte dédiée.

## Prochaines étapes à la reprise

1. Phase 8 (cartes kanban) — découper l'implémentation, en isolant bien :
   - la carte « calcul SPP réel » (hors scope, différée — réutilisera `assets/references/spp_rules.json` déjà préparé)
   - le point technique domain event → AppEvent enrichi (à trancher au moment de la carte concernée)
2. Implémenter carte par carte
