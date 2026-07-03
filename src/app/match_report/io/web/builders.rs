use crate::app::match_report::domain::value_objects::MatchAction;
use crate::app::match_report::io::web::view_models::action_player_key;
use crate::app::match_report::ports::{
    ICoachDataPort, ICompetitionDataPort, ISppCalculatorPort, TeamInfoDto,
};

// ── Récap — VMs dépendant d'un port (builders.rs) ──────────────────────────────

pub struct TeamBannerVm {
    pub team_name: String,
    pub team_initials: String,
    pub coach_name: String,
    pub logo_url: Option<String>,
    pub result_badge: &'static str,
}

pub fn build_team_banner(info: TeamInfoDto, own_score: u8, other_score: u8) -> TeamBannerVm {
    let result_badge = match own_score.cmp(&other_score) {
        std::cmp::Ordering::Greater => "Victoire",
        std::cmp::Ordering::Less => "Défaite",
        std::cmp::Ordering::Equal => "Égalité",
    };
    TeamBannerVm {
        team_initials: initials_from(&info.team_name),
        team_name: info.team_name,
        coach_name: info.coach_name,
        logo_url: info.logo_url,
        result_badge,
    }
}

fn initials_from(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

pub struct RoundContextVm {
    pub competition_name: String,
    pub season_name: String,
    pub round_name: String,
}

pub async fn build_round_context_vm(
    comp_data: &dyn ICompetitionDataPort,
    season_id: &str,
    round_id: &str,
) -> Option<RoundContextVm> {
    let dto = comp_data.find_round_context(season_id, round_id).await?;
    Some(RoundContextVm {
        competition_name: dto.competition_name,
        season_name: dto.season_name,
        round_name: dto.round_name,
    })
}

pub struct PerformanceRowVm {
    pub side: &'static str,
    pub player_display_name: String,
    pub player_position: String,
    pub spp_earned: u8,
}

pub async fn build_performance_rows(
    spp_port: &dyn ISppCalculatorPort,
    home_actions: &[MatchAction],
    away_actions: &[MatchAction],
    home_roster_id: &str,
    away_roster_id: &str,
) -> Vec<PerformanceRowVm> {
    let result = spp_port
        .calculate_match_spp(home_actions, away_actions, home_roster_id, away_roster_id)
        .await;

    let home = result.home.into_iter().filter_map(|dto| {
        find_action_display(home_actions, &dto.action_player)
            .map(|(name, pos)| PerformanceRowVm { side: "home", player_display_name: name, player_position: pos, spp_earned: dto.spp })
    });
    let away = result.away.into_iter().filter_map(|dto| {
        find_action_display(away_actions, &dto.action_player)
            .map(|(name, pos)| PerformanceRowVm { side: "away", player_display_name: name, player_position: pos, spp_earned: dto.spp })
    });
    let mut rows: Vec<PerformanceRowVm> = home.chain(away).collect();
    rows.sort_by(|a, b| b.spp_earned.cmp(&a.spp_earned));
    rows
}

fn find_action_display(
    actions: &[MatchAction],
    player: &crate::app::match_report::domain::value_objects::ActionPlayer,
) -> Option<(String, String)> {
    let key = action_player_key(player);
    actions
        .iter()
        .find(|a| action_player_key(&a.player) == key)
        .map(|a| (a.player_display_name.clone(), a.player_position.clone()))
}

pub async fn build_submitted_by(coach_data: &dyn ICoachDataPort, created_by: &str) -> Option<String> {
    coach_data.find_coach_name(created_by).await
}
