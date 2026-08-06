use crate::app::routes::AppRoutes;
use crate::app::teams::domain::team::{GamePhase, ParticipationStatus, Team};
use crate::app::teams::ports::{IRosterCatalogPort, ITeamRepository, RepositoryError};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use std::time::Duration;

// ── View models ───────────────────────────────────────────────────────────────

pub struct StaffLineVm {
    pub label: String,
    pub quantity: u8,
    pub unit_price: u32,
    pub total_price: u32,
}

pub struct StaffVm {
    pub lines: Vec<StaffLineVm>,
    pub grand_total: u32,
}

impl StaffVm {
    fn from(team: &Team, reroll_price_kpo: u32) -> Self {
        let lines = vec![
            StaffLineVm {
                label: "Relances".into(),
                quantity: team.rerolls.0,
                unit_price: reroll_price_kpo,
                total_price: team.rerolls.0 as u32 * reroll_price_kpo,
            },
            StaffLineVm {
                label: "Apothicaire".into(),
                quantity: team.apothecaries.0,
                unit_price: 50,
                total_price: team.apothecaries.0 as u32 * 50,
            },
            StaffLineVm {
                label: "Assistants entraîneurs".into(),
                quantity: team.assistants.0,
                unit_price: 10,
                total_price: team.assistants.0 as u32 * 10,
            },
            StaffLineVm {
                label: "Pom-pom girls".into(),
                quantity: team.cheerleaders.0,
                unit_price: 10,
                total_price: team.cheerleaders.0 as u32 * 10,
            },
        ];

        let grand_total = lines.iter().map(|l| l.total_price).sum();
        Self { lines, grand_total }
    }
}

pub enum BannerCtaVm {
    Print,
    Navigate { label: String, href: String },
    Mutate { label: String, post_url: String },
}

pub struct BannerVm {
    pub css_variant: String,
    pub icon: String,
    pub title: String,
    pub detail: String,
    pub ctas: Vec<BannerCtaVm>,
}

impl BannerVm {
    fn from_domain(team: &Team, space_id: &str, app_routes: &AppRoutes) -> Option<Self> {
        use GamePhase::*;
        use ParticipationStatus::*;

        let team_id = team.id.to_string();
        match (&team.participation_status, &team.game_phase) {
            (PendingEnrollment, _) => Some(Self {
                css_variant: "pending".into(),
                icon: "📋".into(),
                title: "Équipe en attente d'inscription.".into(),
                detail: "L'inscription est en cours de validation par un commissaire de ligue."
                    .into(),
                ctas: vec![],
            }),
            (Enrolled, Some(ReadyToPlay)) => Some(Self {
                css_variant: "ready".into(),
                icon: "✅".into(),
                title: "Équipe prête à jouer.".into(),
                detail: "Aucune action requise avant le prochain match.".into(),
                ctas: vec![BannerCtaVm::Print],
            }),
            (Enrolled, Some(MatchReporting)) => {
                let href = team
                    .current_match_report_id
                    .as_ref()
                    .map(|id| {
                        app_routes
                            .match_report
                            .edit_match_report(space_id, &id.to_string())
                    })
                    .unwrap_or_default();
                Some(Self {
                    css_variant: "phase".into(),
                    icon: "📝".into(),
                    title: "Rapport de match en cours.".into(),
                    detail: "La saisie du dernier match n'est pas terminée.".into(),
                    ctas: vec![BannerCtaVm::Navigate {
                        label: "Reprendre le rapport →".into(),
                        href,
                    }],
                })
            }
            (Enrolled, Some(PlayerImprovement)) => Some(Self {
                css_variant: "phase".into(),
                icon: "⚡".into(),
                title: "Phase d'amélioration des joueurs.".into(),
                detail: "Des joueurs ont des SPP à dépenser suite au dernier match.".into(),
                ctas: vec![BannerCtaVm::Mutate {
                    label: "Évolutions terminées".into(),
                    post_url: app_routes
                        .teams
                        .validate_improvement_phase(space_id, &team_id),
                }],
            }),
            // La validation de phase vit dans le panier, pas ici : depuis la
            // bannière, le coach clôturerait ses achats sans avoir vu ce qu'il
            // valide — et un refus en bloc tomberait sans qu'il comprenne
            // pourquoi.
            (Enrolled, Some(Recruitment)) => Some(Self {
                css_variant: "phase".into(),
                icon: "🛒".into(),
                title: "Phase de recrutement.".into(),
                detail: "Achetez des joueurs ou du staff avant de terminer les achats.".into(),
                ctas: vec![BannerCtaVm::Navigate {
                    label: "Recruter →".into(),
                    href: app_routes.teams.recruitment_page(space_id, &team_id),
                }],
            }),
            // Même raison qu'au recrutement : la validation vit dans le panier,
            // pas ici. Depuis la fiche d'équipe, le coach clôturerait la phase
            // sans avoir vu qui il renvoie — et un refus en bloc tomberait sans
            // qu'il comprenne pourquoi.
            (Enrolled, Some(Dismissals)) => Some(Self {
                css_variant: "phase".into(),
                icon: "🚪".into(),
                title: "Phase de renvois.".into(),
                detail: "Renvoyez les joueurs dont vous ne voulez plus avant de valider.".into(),
                ctas: vec![BannerCtaVm::Navigate {
                    label: "Gérer les renvois →".into(),
                    href: app_routes.teams.dismissals_page(space_id, &team_id),
                }],
            }),
            _ => None,
        }
    }
}

pub struct TeamDetailVm {
    pub id: String,
    pub name: String,
    pub initials: String,
    pub logo_url: Option<String>,
    pub roster_name: String,
    pub roster_initials: String,
    pub roster_logo_url: Option<String>,
    pub coach_name: String,
    pub dedicated_fans: u8,
    pub treasury_kpo: u32,
    pub team_value_kpo: u32,
    pub competition_name: Option<String>,
    pub season_name: Option<String>,
    pub status_label: String,
    pub status_css_class: String,
    pub players_widget_url: String,
    pub staff: StaffVm,
    pub banner: Option<BannerVm>,
}

impl TeamDetailVm {
    fn from(team: &Team, space_id: &str, roster_catalog_port: &dyn IRosterCatalogPort) -> Self {
        let (status_label, status_css_class) = status_display(team);
        let roster_initials = team
            .roster_name
            .as_ref()
            .split_whitespace()
            .filter_map(|w: &str| w.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase();

        let roster_info = roster_catalog_port.find_catalog(&team.roster_id.to_string());

        let roster_logo_url = roster_info
            .as_ref()
            .and_then(|t| t.logo.as_deref())
            .map(|url| {
                crate::app::shared_kernel::identity::cloudinary::transform(
                    url,
                    "c_fill,w_120,h_120,q_auto,f_auto",
                )
            });

        let reroll_price_kpo = roster_info.map(|t| t.reroll_base_cost).unwrap_or(50);

        let logo_url = team.logo_url.as_deref().map(|url| {
            crate::app::shared_kernel::identity::cloudinary::transform(
                url,
                "c_fill,w_120,h_120,q_auto,f_auto",
            )
        });

        let app_routes = AppRoutes::default();
        let banner = BannerVm::from_domain(team, space_id, &app_routes);

        Self {
            id: team.id.to_string(),
            name: team.name.to_string(),
            initials: team.initials.clone(),
            logo_url,
            roster_name: team.roster_name.to_string(),
            roster_initials,
            roster_logo_url,
            coach_name: team.coach_name.clone(),
            dedicated_fans: team.dedicated_fans.into_inner(),
            treasury_kpo: team.treasury.0,
            team_value_kpo: team.team_value.0,
            competition_name: team.competition_name.clone(),
            season_name: team.season_name.clone(),
            status_label,
            status_css_class,
            players_widget_url: app_routes
                .players
                .players_by_team_widget(space_id, &team.id.to_string()),
            staff: StaffVm::from(team, reroll_price_kpo),
            banner,
        }
    }
}

fn status_display(team: &Team) -> (String, String) {
    match &team.participation_status {
        ParticipationStatus::Dismissed => ("Renvoyée".into(), "dismissed".into()),
        ParticipationStatus::Rejected => ("Inscription refusée".into(), "dismissed".into()),
        ParticipationStatus::PendingEnrollment => {
            ("En attente d'inscription".into(), "pending".into())
        }
        ParticipationStatus::Enrolled => match &team.game_phase {
            Some(GamePhase::ReadyToPlay) => ("Prête à jouer".into(), "ready".into()),
            Some(GamePhase::MatchReporting) => ("Rapport en cours".into(), "phase".into()),
            Some(GamePhase::PlayerImprovement) => ("Phase d'amélioration".into(), "phase".into()),
            Some(GamePhase::Recruitment) => ("Phase de recrutement".into(), "phase".into()),
            Some(GamePhase::Dismissals) => ("Phase de renvois".into(), "phase".into()),
            Some(GamePhase::TemporaryRetirement) => ("Retraite temporaire".into(), "phase".into()),
            Some(GamePhase::OffSeason) => ("Off-season".into(), "offseason".into()),
            None => ("Inscrite".into(), "ready".into()),
        },
    }
}

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "teams-team-detail.html")]
pub struct TeamDetailTemplate {
    pub app_routes: AppRoutes,
    pub vm: TeamDetailVm,
    pub back_url: String,
}

impl IntoResponse for TeamDetailTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

// ── Handler ───────────────────────────────────────────────────────────────────

/// Retente la lecture avant d'abandonner : la création d'une équipe redirige
/// ici avant que le listener cross-BC `teams` ait forcément commité la
/// projection issue de l'app event émis par `team_creation`. Seul le cas
/// « pas encore trouvée » est retenté — une vraie erreur repository remonte
/// immédiatement.
async fn find_team_with_retry(
    repo: &dyn ITeamRepository,
    team_id: &str,
) -> Result<Option<Team>, RepositoryError> {
    const MAX_ATTEMPTS: u32 = 3;
    const BACKOFF: Duration = Duration::from_millis(50);

    for attempt in 0..MAX_ATTEMPTS {
        if let Some(team) = repo.find_by_id(team_id).await? {
            return Ok(Some(team));
        }
        if attempt + 1 < MAX_ATTEMPTS {
            tokio::time::sleep(BACKOFF).await;
        }
    }
    Ok(None)
}

pub async fn team_detail(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team = match find_team_with_retry(state.teams.team_repository.as_ref(), &team_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            tracing::warn!("team_detail: team {team_id} introuvable après plusieurs tentatives");
            return StatusCode::NOT_FOUND.into_response();
        }
        Err(e) => {
            tracing::error!("team_detail find_by_id {team_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let back_url = AppRoutes::default().team_creation.my_teams(&space_id);
    let roster_catalog_port = state.teams.roster_catalog_port.as_ref();

    TeamDetailTemplate {
        app_routes: Default::default(),
        vm: TeamDetailVm::from(&team, &space_id, roster_catalog_port),
        back_url,
    }
    .into_response()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RosterId, SeasonId};
    use crate::app::shared_kernel::bloodbowl::staff_counts::{
        ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
    use crate::app::teams::domain::team::TeamDomainEvent;
    use crate::app::teams::domain::value_objects::{DedicatedFans, Kpo, RosterName, TeamName};
    use crate::app::teams::ports::{TeamCardRow, TeamEnrollmentRow};
    use async_trait::async_trait;
    use std::sync::Mutex;

    fn created_event() -> TeamDomainEvent {
        TeamDomainEvent::TeamCreated {
            team_id: TeamId::try_new("00000000000000000000000001").unwrap(),
            space_id: SpaceId::try_new("00000000000000000000000002").unwrap(),
            competition_id: CompetitionId::try_new("00000000000000000000000003").unwrap(),
            competition_name: "Ligue de Condate".to_string(),
            season_id: SeasonId::try_new("00000000000000000000000004").unwrap(),
            season_name: "Saison 2025".to_string(),
            name: TeamName::try_new("Les Korrigans FC".to_string()).unwrap(),
            logo_url: None,
            roster_id: RosterId::try_new("00000000000000000000000005").unwrap(),
            roster_name: RosterName::try_new("Elfes Sylvestres".to_string()).unwrap(),
            coach_id: CoachId::try_new("00000000000000000000000006").unwrap(),
            coach_name: "Colonel Castor".to_string(),
            treasury: Kpo(1000),
            dedicated_fans: DedicatedFans::try_new(2).unwrap(),
            rerolls: RerollCount(3),
            apothecaries: ApothecaryCount(1),
            assistants: AssistantCount(2),
            cheerleaders: CheerleaderCount(3),
        }
    }

    /// Double qui ne trouve la team qu'à partir de la Nème lecture — imite le
    /// délai d'écriture asynchrone de la projection cross-BC après création.
    struct CountingRepo {
        found_at_attempt: u32,
        calls: Mutex<u32>,
    }

    #[async_trait]
    impl ITeamRepository for CountingRepo {
        async fn append(
            &self,
            _team_id: &str,
            _event: &TeamDomainEvent,
            _expected_version: u64,
        ) -> Result<u64, RepositoryError> {
            unimplemented!("non exercé par find_team_with_retry")
        }

        async fn append_batch(
            &self,
            _team_id: &str,
            _events: &[TeamDomainEvent],
            _expected_version: u64,
        ) -> Result<u64, RepositoryError> {
            unimplemented!("non exercé par find_team_with_retry")
        }

        async fn find_by_id(&self, _team_id: &str) -> Result<Option<Team>, RepositoryError> {
            let mut calls = self.calls.lock().unwrap();
            *calls += 1;
            if *calls >= self.found_at_attempt {
                Ok(Team::hydrate(&[created_event()]))
            } else {
                Ok(None)
            }
        }

        async fn find_by_season_and_status(
            &self,
            _season_id: &str,
            _status: &str,
        ) -> Result<Vec<TeamEnrollmentRow>, RepositoryError> {
            unimplemented!("non exercé par find_team_with_retry")
        }

        async fn find_enrolled_for_season(
            &self,
            _season_id: &str,
        ) -> Result<Vec<TeamCardRow>, RepositoryError> {
            unimplemented!("non exercé par find_team_with_retry")
        }
    }

    #[tokio::test]
    async fn retries_until_found() {
        let repo = CountingRepo {
            found_at_attempt: 3,
            calls: Mutex::new(0),
        };

        let team = find_team_with_retry(&repo, "whatever").await.unwrap();

        assert!(team.is_some());
        assert_eq!(*repo.calls.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn gives_up_after_max_attempts() {
        let repo = CountingRepo {
            found_at_attempt: 99,
            calls: Mutex::new(0),
        };

        let team = find_team_with_retry(&repo, "whatever").await.unwrap();

        assert!(team.is_none());
        assert_eq!(*repo.calls.lock().unwrap(), 3);
    }
}
