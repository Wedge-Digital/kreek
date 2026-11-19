# BC match_report — Page inducements GET + POST

**Priorité : haute**
**Dépend de :** 109, 110, 111
**Contexte :** match_report step2-inducements — handlers + template

## Objectif

Implémenter les handlers GET (affichage de la page) et POST (enregistrement des achats) pour la page d'achat des inducements, et créer le template Askama.

## Conception

Cf. `docs/specs/match-report/step2-inducements/03-back.md`, `04-dtos.md`

### Nouveau fichier `io/web/inducements_controller.rs`

#### Handler GET `get_inducements`

1. Extrait `match_report_id`, `team_id` du path
2. Charge l'agrégat → `PreMatch` avec TV présentes
3. Fetch `find_team_info(team_id)` → `team_name`, `roster_id`
4. Fetch `find_tier_rules_for_roster(season_id, roster_id)` → `TierRulesDto`
5. Fetch `find_team_treasury(team_id)` → budget brut
6. Calcule `budget = pm.inducement_budget_for(&team_id, treasury)`
7. Construit `inducement_selector_url` avec params CSV des UIDs + `roster_id` + `instance_id`
8. Rend `InducementsTemplate`

#### Handler POST `post_inducements`

1. Parse `RecordInducementsForm` (champ `selection` JSON)
2. Désérialise `Vec<InducementPurchaseInput>` depuis `selection`
3. Construit `RecordInducementsCommand` via smart constructors
4. Appelle `record_inducements_use_case::execute()`
5. Selon `RecordInducementsOutcome` : redirect HX vers page équipe suivante ou step3

### Nouveau template `io/web/templates/inducements.html`

```rust
pub struct InducementsTemplate {
    pub app_routes:              AppRoutes,
    pub space_id:                String,
    pub match_report_id:         String,
    pub team_id:                 String,
    pub team_name:               String,
    pub team_initials:           String,
    pub order_label:             String,
    pub budget:                  u32,
    pub budget_label:            String,
    pub inducement_selector_url: String,
    pub form_action:             String,
    pub pass_url:                String,
}
```

Structure de la page :
- Header : nom de l'équipe + badge ordre (TopDog/Underdog)
- Bannière budget : montant disponible + label contextuel
- Zone widget : `<div hx-get="{{ inducement_selector_url }}" hx-trigger="load">`
- Footer sticky : cart Alpine réactif à `inducementSelectionChanged` + bouton Valider (disabled si budget dépassé) + lien Passer

### Routes

Dans `routes.rs` BC MatchReport :

```rust
pub fn step2_inducements(&self, space_id: &str, mr_id: &str, team_id: &str) -> String
```

Dans `io/web/mod.rs` BC MatchReport : enregistrer GET + POST sur `/app/:space_id/match-report/:mr_id/step2/inducements/:team_id`.

## Checklist

- [ ] `get_inducements` handler (≤ 20 lignes — découper en `build_vm`, etc.)
- [ ] `post_inducements` handler (≤ 20 lignes)
- [ ] `RecordInducementsForm` + désérialisation `selection`
- [ ] `InducementsTemplate` struct
- [ ] Template : header, bannière budget, zone widget hx-get, footer cart Alpine
- [ ] Alpine cart : écoute `inducementSelectionChanged`, met à jour total, désactive Valider si dépassement
- [ ] Bouton "Passer" → GET vers URL étape suivante (TopDog → Underdog ou Underdog → Step 3)
- [ ] Route `step2_inducements` dans `routes.rs`
- [ ] Enregistrement route dans `mod.rs`
- [ ] Pas de styles inline — classes CSS uniquement
