//! La garde des routes de saisie d'un compte rendu (carte 622).
//!
//! **Qui** : les admins d'espace ou de compétition, et les coachs des deux
//! équipes du match — la règle d'`is_authorized`, celle du récapitulatif. Les
//! autres membres de l'espace reçoivent 403.
//!
//! **Où** : posée une fois, par `route_layer`, sur le groupe des routes de
//! saisie (`router.rs`) — l'écriture et les écrans qui la préparent. Une route
//! ajoutée à ce groupe est gardée sans qu'on y pense ; c'est le point du
//! découpage, préféré à vingt appels recopiés dans autant de handlers. Même
//! patron que `garde_action_equipe` dans `teams`.
//!
//! **Hors du groupe** : le hors-calendrier (`/new`), les feuilles imprimables
//! et le récapitulatif, qui se garde lui-même (décision du 2026-11-01).
//!
//! # La limite
//!
//! Elle lit `match_report_id` dans le **chemin**, comme `space_scope_middleware`.
//! Une route qui désignerait son rapport autrement lui échapperait : toutes
//! celles du groupe le portent dans le chemin.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::domain::user::User;
use crate::app::match_report::use_cases::match_report_access_service::{
    is_authorized, AccesRapportDeps, PorteeRapport,
};
use crate::state::AppState;
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::RequestPartsExt;
use std::collections::HashMap;

pub async fn garde_saisie(
    State(state): State<AppState>,
    auth_session: AuthSession,
    request: Request,
    next: Next,
) -> Response {
    let (mut parts, corps) = request.into_parts();
    let params = match parts.extract::<Path<HashMap<String, String>>>().await {
        Ok(Path(params)) => params,
        Err(_) => HashMap::new(),
    };
    if let Err(refus) = autoriser(&state, auth_session.user.as_ref(), &params).await {
        return refus;
    }
    next.run(Request::from_parts(parts, corps)).await
}

async fn autoriser(
    state: &AppState,
    user: Option<&User>,
    params: &HashMap<String, String>,
) -> Result<(), Response> {
    let (Some(space_id), Some(match_report_id)) =
        (params.get("space_id"), params.get("match_report_id"))
    else {
        // Une route du groupe sans ces deux segments : mal placée. La laisser
        // passer ouvrirait une saisie sans que personne ne s'en aperçoive.
        tracing::error!("garde_saisie : route sans space_id ou match_report_id dans le chemin");
        return Err(StatusCode::BAD_REQUEST.into_response());
    };
    let portee = charger_portee(state, match_report_id).await?;
    let deps = AccesRapportDeps::from_state(state);
    decider(&deps, user, space_id, portee.as_ref())
        .await
        .map_err(IntoResponse::into_response)
}

async fn charger_portee(
    state: &AppState,
    match_report_id: &str,
) -> Result<Option<PorteeRapport>, Response> {
    match state
        .match_report
        .match_report_repo
        .find_access_scope(match_report_id)
        .await
    {
        Ok(found) => Ok(found.map(
            |(competition_id, home_team_id, away_team_id)| PorteeRapport {
                competition_id,
                home_team_id,
                away_team_id,
            },
        )),
        Err(e) => {
            tracing::error!("garde_saisie : portée de {match_report_id} : {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

/// La décision, séparée du chargement pour se tester sans dépôt : sans
/// session 401, rapport inconnu 404, ni admin ni coach 403.
pub async fn decider(
    deps: &AccesRapportDeps<'_>,
    user: Option<&User>,
    space_id: &str,
    portee: Option<&PorteeRapport>,
) -> Result<(), StatusCode> {
    let Some(user) = user else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let Some(portee) = portee else {
        return Err(StatusCode::NOT_FOUND);
    };
    if is_authorized(deps, user, space_id, portee).await {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::ports::{
        ITeamDataPort, JourneymanPositionDto, RosterPositionDto, TeamInfoDto,
    };
    use crate::app::shared_kernel::bloodbowl::admin_access::FakeAdminAccess;
    use crate::app::shared_kernel::identity::coach_name::CoachName;
    use crate::app::shared_kernel::identity::email::Email;
    use crate::app::shared_kernel::identity::ids::{EntityId, UserId};

    const HOME: &str = "home-team";
    const AWAY: &str = "away-team";
    const SPACE: &str = "00000000000000000000000031";
    const COMPETITION: &str = "00000000000000000000000032";
    const UTILISATEUR: &str = "00000000000000000000000033";

    /// Les équipes dont l'utilisateur est coach.
    struct FakeTeamData(Vec<&'static str>);

    #[async_trait::async_trait]
    impl ITeamDataPort for FakeTeamData {
        async fn is_coach_of_team(&self, team_id: &str, _: &str) -> Result<bool, String> {
            Ok(self.0.contains(&team_id))
        }
        async fn is_team_ready_to_play(&self, _: &str) -> Result<bool, String> {
            Ok(true)
        }
        async fn is_team_in_player_improvement(&self, _: &str) -> Result<bool, String> {
            Ok(false)
        }
        async fn find_team_info(&self, _: &str) -> Option<TeamInfoDto> {
            None
        }
        async fn find_team_value(&self, _: &str) -> Option<u32> {
            None
        }
        async fn find_team_treasury(&self, _: &str) -> Option<u32> {
            None
        }
        async fn find_journeyman_position(&self, _: &str) -> Option<JourneymanPositionDto> {
            None
        }
        async fn find_roster_positions(&self, _: &str) -> Vec<RosterPositionDto> {
            vec![]
        }
        async fn find_team_season(&self, _: &str) -> Option<String> {
            None
        }
    }

    fn user() -> User {
        User::new(
            UserId::try_new(UTILISATEUR).unwrap(),
            CoachName::try_new("Testeur".to_string()).unwrap(),
            None,
            Email::try_new("testeur@example.com".to_string()).unwrap(),
            String::new(),
        )
    }

    fn portee() -> PorteeRapport {
        PorteeRapport {
            competition_id: COMPETITION.to_string(),
            home_team_id: HOME.to_string(),
            away_team_id: AWAY.to_string(),
        }
    }

    async fn decision(
        admin: bool,
        coached: Vec<&'static str>,
        user: Option<&User>,
        portee: Option<&PorteeRapport>,
    ) -> Result<(), StatusCode> {
        let mut admin_access = FakeAdminAccess::new();
        if admin {
            admin_access = admin_access.space_admin(
                &EntityId::try_new(UTILISATEUR).unwrap(),
                &EntityId::try_new(SPACE).unwrap(),
            );
        }
        let team_data = FakeTeamData(coached);
        let deps = AccesRapportDeps {
            admin_access: &admin_access,
            team_data: &team_data,
        };
        decider(&deps, user, SPACE, portee).await
    }

    #[tokio::test]
    async fn an_admin_may_enter_the_report() {
        assert_eq!(
            decision(true, vec![], Some(&user()), Some(&portee())).await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn a_coach_of_either_team_may_enter_the_report() {
        assert_eq!(
            decision(false, vec![HOME], Some(&user()), Some(&portee())).await,
            Ok(())
        );
        assert_eq!(
            decision(false, vec![AWAY], Some(&user()), Some(&portee())).await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn a_third_party_is_forbidden() {
        assert_eq!(
            decision(
                false,
                vec!["une-autre-equipe"],
                Some(&user()),
                Some(&portee())
            )
            .await,
            Err(StatusCode::FORBIDDEN)
        );
    }

    #[tokio::test]
    async fn an_unknown_report_is_not_found() {
        assert_eq!(
            decision(true, vec![], Some(&user()), None).await,
            Err(StatusCode::NOT_FOUND)
        );
    }

    #[tokio::test]
    async fn no_session_is_unauthorized() {
        assert_eq!(
            decision(true, vec![HOME], None, Some(&portee())).await,
            Err(StatusCode::UNAUTHORIZED)
        );
    }
}
