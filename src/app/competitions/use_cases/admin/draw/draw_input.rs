//! Ce qui compose l'entrée du tirage, pour **tous** les chemins qui l'appellent.
//!
//! « Générer une journée complète » (`generate_pairings`) et l'appariement des
//! présents (aperçu et écriture) construisent le même `DrawInput` à partir des
//! mêmes données. Les recopier ferait des tirages qui divergeraient — ce que la
//! branche `sondage-presence` a découvert en trouvant R8 alimentée d'un seul
//! côté.
//!
//! Un module à part et non des fonctions publiques dans l'un des use cases :
//! aucun n'a à importer l'autre.
//!
//! **La matière de R4 est inter-BC, la règle est métier.** La relation équipe →
//! coach arrive par `ITeamInfoPort` ; le domaine reçoit des paires interdites, il
//! n'a pas à savoir qu'un coach existe.

use crate::app::competitions::domain::draw::{pair, DrawInput, MatchesPlayed, PlayedMeetings};
use crate::app::competitions::domain::group_repository_port::{GroupWithTeams, IGroupRepository};
use crate::app::competitions::domain::match_day::{DrawRules, MatchDay, Pairing};
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, MatchDayRepositoryError, NewPairingProjection,
};
use crate::app::competitions::ports::{ITeamInfoPort, TeamInfoDto};
use crate::app::competitions::use_cases::admin::pairing_created::{
    emit_pairing_created, PairingLocation,
};
use crate::app::competitions::use_cases::admin::team_enrollment::{
    build_new_pairing_projection, load_enrolled_teams,
};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::common::services::event_bus::event_bus::EventBus;
use std::collections::{HashMap, HashSet};

pub fn build_forbidden_pairs(
    teams: &[TeamId],
    team_display: &HashMap<String, TeamInfoDto>,
) -> HashSet<(TeamId, TeamId)> {
    let mut forbidden = HashSet::new();
    for (rank, a) in teams.iter().enumerate() {
        for b in teams.iter().skip(rank + 1) {
            if same_coach(a, b, team_display) {
                forbidden.insert(pair(a, b));
            }
        }
    }
    forbidden
}

pub fn same_coach(a: &TeamId, b: &TeamId, team_display: &HashMap<String, TeamInfoDto>) -> bool {
    match (
        team_display.get(&a.to_string()),
        team_display.get(&b.to_string()),
    ) {
        (Some(x), Some(y)) => x.coach_id == y.coach_id,
        _ => false,
    }
}

/// L'historique de la saison, **compté** par paire, la journée en cours exclue.
///
/// Un `HashSet` ne disait que « déjà jouée » : c'est ce choix de type qui
/// rendait la minimisation de R5 impossible.
pub fn build_history(days: &[MatchDay], exclude_id: &str) -> PlayedMeetings {
    let mut history = PlayedMeetings::new();
    for day in days.iter().filter(|d| d.id.to_string() != exclude_id) {
        for p in &day.pairings {
            history.record(
                &p.home_team_id,
                &p.away_team_id,
                day.position,
                day.name.clone(),
            );
        }
    }
    history
}

/// R8 — le nombre d'appariements **programmés** de chaque équipe sur la saison.
///
/// Même boucle que `build_historique`, même exclusion de la journée tirée : les
/// appariements qu'on s'apprête à écrire ne comptent pas contre les équipes
/// qu'ils concernent. Une fonction séparée plutôt qu'un tuple — chacune reste
/// courte et porte son nom.
///
/// Les appariements et non les rapports de match : ils vivent dans les tables du
/// BC, donc le compte se lit sans port.
pub fn build_matches_played(days: &[MatchDay], exclude_id: &str) -> HashMap<TeamId, MatchesPlayed> {
    let mut counts: HashMap<TeamId, MatchesPlayed> = HashMap::new();
    for day in days.iter().filter(|d| d.id.to_string() != exclude_id) {
        for p in &day.pairings {
            for side in [p.home_team_id, p.away_team_id] {
                counts.entry(side).or_default().0 += 1;
            }
        }
    }
    counts
}

// ── Le chargement commun ─────────────────────────────────────────────────────

/// Les dépôts et ports que tout chemin du tirage consulte.
pub struct DrawDeps<'a> {
    pub match_day_repo: &'a dyn IMatchDayRepository,
    pub group_repo: &'a dyn IGroupRepository,
    pub team_port: &'a dyn ITeamInfoPort,
}

/// Tout ce qu'un tirage lit, chargé par un seul chemin.
pub struct DrawContext {
    pub match_day: MatchDay,
    /// Les inscrites, par identifiant.
    pub teams: HashMap<String, TeamInfoDto>,
    pub groups: Vec<GroupWithTeams>,
    /// Toute la saison, journée tirée comprise : historique et matchs joués.
    pub season_days: Vec<MatchDay>,
}

#[derive(Debug)]
pub enum DrawLoadError {
    RoundNotFound,
    Repository(String),
}

impl From<MatchDayRepositoryError> for DrawLoadError {
    fn from(e: MatchDayRepositoryError) -> Self {
        Self::Repository(e.to_string())
    }
}

// arch:no-instrument — chargement commun : assemble les lectures d'un tirage, sans intention métier
pub async fn load_draw_context(
    round_id: &str,
    season_id: &str,
    deps: &DrawDeps<'_>,
) -> Result<DrawContext, DrawLoadError> {
    let match_day = deps
        .match_day_repo
        .find_by_id(round_id)
        .await?
        .ok_or(DrawLoadError::RoundNotFound)?;
    let teams = load_enrolled_teams(season_id, deps.team_port)
        .await
        .map_err(DrawLoadError::Repository)?;
    let groups = load_groups(season_id, deps.group_repo, &teams).await?;
    let season_days = deps.match_day_repo.find_by_season(season_id).await?;
    Ok(DrawContext {
        match_day,
        teams,
        groups,
        season_days,
    })
}

/// Les poules de la saison, ou une poule « Toutes les équipes » quand la saison
/// n'en a pas. **Vide** quand il n'y a ni poule ni inscrite : c'est à l'appelant
/// de dire si c'est un refus, après avoir jugé la journée elle-même.
async fn load_groups(
    season_id: &str,
    group_repo: &dyn IGroupRepository,
    team_display: &HashMap<String, TeamInfoDto>,
) -> Result<Vec<GroupWithTeams>, DrawLoadError> {
    let groups = group_repo
        .find_groups(season_id)
        .await
        .map_err(|e| DrawLoadError::Repository(e.to_string()))?;

    if !groups.is_empty() {
        return Ok(groups);
    }
    if team_display.is_empty() {
        return Ok(vec![]);
    }
    Ok(vec![GroupWithTeams {
        group_id: "default".to_string(),
        group_name: "Toutes les équipes".to_string(),
        position: 0,
        team_ids: team_display.keys().cloned().collect(),
    }])
}

// ── L'écriture commune ───────────────────────────────────────────────────────

/// Un seul `save_pairings` pour la journée (R14), **puis** les annonces :
/// jamais d'événement pour une rencontre que la transaction aurait annulée.
// arch:no-instrument — écriture commune : ses appelants (generate_pairings, confirm_draw) sont les use cases instrumentés
pub async fn write_round(
    ctx: &DrawContext,
    pairings: &[Pairing],
    location: PairingLocation<'_>,
    match_day_repo: &dyn IMatchDayRepository,
    event_bus: &EventBus,
) -> Result<(), MatchDayRepositoryError> {
    if pairings.is_empty() {
        return Ok(());
    }
    let rows: Vec<(Pairing, NewPairingProjection)> = pairings
        .iter()
        .map(|p| (p.clone(), projection_of(p, ctx)))
        .collect();
    match_day_repo
        .save_pairings(location.round_id, &rows)
        .await?;
    for (pairing, projection) in &rows {
        emit_pairing_created(event_bus, location, pairing, projection);
    }
    Ok(())
}

pub fn projection_of(pairing: &Pairing, ctx: &DrawContext) -> NewPairingProjection {
    let season_id = ctx.match_day.season_id.to_string();
    let home = pairing.home_team_id.to_string();
    let away = pairing.away_team_id.to_string();
    build_new_pairing_projection(&home, &away, &season_id, &ctx.match_day, &ctx.teams)
}

/// Le `DrawInput` d'une poule : ses équipes, l'historique de la saison, les
/// paires interdites et les matchs joués, la journée tirée exclue.
pub fn pool_input(teams: Vec<TeamId>, ctx: &DrawContext) -> DrawInput {
    let round_id = ctx.match_day.id.to_string();
    DrawInput {
        forbidden: build_forbidden_pairs(&teams, &ctx.teams),
        history: build_history(&ctx.season_days, &round_id),
        matches_played: build_matches_played(&ctx.season_days, &round_id),
        teams,
    }
}

/// Ce que la journée vérifie d'un tirage (R3, R4, R11) : les inscrites, leurs
/// paires interdites et l'appartenance aux poules de la saison.
pub fn draw_rules(ctx: &DrawContext) -> DrawRules {
    let enrolled = parse_team_ids(ctx.teams.keys());
    DrawRules {
        forbidden: build_forbidden_pairs(&enrolled, &ctx.teams),
        enrolled: enrolled.into_iter().collect(),
        pools: ctx
            .groups
            .iter()
            .map(|group| parse_team_ids(group.team_ids.iter()).into_iter().collect())
            .collect(),
    }
}

/// Les identifiants lus en base sont valides ; un identifiant illisible ne
/// peut être ni tiré ni accepté, il est donc écarté.
pub fn parse_team_ids<'a>(ids: impl Iterator<Item = &'a String>) -> Vec<TeamId> {
    ids.filter_map(|id| TeamId::try_new(id).ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{
        MatchDayName, MatchDayPosition, MatchDayType, Pairing,
    };
    use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId, SeasonId};

    fn match_day_with_pairings(pairings: Vec<Pairing>) -> MatchDay {
        MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("Journée 1".to_string()).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(0).unwrap(),
            pairings,
        }
    }

    // ── R8 — le compte qui alimente le critère d'exemption (carte 541) ───────

    fn pairing(home: &TeamId, away: &TeamId) -> Pairing {
        Pairing {
            id: PairingId::new(),
            home_team_id: *home,
            away_team_id: *away,
        }
    }

    #[test]
    fn build_matches_played_counts_both_sides() {
        let (a, b, c) = (TeamId::new(), TeamId::new(), TeamId::new());
        let days = vec![
            match_day_with_pairings(vec![pairing(&a, &b)]),
            match_day_with_pairings(vec![pairing(&a, &c)]),
        ];

        let counts = build_matches_played(&days, "aucune");

        assert_eq!(counts.get(&a), Some(&MatchesPlayed(2)));
        assert_eq!(counts.get(&b), Some(&MatchesPlayed(1)));
        assert_eq!(counts.get(&c), Some(&MatchesPlayed(1)));
    }

    /// Les appariements qu'on s'apprête à écrire ne comptent pas contre les
    /// équipes qu'ils concernent — même exclusion que `build_historique`.
    #[test]
    fn build_matches_played_ignores_the_drawn_round() {
        let (a, b) = (TeamId::new(), TeamId::new());
        let days = vec![
            match_day_with_pairings(vec![pairing(&a, &b)]),
            match_day_with_pairings(vec![pairing(&a, &b)]),
        ];
        let drawn = days[1].id.to_string();

        let counts = build_matches_played(&days, &drawn);

        assert_eq!(counts.get(&a), Some(&MatchesPlayed(1)));
    }

    /// Le défaut que la 541 corrige : le champ arrivait vide au tirage, donc le
    /// critère ne départageait rien. Ce test échouerait sur le code d'avant.
    #[test]
    fn a_blank_season_gives_no_count() {
        let counts = build_matches_played(&[], "aucune");

        assert!(
            counts.is_empty(),
            "aucune journée, aucun match — et toutes les équipes à égalité"
        );
    }
}
