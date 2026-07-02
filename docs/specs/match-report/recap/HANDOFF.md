# Hand-off — Page Récap du match report

## État d'avancement

Workflow new-feature en cours. Phase 2 (architecture front) **validée et écrite** (`02-front.md`).
Phase 3 (architecture back) **proposée, en attente de validation**.

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

### Questions en suspens avant de valider Phase 3
1. **BC References port** — `IRosterSppPort` doit appeler BC References. Ce port existe-t-il déjà avec un adapter, ou faut-il créer de zéro ?
2. **`MatchReportPublished` struct** — conserve-t-elle tous les champs de `ReadyToPublish` + `published_at` ? (probablement oui pour ré-afficher le récap)
3. **Redirect post-publication** — après POST `/publish`, on redirige vers `GET /recap` (état Published). Pas de page de confirmation séparée ?

---

## Prochaines étapes à la reprise

1. Répondre aux 3 questions de la Phase 3 ci-dessus
2. Valider la Phase 3 → écrire `03-back.md`
3. Enchaîner Phase 4 (DTOs), 5 (use cases), 6 (domaine), 7 (intégration), 8 (cartes kanban)
4. Implémenter carte par carte
