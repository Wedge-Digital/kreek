# Ranking — Regroupement des stats de match par équipe (refacto pure)

**Priorité : haute**
**Dépend de :** —
**Contexte :** `src/app/ranking/use_cases/record_match_ranking_use_case.rs`, `src/app/ranking/io/app_events/match_report_published_listener.rs`
**Spec :** `docs/specs/ranking/tiebreakers/tiebreak-calc/04-dtos.md`

## Objectif

Regrouper les stats de match par équipe dans la commande, **sans ajouter aucun champ
nouveau**. Refacto pure : comportement identique, aucun test métier à modifier.

Carte séparée exprès : la carte 216 ajoutera deux compteurs à une structure déjà propre,
au lieu de mêler un renommage de 8 champs et un ajout de logique dans le même diff.

## Pourquoi

`RecordMatchRankingCommand` porte 12 champs, dont quatre par paire symétrique
(`home_score`/`away_score`, `home_casualties_inflicted`/…). La carte 216 la porterait à 16.

Au-delà du nombre : `record_home` / `record_away` piochent aujourd'hui champ par champ
dans huit champs à préfixe. C'est là que se glisse une erreur de symétrie — utiliser
`home_score` pour l'équipe away compile sans broncher.

## Conception (cf. `04-dtos.md`)

```rust
pub struct TeamMatchStats {
    pub score:      MatchScore,
    pub casualties: CasualtiesInflicted,
}

pub struct RecordMatchRankingCommand {
    pub competition_id:  CompetitionId,
    pub season_id:       SeasonId,
    pub round_id:        RoundId,
    pub match_report_id: MatchReportId,
    pub home_team_id:    TeamId,
    pub away_team_id:    TeamId,
    pub home:            TeamMatchStats,
    pub away:            TeamMatchStats,
    pub published_at:    DateTime<Utc>,
}
```

`fouls` et `completions` **ne sont pas** ajoutés ici — c'est la carte 216.

`record_home` / `record_away` construisent leur `MatchStats` en croisant les deux structs.
**Attention** : `own_td` / `opponent_td` se croisent entre équipes, `casualties` **ne se
croise pas** (stat propre à l'équipe).

### Sites à adapter

| Site | Adaptation |
|---|---|
| `match_report_published_listener` (`:60-66`) | Construit `home: TeamMatchStats { … }` / `away: …` |
| `record_home` / `record_away` | Lisent `cmd.home` / `cmd.away` |
| Tests du use case | Littéraux de commande regroupés |
| `test_match_report_published_pipeline.rs` | Idem si un littéral y figure |

## Checklist

- [ ] `TeamMatchStats` défini ; commande à 9 champs
- [ ] Listener et use case adaptés
- [ ] **Aucun champ nouveau**, aucune assertion métier modifiée
- [ ] Test de symétrie ajouté ou renforcé : sur un match 2-1, les TD se croisent, les
      sorties non
- [ ] `make test` + `make check-arch` passent
