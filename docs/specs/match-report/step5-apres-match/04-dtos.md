# Step 5 — Contrats de données

## Value objects à créer (domaine)

Ces types vivent dans `domain/value_objects.rs`. Ils sont utilisés côté commande uniquement.

```rust
// Gain de match par équipe — doit être > 0
#[nutype(validate(greater = 0), derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize))]
pub struct MatchGain(u32);

// Modification du fan factor — entre -2 et +2 inclus
#[nutype(validate(greater_or_equal = -2, less_or_equal = 2), derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize))]
pub struct FanFactorMod(i8);
```

`summary_title` et `summary_body` restent `Option<String>` — texte libre sans invariant domaine.

---

## DTO d'entrée POST

**Émis par** : formulaire HTML `step5.html`
**Consommé par** : handler `post_step5`

```rust
// io/web/step5_controller.rs
#[derive(Deserialize)]
pub struct RecordPostMatchForm {
    pub home_gain: u32,
    pub away_gain: u32,
    pub home_fan_mod: i8,
    pub away_fan_mod: i8,
    pub summary_title: Option<String>,
    pub summary_body: Option<String>,
}
```

Le handler construit les value objects via leurs smart constructors avant d'appeler le use case.

---

## Commande use case

**Émise par** : handler `post_step5`
**Consommée par** : `record_post_match_use_case::execute`

```rust
// use_cases/record_post_match_use_case.rs
pub struct RecordPostMatchCommand {
    pub match_report_id: MatchReportId,
    pub home_gain: MatchGain,
    pub away_gain: MatchGain,
    pub home_fan_mod: FanFactorMod,
    pub away_fan_mod: FanFactorMod,
    pub summary_title: Option<String>,
    pub summary_body: Option<String>,
    pub recorded_by: CoachId,
}
```

---

## Événement domaine

**Émis par** : méthode `MatchReportPreMatch::record_post_match()`
**Persisté dans** : event store
**Réhydraté par** : `match_report_state::rehydrate()` → produit `MatchReportReadyToPublish`

```rust
// domain/events.rs
PostMatchRecorded {
    home_gain: MatchGain,
    away_gain: MatchGain,
    home_fan_mod: FanFactorMod,
    away_fan_mod: FanFactorMod,
    summary_title: Option<String>,
    summary_body: Option<String>,
    recorded_by: CoachId,
}
```

---

## DTO de sortie GET (template)

**Émis par** : handler `get_step5`
**Consommé par** : template `step5.html`

```rust
// io/web/step5_controller.rs
#[derive(Template)]
#[template(path = "step5.html")]
pub struct Step5Template {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub match_report_id: String,
    pub form_action: String,
    pub back_url: String,
    // Score banner
    pub home_team_name: String,
    pub away_team_name: String,
    pub home_initials: String,
    pub away_initials: String,
    pub home_logo_url: Option<String>,
    pub away_logo_url: Option<String>,
    pub home_score: u8,
    pub away_score: u8,
    pub home_cas: u8,
    pub away_cas: u8,
    // Formulaire
    // Pré-rempli avec valeurs existantes si ReadyToPublish, sinon suggestion
    pub home_gain: u32,
    pub away_gain: u32,
    pub home_fan_mod: i8,
    pub away_fan_mod: i8,
    pub summary_title: Option<String>,
    pub summary_body: Option<String>,
    pub already_recorded: bool,
}
```

`home_gain` / `away_gain` : le handler pré-calcule la valeur à afficher — valeur existante si le rapport est déjà en `ReadyToPublish`, sinon résultat de `suggest_gains()`.

`home_fan_mod` / `away_fan_mod` : défaut à `0` si pas encore saisi.

---

## Précisions

- `compute_cas()` compte uniquement les actions `MatchActionType::Sortie` — pas les `Blesse { .. }`
- Les VMs sont des primitives (`u8`, `i8`, `u32`, `String`) : la couche template ne manipule jamais les value objects domaine
