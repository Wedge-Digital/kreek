# Classement — Phase 4 : Contrats de données

## DTO d'entrée

Pas de struct dédiée — extracteur `Path<(String, String, String)>` = `(space_id, competition_id, season_id)`, tous primitifs (lecture pure, GET sans query param).

## DTOs de port (`ranking/ports.rs`)

```rust
pub struct RankingRulesInfo {
    pub win_points:  u32,
    pub draw_points: u32,
    pub lose_points: u32,
}

pub struct EnrolledTeamInfo {
    pub team_id:   String,
    pub team_name: String,
}
```

## DTO de repository interne (`ranking/ports.rs`, lecture)

```rust
/// Dernière ligne de classement enregistrée pour une équipe dans une saison —
/// contient les compteurs cumulés depuis le début de la saison, pas seulement
/// les points. Une seule ligne par équipe est retournée (la plus récente).
pub struct RankingLineRow {
    pub team_id:        String,
    pub matches_played: u32,
    pub wins:            u32,
    pub draws:            u32,
    pub losses:           u32,
    pub ranking_points:   u32,
}
```

**Confirmé** : la ligne de classement stocke aussi les compteurs cumulés MJ/V/N/D, pas seulement les points de classement — calculés en même temps, jamais recalculés à la lecture par agrégation de plusieurs lignes (cohérent avec la règle "on ne prend que la dernière ligne").

## VMs de sortie (`ranking/io/web/widgets/classement_widget.rs`)

```rust
pub struct ClassementRowVm {
    pub rank:       u32,
    pub team_name:  String,
    pub played:     u32,
    pub wins:       u32,
    pub draws:      u32,
    pub losses:     u32,
    pub points:     u32,
}

pub struct ClassementWidgetVm {
    pub rules_missing:      bool,   // true → état erreur
    pub has_enrolled_teams: bool,   // false → état vide "aucune équipe"
    pub rows:                Vec<ClassementRowVm>,  // vide + has_enrolled_teams=true → état vide "aucun match joué"
}
```

Template `classement-widget.html` — ordre de décision :
```
{% if rules_missing %}           erreur
{% else if !has_enrolled_teams %} vide : aucune équipe
{% else if rows.is_empty() %}     vide : aucun match joué
{% else %}                        tableau
```

`ClassementRowVm` dépend d'un DTO de port (`EnrolledTeamInfo`, pour le nom d'équipe) en plus de `RankingLineRow` (domaine/projection) → construit dans `builders.rs`, pas de `from_domain()` sur le VM (règle CLAUDE.md).

## Interfaces d'utilisation (émetteur → consommateur)

| DTO | Émetteur | Consommateur |
|---|---|---|
| `RankingRulesInfo` | `RankingCompetitionAdapter::find_ranking_rules` (`infrastructure/ranking/`) | `classement_widget` handler — teste seulement `Option::is_some()` pour `rules_missing` (via `builders.rs`) ; **et** `record_match_ranking_use_case` (Phase 5) — utilise les valeurs `win_points`/`draw_points`/`lose_points` pour calculer les points du match |
| `EnrolledTeamInfo` | `RankingCompetitionAdapter::find_enrolled_teams` | `classement_widget` handler (via `builders.rs`) — `has_enrolled_teams` + résolution des noms d'équipe pour `ClassementRowVm` |
| `RankingLineRow` | `IRankingRepository::find_latest_lines_for_season` (implémentation Postgres) | `builders.rs::build_classement_rows` |
| `ClassementRowVm` | `builders.rs::build_classement_rows` (combine `RankingLineRow` + `EnrolledTeamInfo`, trie par `ranking_points` décroissant, assigne `rank`) | Template `classement-widget.html` |
| `ClassementWidgetVm` | `classement_widget` handler (assemble les 3 sources via `builders.rs`) | Template `classement-widget.html` |

## Règles métier identifiées

- La ligne de classement stocke les compteurs cumulés (MJ/V/N/D + points), pas seulement les points — confirmé
- Le tri par rang se fait uniquement au moment de construire `ClassementRowVm` (côté `ranking`), jamais stocké sur la ligne elle-même — le rang d'une équipe peut changer à chaque nouvelle ligne d'une autre équipe, ce n'est pas une propriété figée de la ligne
