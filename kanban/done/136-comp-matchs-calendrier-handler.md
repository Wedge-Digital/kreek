# 136 — Handler + template onglet Calendrier

## Objectif

Implémenter le controller et le template Askama pour l'onglet Calendrier (matchs `upcoming`, scroll infini, du plus proche au plus lointain).

## Dépendances

- 134 (méthodes repository disponibles)

## Conception détaillée

### `io/web/calendrier_tab_controller.rs` (nouveau fichier)

Structure identique à `resultats_tab_controller.rs`, avec :
- Appel à `match_day_repo.list_calendrier(season_id, query.cursor, 3)`
- `next_cursor` = `round_position` de la 3ème journée si exactement 3 journées retournées
- Template `CalendrierTabTemplate` avec `#calendrier-list` comme conteneur cible

**VMs** :

```rust
pub struct MatchCalendrierVm {
    pub home_name: String,
    pub home_logo: Option<String>,
    pub home_initials: String,
    pub away_name: String,
    pub away_logo: Option<String>,
    pub away_initials: String,
    pub date: String,
}

pub struct JourneeCalendrierVm {
    pub label: String,
    pub date_range: String,   // calculé depuis round_day_type + round_date_start/end
    pub match_count: usize,
    pub matches: Vec<MatchCalendrierVm>,
}
```

**Calcul de `date_range`** (fonction privée dans le controller) :

```rust
fn format_date_range(day_type: &str, start: Option<&str>, end: Option<&str>) -> String {
    match day_type {
        "fixed_date" => start.unwrap_or("").to_string(),
        "time_frame" => match (start, end) {
            (Some(s), Some(e)) => format!("{} – {}", s, e),
            (Some(s), None) => s.to_string(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}
```

### `templates/competition-tab-calendrier.html` (nouveau fichier)

Structure :
- `{% if is_initial %}<div id="calendrier-list">{% endif %}`
- Boucle sur `journees` → section block par journée
- Header journée : label + `date_range` + compteur
- Chaque ligne : `.cal-row` avec logos 38px, "VS" centré, date
- Pas de score, pas de badge statut
- `{% if let Some(cursor) = next_cursor %}` → sentinel HTMX
- `{% if is_initial %}</div>{% endif %}`

Réutiliser les classes CSS de la maquette : `.cal-row`, `.cal-team`, `.cal-logo`, `.cal-center`, `.cal-vs`, `.cal-date`, `.scroll-sentinel`.

## Checklist

- [ ] `calendrier_tab_controller.rs` créé
- [ ] VMs définis et mapping `PairingDisplayDto` → `JourneeCalendrierVm`
- [ ] `format_date_range()` implémentée et testée
- [ ] `next_cursor` calculé correctement
- [ ] Template `competition-tab-calendrier.html` créé
- [ ] Équipes visiteuses alignées à droite (`.cal-team-away`)
- [ ] Fallback navigation directe (full page) fonctionnel
- [ ] `cargo build` passe
