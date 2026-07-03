# Récap — Phase 6 : Domaine ✅

## Récapitulatif des règles métier (validé)

| # | Règle | Responsable |
|---|---|---|
| BR1 | Publication possible uniquement depuis `ReadyToPublish` (Draft/PreMatch → 404, `Published` → 409, `Cancelled` → 410) | **Use case** (garanti par le typestate — `publish()` n'existe que sur `MatchReportReadyToPublish`) |
| BR2 | Publication irréversible — aucune méthode ne permet de repasser en arrière ni de modifier `MatchReportPublished` | **Domaine** (typestate — `MatchReportPublished` n'expose aucune méthode de mutation) |
| BR3 | Aucune validation bloquante avant publication (`summary_title`/`summary_body` restent optionnels) | **Domaine** (`publish()` infaillible, pas de `Result`) |
| BR4 | La publication émet `MatchReportDomainEvent::MatchReportPublished` | **Domaine** |
| BR5 | Les actions **subies** (`Blesse{injury}`) ne génèrent jamais de SPP | **`spp_calculator` — hors scope d'implémentation réelle, stub pour cette carte** |
| BR6 | `injury_label` n'est jamais renseigné pour `MatchActionType::Sortie` — la carte « Bilan sanitaire » ne liste que les `Blesse{injury}` | **Use case / builders** (view models, phase 4) |
| BR7 | `result_badge` dérivé du score à l'affichage, jamais stocké | **builders.rs** |
| BR8 | Authentification requise pour `GET /recap` et `POST /recap/publish` | **Middleware existant** (`require_auth`), rien de spécifique à ajouter |

BR6-BR8 sont déjà couvertes par les phases précédentes (4-5) — ce fichier se concentre sur BR1-BR4 (nouveau domaine) et le stub BR5.

---

## Nouvel état domaine — `MatchReportPublished`

### Fichier : `src/app/match_report/domain/match_report_published.rs` (nouveau)

```rust
#[derive(Debug, Clone)]
pub struct MatchReportPublished {
    // Copie exhaustive des champs de MatchReportReadyToPublish (validé phase 3, décision Q2)
    pub id: MatchReportId,
    pub space_id: SpaceId,
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
    pub created_by: CoachId,
    pub origin: MatchReportOrigin,
    pub pairing_id: Option<String>,
    pub home_fan_roll: Option<D3Roll>,
    pub away_fan_roll: Option<D3Roll>,
    pub home_dedicated_fans: u32,
    pub away_dedicated_fans: u32,
    pub home_inducements: Option<Vec<InducementPurchase>>,
    pub away_inducements: Option<Vec<InducementPurchase>>,
    pub star_engagements: Vec<(TeamId, InducementId)>,
    pub home_temp_players: Vec<TempPlayer>,
    pub away_temp_players: Vec<TempPlayer>,
    pub home_actions: Vec<MatchAction>,
    pub away_actions: Vec<MatchAction>,
    pub home_gain: MatchGain,
    pub away_gain: MatchGain,
    pub home_fan_mod: FanFactorMod,
    pub away_fan_mod: FanFactorMod,
    pub summary_title: Option<String>,
    pub summary_body: Option<String>,
    pub version: u64,
    // Nouveau
    pub published_by: CoachId,
    pub published_at: DateTime<Utc>,
}
```

### Méthode domaine — `MatchReportReadyToPublish::publish()`

Fichier : `src/app/match_report/domain/match_report_ready_to_publish.rs`

```rust
impl MatchReportReadyToPublish {
    pub fn publish(&self, published_by: CoachId) -> (MatchReportPublished, MatchReportDomainEvent) {
        let published_at = Utc::now();
        let event = MatchReportDomainEvent::MatchReportPublished {
            published_by: published_by.clone(),
            published_at,
        };
        let published = MatchReportPublished::from_ready_to_publish(self, published_by, published_at);
        (published, event)
    }
}
```

Infaillible (pas de `Result`) — cohérent avec BR3 et avec le pattern déjà utilisé par `record_fan_factor`/`record_team_values` (aucune invariant à vérifier, pas de `DomainError` possible).

`MatchReportPublished::from_ready_to_publish` — constructeur miroir de `MatchReportReadyToPublish::from_pre_match`, copie tous les champs + ajoute `published_by`/`published_at`. `version` incrémenté de 1.

### Nouveau variant `MatchReportDomainEvent`

Fichier : `src/app/match_report/domain/events.rs`

```rust
MatchReportPublished {
    published_by: CoachId,
    published_at: DateTime<Utc>,
},
```

+ entrée dans `type_name()` (`"MatchReportPublished"`) et `schema_version()` (`"1.0"`), même pattern que les variants existants.

### Nouveau variant `MatchReportState`

Fichier : `src/app/match_report/domain/match_report_state.rs`

```rust
pub enum MatchReportState {
    Draft(MatchReportDraft),
    PreMatch(MatchReportPreMatch),
    ReadyToPublish(MatchReportReadyToPublish),
    Published(MatchReportPublished),   // NOUVEAU
    Cancelled(MatchReportCancelled),
}
```

Branche de réhydratation ajoutée dans `rehydrate()`, miroir de la transition `PreMatch → ReadyToPublish` existante (via `PostMatchRecorded`) :

```rust
(
    Some(MatchReportState::ReadyToPublish(rtp)),
    MatchReportDomainEvent::MatchReportPublished { published_by, published_at },
) => MatchReportState::Published(
    MatchReportPublished::from_ready_to_publish(&rtp, published_by.clone(), *published_at)
),
```

Pas de branche `Published → autre chose` : BR2 (irréversibilité) est garantie par l'absence de tout event/transition sortant de `Published` dans `rehydrate()`.

---

## `spp_calculator` — domaine stubbé (BR5, hors scope réel)

Fichier : `src/app/spp_calculator/domain/calculator.rs`

```rust
pub fn calculate(home_actions: &[SppActionInput], away_actions: &[SppActionInput]) -> SppCalculationResult {
    // STUB — retourne une valeur plausible (10 SPP) pour chaque joueur distinct ayant
    // agi, home et away confondus, quelle que soit l'action. Permet à la carte
    // "Performances (SPP)" de s'afficher avec des lignes pendant l'implémentation,
    // sans encoder la vraie règle de calcul (carte dédiée, hors scope de cette page).
    const STUB_SPP: u8 = 10;
    let home = distinct_actors(home_actions).map(|actor| (actor, STUB_SPP)).collect();
    let away = distinct_actors(away_actions).map(|actor| (actor, STUB_SPP)).collect();
    SppCalculationResult { home, away }
}
```

Seule règle figée pour l'instant (BR5) : `Blesse{injury}` ne doit jamais apparaître comme actor générateur de SPP — le stub doit donc exclure les entrées `SppActionInput` correspondant à une blessure subie avant de dédupliquer les acteurs (sinon un joueur blessé mais jamais acteur d'une autre action recevrait quand même 10 SPP à tort). Pas de test sur la valeur `10` elle-même (arbitraire) ; un test vérifie que BR5 est respectée même par le stub.

`IRosterSppPort` reste défini (phase 3/4) mais n'est pas appelé par le stub — l'adapter `roster_spp_adapter.rs` peut être implémenté minimalement (retourne `vec![]`) ou différé lui aussi à la carte dédiée, au choix de l'implémentation (phase 8, découpage des cartes).

---

## Erreurs domaine

Aucun nouveau variant `DomainError` — `publish()` est infaillible. Les erreurs de la Phase 5 (`PublishMatchReportError::NotFound/AlreadyPublished/Cancelled`) sont des erreurs **applicatives** (use case), pas domaine : elles constatent un état incompatible avant même d'appeler `publish()`, elles ne remontent jamais depuis le domaine.

---

## Tests unitaires prévus

Tous dans `#[cfg(test)]` de `match_report_ready_to_publish.rs` (nouveau module de tests, le fichier n'en a pas encore) et `match_report_state.rs` (module existant) :

```rust
#[test]
fn publish_produces_published_state_with_all_fields_copied() {
    // rtp avec summary_title/body renseignés → published.summary_title == rtp.summary_title, etc.
    // published.published_by == coach fourni, published_at proche de Utc::now()
}

#[test]
fn publish_succeeds_without_summary() {
    // rtp avec summary_title: None, summary_body: None → publish() réussit quand même (BR3)
}

#[test]
fn publish_increments_version() {
    // published.version == rtp.version + 1
}

#[test]
fn rehydrate_ready_to_publish_then_published_yields_published_state() {
    // events: [..., PostMatchRecorded, MatchReportPublished] → rehydrate() donne MatchReportState::Published
}
```

Test dédié au stub, dans `spp_calculator/domain/calculator.rs` :

```rust
#[test]
fn calculate_stub_never_credits_spp_to_an_injury_only_actor() {
    // un actor n'ayant qu'une action Blesse{injury} (aucune autre action) → absent du résultat
}

#[test]
fn calculate_stub_credits_flat_spp_to_other_actors() {
    // un actor ayant au moins une action non-Blesse → présent avec STUB_SPP (10)
}
```

Pas de test « publish depuis Draft/PreMatch/Cancelled échoue » côté domaine — ces cas n'existent pas au niveau du type (`publish()` n'est pas une méthode de `MatchReportDraft`/`MatchReportPreMatch`/`MatchReportCancelled`), donc rien à tester domaine ; c'est testé côté use case (Phase 7/8) via le pattern-matching sur `MatchReportState`.

---

## Résumé des fichiers créés/modifiés

| Fichier | Nature |
|---|---|
| `src/app/match_report/domain/match_report_published.rs` | **Nouveau** — struct `MatchReportPublished` + `from_ready_to_publish()` |
| `src/app/match_report/domain/match_report_ready_to_publish.rs` | Ajout méthode `publish()` + tests |
| `src/app/match_report/domain/events.rs` | Ajout variant `MatchReportPublished { published_by, published_at }` + `type_name()`/`schema_version()` |
| `src/app/match_report/domain/match_report_state.rs` | Ajout variant `Published(MatchReportPublished)` + branche `rehydrate()` + test |
| `src/app/spp_calculator/domain/calculator.rs` | **Nouveau, stub** — `calculate()` retourne un résultat vide |
