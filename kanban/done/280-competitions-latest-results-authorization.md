# BC `competitions` — autorisation + VMs des derniers résultats

**Priorité : haute**
**Dépend de :** `279-competitions-latest-results-repository.md`
**Contexte :** `competitions/io/web/latest_results_view.rs` (nouveau, miroir de `resultats_view.rs`)

## Objectif

Décider, pour chaque résultat, si le lien vers le rapport de match est
visible (même règle que l'onglet Résultats : admin d'espace, admin de la
compétition du match, ou coach de l'une des deux équipes), et construire les
VMs. Spec complète :
`docs/specs/accueil-derniers-resultats/widget-derniers-resultats/03-back.md`
et `04-dtos.md`.

**Important** : utiliser `state.competitions.space_member_port` pour la
vérification admin d'espace — **pas** `state.spaces.space_repository`
directement (violation de souveraineté déjà présente dans `resultats_view.rs`,
cf. carte 277, à ne pas reproduire ici).

---

## Conception

```rust
pub struct LatestResultsAuthorization {
    is_space_admin: bool,
    admin_competition_ids: HashSet<String>,
    my_team_ids: HashSet<String>,
}

impl LatestResultsAuthorization {
    pub fn allows(&self, competition_id: &str, home_team_id: &str, away_team_id: &str) -> bool {
        self.is_space_admin
            || self.admin_competition_ids.contains(competition_id)
            || self.my_team_ids.contains(home_team_id)
            || self.my_team_ids.contains(away_team_id)
    }
}

pub async fn compute_authorization(
    state: &AppState, user: &User, space_id: &SpaceId, rows: &[LatestResultDto],
) -> LatestResultsAuthorization {
    let is_space_admin = matches!(
        state.competitions.space_member_port.find_member_profile(&user.id, space_id).await,
        Some(SpaceProfile::SpaceAdmin)
    );
    if is_space_admin {
        return LatestResultsAuthorization { is_space_admin: true, admin_competition_ids: HashSet::new(), my_team_ids: HashSet::new() };
    }
    // admin_competition_ids : find_base_info par competition_id distinct des `rows`
    // my_team_ids : team_info_port.find_enrolled_teams par season_id distinct des `rows`, filtré sur coach_id == user.id
    LatestResultsAuthorization { is_space_admin: false, admin_competition_ids, my_team_ids }
}

pub fn to_latest_result_vm(row: LatestResultDto, authz: &LatestResultsAuthorization) -> LatestResultVm {
    let home_score = row.home_score.unwrap_or(0) as u32;
    let away_score = row.away_score.unwrap_or(0) as u32;
    let report_url = if authz.allows(&row.competition_id, &row.home_team_id, &row.away_team_id) {
        row.match_report_url
    } else {
        None
    };
    LatestResultVm {
        competition_name: row.competition_name,
        round_name: row.round_name,
        home_name: row.home_team_name,
        home_score,
        home_is_winner: home_score > away_score,
        away_name: row.away_team_name,
        away_score,
        away_is_winner: away_score > home_score,
        date: format_date(row.published_at),
        report_url,
    }
}
```

`format_date` : petit helper privé local (mois en français), pas de
réutilisation du helper équivalent de `news_feed.rs` (BC différent).

## Checklist

- [ ] `LatestResultsAuthorization` + `allows` (déduplication `competition_id`/`season_id` sur les `rows` avant les appels IO)
- [ ] `compute_authorization` utilise `space_member_port`, **pas** `state.spaces`
- [ ] `to_latest_result_vm` : égalité → `home_is_winner`/`away_is_winner` tous deux `false`
- [ ] `format_date` local (format "24 août 2024")
- [ ] Tests unitaires : `allows` (admin espace / admin compétition / coach home / coach away / aucun des trois) ; `to_latest_result_vm` (victoire home, victoire away, égalité)
