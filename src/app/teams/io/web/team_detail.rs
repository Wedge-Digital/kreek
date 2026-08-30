use crate::app::auth::auth_backend::AuthSession;
use crate::app::routes::AppRoutes;
use crate::app::teams::domain::team::{GamePhase, ParticipationStatus, Team};
use crate::app::teams::ports::{
    IRosterCatalogPort, ITeamRepository, RepositoryError, TreasuryMovementRow,
};
use crate::app::teams::use_cases::roster_edit_access_service;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
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
    Navigate {
        label: String,
        href: String,
    },
    Mutate {
        label: String,
        post_url: String,
    },
    /// Déclencheur de l'édition d'effectif. Aucune URL : le bandeau ne connaît
    /// pas le widget joueurs, qui appartient à un autre BC. Il publie trois
    /// événements DOM sur `body`, le widget s'y abonne — c'est la règle 2 des
    /// widgets, et c'est ce qui permet aux deux BCs de s'ignorer.
    RosterEdit,
}

pub struct BannerVm {
    pub css_variant: String,
    pub icon: String,
    pub title: String,
    pub detail: String,
    pub ctas: Vec<BannerCtaVm>,
}

impl BannerVm {
    /// `peut_editer` ne conditionne **que** le déclencheur d'édition. Le
    /// bandeau, son texte et le bouton d'impression restent identiques pour
    /// tout visiteur : on retire un raccourci qu'il n'a pas le droit
    /// d'emprunter, on ne lui cache pas la page.
    fn from_domain(
        team: &Team,
        space_id: &str,
        app_routes: &AppRoutes,
        peut_editer: bool,
    ) -> Option<Self> {
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
                ctas: match peut_editer {
                    true => vec![BannerCtaVm::RosterEdit, BannerCtaVm::Print],
                    false => vec![BannerCtaVm::Print],
                },
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
            (Enrolled, Some(CostlyMistakes)) => Some(Self {
                css_variant: "phase".into(),
                icon: "💸".into(),
                title: "Erreurs coûteuses.".into(),
                detail: "Votre trésorerie attire les ennuis : un jet décide de ce qu'il en reste."
                    .into(),
                ctas: vec![BannerCtaVm::Navigate {
                    label: "Lancer le dé →".into(),
                    href: app_routes.teams.costly_mistakes_page(space_id, &team_id),
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
    fn from(
        team: &Team,
        space_id: &str,
        roster_catalog_port: &dyn IRosterCatalogPort,
        peut_editer: bool,
    ) -> Self {
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
        let banner = BannerVm::from_domain(team, space_id, &app_routes, peut_editer);

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
            Some(GamePhase::CostlyMistakes) => ("Erreurs coûteuses".into(), "phase".into()),
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
    /// L'espace, pour construire les URLs d'onglet. Le VM ne le porte pas : il
    /// décrit l'équipe, pas le chemin par lequel on la regarde.
    pub space_id: String,
    /// « squad » ou « treasury ». Le gabarit s'en sert pour la classe `active`,
    /// et **jamais pour décider quoi rendre** — c'est le handler qui aiguille.
    pub active_tab: String,
    /// Le contenu de l'onglet, **déjà rendu**. Même forme qu'`AdminPageTemplate` :
    /// le gabarit de page ne connaît pas les onglets, il pose leur conteneur.
    pub content: String,
}

/// Le fragment d'un onglet, sans le layout : ce que la route rend quand
/// `HX-Request` est présent.
///
/// **Il emprunte le VM plutôt que de le posséder.** La page en a besoin ensuite
/// pour son en-tête et son bandeau ; le posséder aurait imposé de dériver
/// `Clone` sur cinq types pour un seul clonage.
#[derive(Template)]
#[template(path = "teams-squad-tab.html")]
pub struct TeamSquadTabTemplate<'a> {
    pub vm: &'a TeamDetailVm,
}

impl IntoResponse for TeamSquadTabTemplate<'_> {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("teams squad tab render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
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
    auth_session: AuthSession,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    rendre_fiche(&space_id, &team_id, auth_session, &state, headers, "squad").await
}

/// L'onglet « Trésorerie ». Il ne montre rien : la carte 436 le remplit.
///
/// **La route existe avant son contenu** parce que l'aiguillage doit être posé
/// et testé d'un bloc — mais l'onglet, lui, **reste inerte** dans le gabarit :
/// une route qui répond « rien » se lit comme une panne.
pub async fn team_page_treasury(
    Path((space_id, team_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    rendre_fiche(
        &space_id,
        &team_id,
        auth_session,
        &state,
        headers,
        "treasury",
    )
    .await
}

/// La fiche, pour un onglet donné.
///
/// # Une seule route par onglet, deux usages
///
/// Sous `HX-Request`, elle rend le **fragment** ; sans lui, la page entière avec
/// l'onglet actif. C'est le patron d'`admin-page.html`, et il évite une seconde
/// route qui doublerait la surface pour la même réponse.
///
/// Conséquence à ne pas perdre de vue : **le fragment est le chemin de
/// navigation**. Tout contrôle posé sur la page complète doit l'être ici aussi,
/// sans quoi changer d'onglet contournerait ce que le chargement direct refuse.
async fn rendre_fiche(
    space_id: &str,
    team_id: &str,
    auth_session: AuthSession,
    state: &AppState,
    headers: HeaderMap,
    active_tab: &str,
) -> Response {
    let team = match find_team_with_retry(state.teams.team_repository.as_ref(), team_id).await {
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

    // Sans session, aucun droit : le bandeau garde son texte et son bouton
    // d'impression, il perd seulement le déclencheur d'édition.
    let peut_editer = match auth_session.user.as_ref() {
        Some(user) => {
            roster_edit_access_service::peut_modifier_effectif(
                &team,
                &user.id,
                &user.coach_name.clone().into_inner(),
                state.teams.access_port.as_ref(),
            )
            .await
        }
        None => false,
    };

    let back_url = AppRoutes::default().team_creation.my_teams(space_id);
    let roster_catalog_port = state.teams.roster_catalog_port.as_ref();

    let vm = TeamDetailVm::from(&team, space_id, roster_catalog_port, peut_editer);

    // **L'aiguillage, calculé une fois.** Le `_` rend « Joueurs & Staff »
    // plutôt que d'échouer : un `active_tab` inconnu vient d'une URL tapée à la
    // main, et répondre 404 sur une page qui existe serait pire que d'afficher
    // l'onglet par défaut.
    let content = match active_tab {
        // Vide jusqu'à la carte 436. L'onglet n'étant pas cliquable, on n'arrive
        // ici qu'en tapant l'URL — mieux vaut un onglet vide qu'un contenu qui
        // n'est pas le sien.
        "treasury" => String::new(),
        _ => match (TeamSquadTabTemplate { vm: &vm }).render() {
            Ok(html) => html,
            Err(e) => {
                tracing::error!("teams squad tab render: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        },
    };

    // Sous `HX-Request`, le contenu **seul** : c'est lui que le `hx-swap`
    // injecte dans `#team-tab-content`.
    if headers.contains_key("hx-request") {
        return Html(content).into_response();
    }

    TeamDetailTemplate {
        app_routes: Default::default(),
        vm,
        back_url,
        space_id: space_id.to_string(),
        active_tab: active_tab.to_string(),
        content,
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
    use crate::app::teams::ports::{MyTeamRow, TeamCardRow, TeamEnrollmentRow};
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// Le bandeau perd **le seul** déclencheur d'édition, et rien d'autre.
    ///
    /// C'est ce qui distingue « masquer un raccourci » de « cacher la page » :
    /// un visiteur tiers garde le bandeau, son texte et son bouton
    /// d'impression. Sans cette moitié-là, un correctif qui viderait les CTA
    /// passerait le test du bouton absent.
    #[test]
    fn le_bandeau_perd_le_seul_bouton_d_edition_pour_un_tiers() {
        // `TeamEnrolled` pose les deux états d'un coup — inscrite et prête à
        // jouer —, seul couple dans lequel le bouton d'édition existe.
        let team = Team::hydrate(&[
            created_event(),
            TeamDomainEvent::TeamEnrolled {
                competition_id: CompetitionId::try_new("00000000000000000000000003").unwrap(),
                competition_name: "Ligue de Condate".to_string(),
                season_id: SeasonId::try_new("00000000000000000000000004").unwrap(),
                season_name: "Saison 2025".to_string(),
            },
        ])
        .unwrap();
        let routes = AppRoutes::default();

        let avec = BannerVm::from_domain(&team, "space", &routes, true).unwrap();
        let sans = BannerVm::from_domain(&team, "space", &routes, false).unwrap();

        assert!(avec
            .ctas
            .iter()
            .any(|c| matches!(c, BannerCtaVm::RosterEdit)));
        assert!(!sans
            .ctas
            .iter()
            .any(|c| matches!(c, BannerCtaVm::RosterEdit)));

        for banniere in [&avec, &sans] {
            assert!(
                banniere
                    .ctas
                    .iter()
                    .any(|c| matches!(c, BannerCtaVm::Print)),
                "le bouton d'impression reste pour tout le monde"
            );
            assert_eq!(banniere.title, avec.title, "le texte ne change pas");
        }
    }

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
        /// Doublure : le contrôle d'appartenance est exercé par les tests de
        /// handler, sur une vraie base.
        async fn find_space_id(&self, _: &str) -> Result<Option<String>, RepositoryError> {
            Ok(None)
        }

        async fn list_treasury_movements(
            &self,
            _: &str,
        ) -> Result<Vec<TreasuryMovementRow>, RepositoryError> {
            Ok(Vec::new())
        }

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

        async fn find_by_coach_and_space(
            &self,
            _coach_id: &str,
            _space_id: &str,
        ) -> Result<Vec<MyTeamRow>, RepositoryError> {
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

    // ── Les onglets de la fiche (carte 434) ──────────────────────────────────

    fn vm_minimal() -> TeamDetailVm {
        TeamDetailVm {
            id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".into(),
            name: "Les Granitiers".into(),
            initials: "LG".into(),
            logo_url: None,
            roster_name: "Nains".into(),
            roster_initials: "NA".into(),
            roster_logo_url: None,
            coach_name: "Bagouze".into(),
            dedicated_fans: 5,
            treasury_kpo: 100,
            team_value_kpo: 1000,
            competition_name: None,
            season_name: None,
            status_label: "Inscrite".into(),
            status_css_class: "ready".into(),
            players_widget_url: "/widget".into(),
            staff: StaffVm {
                lines: vec![],
                grand_total: 0,
            },
            banner: None,
        }
    }

    /// Rend la page **comme le handler la rend** : le contenu de l'onglet est
    /// calculé d'abord, puis passé. Le construire autrement ici ferait tester
    /// une page que personne ne sert.
    fn page(active_tab: &str) -> String {
        let vm = vm_minimal();
        let content = match active_tab {
            "treasury" => String::new(),
            _ => TeamSquadTabTemplate { vm: &vm }.render().unwrap(),
        };
        TeamDetailTemplate {
            app_routes: Default::default(),
            vm,
            back_url: "/retour".into(),
            space_id: "01ARZ3NDEKTSV4RRFFQ69G5FAW".into(),
            active_tab: active_tab.into(),
            content,
        }
        .render()
        .expect("la page doit se rendre")
    }

    /// **Le fragment porte le contenu, jamais son conteneur.**
    ///
    /// Le `CLAUDE.md` proscrit le fragment qui répète l'`id` de sa cible : après
    /// injection, le DOM porterait deux `#team-tab-content`, et les composants
    /// Alpine du contenu s'initialiseraient deux fois.
    #[test]
    fn le_fragment_d_onglet_ne_repete_pas_l_id_de_sa_cible() {
        let vm = vm_minimal();
        let fragment = TeamSquadTabTemplate { vm: &vm }
            .render()
            .expect("le fragment doit se rendre");

        assert!(!fragment.contains("team-tab-content"), "{fragment}");
        assert!(
            fragment.contains("players-widget"),
            "le widget joueurs doit y être"
        );
        assert!(
            fragment.contains("staff-panel"),
            "le panneau staff doit y être"
        );
    }

    /// La page contient exactement ce que le fragment contient — c'est ce qui
    /// rend le déplacement invisible à l'écran.
    #[test]
    fn la_page_porte_le_meme_contenu_que_le_fragment() {
        let vm = vm_minimal();
        let fragment = TeamSquadTabTemplate { vm: &vm }.render().unwrap();
        let page = page("squad");

        assert!(
            page.contains("team-tab-content"),
            "le conteneur est dans la page"
        );
        for marqueur in ["players-widget", "staff-panel", "Sous-total staff"] {
            assert!(page.contains(marqueur), "« {marqueur} » manque à la page");
            assert!(
                fragment.contains(marqueur),
                "« {marqueur} » manque au fragment"
            );
        }
    }

    /// L'onglet actif suit `active_tab`, et lui seul.
    #[test]
    fn l_onglet_actif_suit_le_parametre() {
        let squad = page("squad");
        let treasury = page("treasury");

        // **Le `<a>` et non la chaîne seule** : « Trésorerie » porte la même
        // classe quand il est actif, et une assertion sur la chaîne nue serait
        // satisfaite par le mauvais onglet — elle passerait dans les deux sens.
        assert!(
            squad.contains(r#"<a class="tab active""#),
            "squad : {squad}"
        );
        assert!(
            !treasury.contains(r#"<a class="tab active""#),
            "sur trésorerie, « Joueurs & Staff » ne doit plus être actif"
        );
        assert!(
            treasury.contains(r#"<div class="tab active">Trésorerie</div>"#),
            "{treasury}"
        );
    }

    /// **La page d'un onglet montre le contenu de cet onglet.**
    ///
    /// Trouvé à l'écran : le gabarit incluait le fragment « Joueurs & Staff » en
    /// dur, si bien que `/tresorerie` rendait l'effectif sous un onglet
    /// Trésorerie actif. Le rendu était cohérent, la page bien formée, et le
    /// contenu n'était pas le sien — rien n'aurait signalé l'erreur.
    #[test]
    fn la_page_d_un_onglet_ne_montre_pas_le_contenu_d_un_autre() {
        let treasury = page("treasury");

        assert!(
            !treasury.contains("players-widget"),
            "l'onglet Trésorerie ne doit pas rendre l'effectif : {treasury}"
        );
        assert!(!treasury.contains("staff-panel"));
        // Contre-épreuve : la page reste une page, avec son en-tête et ses
        // onglets. Sans elle, un gabarit vide passerait ce test.
        assert!(treasury.contains("team-tabs"));
        assert!(treasury.contains("Les Granitiers"));
    }

    /// **Un onglet ne devient cliquable que lorsque son contenu existe.**
    ///
    /// « Trésorerie » et « Matchs » restent des `<div>` inertes — les cartes 436
    /// et 477 les câbleront. Une route qui répond « rien » se lit comme une
    /// panne, et l'utilisateur clique deux fois avant de conclure à un défaut.
    #[test]
    fn seul_l_onglet_livre_est_cliquable() {
        let rendu = page("squad");

        let liens: Vec<&str> = rendu
            .match_indices("<a class=\"tab")
            .map(|(_, s)| s)
            .collect();
        assert_eq!(
            liens.len(),
            1,
            "un seul onglet doit porter un lien : {rendu}"
        );
        assert!(rendu.contains(">Matchs</div>"), "« Matchs » reste inerte");
        assert!(
            rendu.contains(">Trésorerie</div>"),
            "« Trésorerie » reste inerte"
        );
    }
}
