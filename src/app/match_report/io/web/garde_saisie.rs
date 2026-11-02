//! Les gardes des routes d'un compte rendu (cartes 622 et 634).
//!
//! **Deux gardes, pour deux groupes de routes** (`router.rs`) :
//!
//! - `garde_suivi`, sur `routes_de_suivi()` : **lire** le compte rendu en
//!   cours. Les admins d'espace ou de compétition, et les coachs des deux
//!   équipes du match — la règle d'`is_authorized`, celle du récapitulatif.
//! - `garde_saisie`, sur `routes_de_saisie()` : **écrire**, et ouvrir les
//!   écrans qui préparent l'écriture. La même règle, puis `can_write_report` :
//!   en « Je saisis tout », le démarreur et les admins seuls (R5 de E21).
//!
//! Les autres membres de l'espace reçoivent 403 des deux. L'autre coach, que
//! seule la seconde refuse, reçoit 403 sur une écriture ou une requête htmx,
//! et une **redirection 303 vers le match en direct** quand il demandait une
//! page complète (carte 636) : il y suit le match, sans bouton d'écriture.
//! Elle remplace la page « Ce match est saisi par … » de la carte 634 : un
//! refus en 403 n'aurait rien montré à une navigation htmx, la mise en page
//! n'affichant aucune réponse 4xx hors 422.
//!
//! **Où** : posées une fois, par `route_layer`. Une route ajoutée à un groupe
//! est gardée sans qu'on y pense ; c'est le point du découpage, préféré à vingt
//! appels recopiés dans autant de handlers. Même patron que
//! `garde_action_equipe` dans `teams`.
//!
//! **Hors des groupes** : le hors-calendrier (`/new`), les feuilles
//! imprimables et le récapitulatif, qui se garde lui-même.
//!
//! # La limite
//!
//! Elles lisent `match_report_id` dans le **chemin**, comme
//! `space_scope_middleware`. Une route qui désignerait son rapport autrement
//! leur échapperait : toutes celles des deux groupes le portent dans le chemin.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::domain::user::User;
use crate::app::match_report::use_cases::match_report_access_service::{
    can_write_report, is_authorized, load_entry_scope, AccesRapportDeps, EntryScope, PorteeRapport,
};
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use crate::web::htmx::veut_la_page_entiere;
use axum::extract::{Path, Request, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use axum::RequestPartsExt;
use std::collections::HashMap;

/// Lire le compte rendu en cours : admins et coachs des deux équipes.
pub async fn garde_suivi(
    State(state): State<AppState>,
    auth_session: AuthSession,
    request: Request,
    next: Next,
) -> Response {
    let (mut parts, corps) = request.into_parts();
    let params = path_params(&mut parts).await;
    if let Err(refus) = autoriser_suivi(&state, auth_session.user.as_ref(), &params).await {
        return refus;
    }
    next.run(Request::from_parts(parts, corps)).await
}

/// Écrire dans le compte rendu : la garde de suivi, puis `can_write_report`.
pub async fn garde_saisie(
    State(state): State<AppState>,
    auth_session: AuthSession,
    request: Request,
    next: Next,
) -> Response {
    let (mut parts, corps) = request.into_parts();
    let params = path_params(&mut parts).await;
    let wants_page = veut_une_page(&parts.method, &parts.headers);
    let user = auth_session.user.as_ref();
    if let Err(refus) = autoriser_saisie(&state, user, &params, wants_page).await {
        return refus;
    }
    next.run(Request::from_parts(parts, corps)).await
}

async fn path_params(parts: &mut Parts) -> HashMap<String, String> {
    match parts.extract::<Path<HashMap<String, String>>>().await {
        Ok(Path(params)) => params,
        Err(_) => HashMap::new(),
    }
}

/// Une page complète mérite d'être menée au match en direct plutôt qu'un 403
/// nu : un `GET` du navigateur, ou une navigation htmx vers le contenu
/// applicatif.
fn veut_une_page(method: &Method, headers: &HeaderMap) -> bool {
    method == Method::GET && veut_la_page_entiere(headers)
}

/// Ce que la garde de suivi a établi, et dont la garde de saisie repart.
struct RapportSuivi<'a> {
    user: &'a User,
    space_id: &'a str,
    match_report_id: &'a str,
    portee: PorteeRapport,
}

/// La règle d'`is_authorized`. `decider` n'accepte ni session absente ni
/// rapport inconnu : les deux sont là quand il répond `Ok`.
async fn autoriser_suivi<'a>(
    state: &AppState,
    user: Option<&'a User>,
    params: &'a HashMap<String, String>,
) -> Result<RapportSuivi<'a>, Response> {
    let (space_id, match_report_id) = identifiants_du_chemin(params)?;
    let portee = charger_portee(state, match_report_id).await?;
    let deps = AccesRapportDeps::from_state(state);
    let decision = decider(&deps, user, space_id, portee.as_ref()).await;
    match (decision, user, portee) {
        (Ok(()), Some(user), Some(portee)) => Ok(RapportSuivi {
            user,
            space_id,
            match_report_id,
            portee,
        }),
        (Err(status), _, _) => Err(status.into_response()),
        _ => Err(StatusCode::INTERNAL_SERVER_ERROR.into_response()),
    }
}

/// `(space_id, match_report_id)`, lus dans le chemin.
fn identifiants_du_chemin(params: &HashMap<String, String>) -> Result<(&str, &str), Response> {
    match (params.get("space_id"), params.get("match_report_id")) {
        (Some(space_id), Some(match_report_id)) => Ok((space_id, match_report_id)),
        _ => {
            // Une route du groupe sans ces deux segments : mal placée. La
            // laisser passer ouvrirait une saisie sans que personne ne s'en
            // aperçoive.
            tracing::error!("garde_saisie : route sans space_id ou match_report_id dans le chemin");
            Err(StatusCode::BAD_REQUEST.into_response())
        }
    }
}

async fn autoriser_saisie(
    state: &AppState,
    user: Option<&User>,
    params: &HashMap<String, String>,
    wants_page: bool,
) -> Result<(), Response> {
    let rapport = autoriser_suivi(state, user, params).await?;
    let entry = charger_entree(state, rapport.match_report_id).await?;
    let deps = AccesRapportDeps::from_state(state);
    let RapportSuivi {
        user,
        space_id,
        portee,
        match_report_id,
    } = &rapport;
    if can_write_report(&deps, user, space_id, portee, &entry).await {
        return Ok(());
    }
    Err(refuser_l_ecriture(space_id, match_report_id, wants_page))
}

/// Le mode de saisie et le démarreur. Absents alors que la portée vient
/// d'être lue : une projection incohérente, qui refuse plutôt que d'ouvrir.
async fn charger_entree(state: &AppState, match_report_id: &str) -> Result<EntryScope, Response> {
    let repo = state.match_report.match_report_repo.as_ref();
    match load_entry_scope(repo, match_report_id).await {
        Ok(Some(entry)) => Ok(entry),
        Ok(None) => Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("garde_saisie : mode de saisie de {match_report_id} : {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

/// Le refus fait à l'autre coach : 403 nu sur une écriture ou un fragment, et
/// la redirection vers le match en direct quand c'est une page complète qu'il
/// demandait (carte 636).
fn refuser_l_ecriture(space_id: &str, match_report_id: &str, wants_page: bool) -> Response {
    if !wants_page {
        return StatusCode::FORBIDDEN.into_response();
    }
    let live = AppRoutes::default()
        .match_report
        .live(space_id, match_report_id);
    Redirect::to(&live).into_response()
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

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(*k, v.parse().unwrap());
        }
        h
    }

    /// La redirection ne vaut que pour une page : une écriture, ou un
    /// fragment demandé par htmx, reçoit un 403 nu (cartes 634 et 636).
    #[test]
    fn only_a_page_request_is_led_to_the_live_match() {
        assert!(veut_une_page(&Method::GET, &headers(&[])));
        assert!(veut_une_page(
            &Method::GET,
            &headers(&[("hx-request", "true"), ("hx-target", "app-content")])
        ));
        assert!(!veut_une_page(
            &Method::GET,
            &headers(&[("hx-request", "true")])
        ));
        assert!(!veut_une_page(&Method::POST, &headers(&[])));
        assert!(!veut_une_page(&Method::DELETE, &headers(&[])));
    }

    #[test]
    fn a_page_request_is_redirected_to_the_live_match() {
        let response = refuser_l_ecriture("espace", "rapport", true);
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let location = response.headers().get("location").unwrap();
        assert_eq!(
            location.to_str().unwrap(),
            "/app/espace/match-report/rapport/live"
        );
    }

    #[test]
    fn a_write_is_forbidden_without_redirection() {
        let response = refuser_l_ecriture("espace", "rapport", false);
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn no_session_is_unauthorized() {
        assert_eq!(
            decision(true, vec![HOME], None, Some(&portee())).await,
            Err(StatusCode::UNAUTHORIZED)
        );
    }
}
