use crate::app::auth::auth_backend::AuthSession;
use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::value_objects::{
    ActionId, ActionPlayer, HatredKeyword, InjuryType, MatchActionType, SequelStat, TeamSide,
    TempPlayerId, TurnNumber,
};
use crate::app::match_report::use_cases::delete_action_use_case::{self, DeleteActionCommand};
use crate::app::match_report::use_cases::record_action_use_case::{
    self, RecordActionCommand, RecordActionError,
};
use crate::app::shared_kernel::bloodbowl::ids::{MatchReportId, PlayerId};
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RecordActionForm {
    pub turn: u8,
    pub player_id: String,
    pub player_type: String,
    pub action_type: String,
    pub injury_type: Option<String>,
    pub sequel_stat: Option<String>,
    /// `Option<bool>` et non `bool` : l'absence dit « la question n'a pas été
    /// posée » — le cas d'une Commotion — quand `Some(false)` dit « posée, et la
    /// réponse est non ». Un booléen nu confondrait les deux.
    pub hate_gained: Option<bool>,
    pub hate_keyword: Option<String>,
}

pub async fn post_action_step3(
    auth_session: AuthSession,
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<RecordActionForm>,
) -> Response {
    let _ = space_id;
    post_action(auth_session, mr_id, TeamSide::Home, form, state).await
}

pub async fn post_action_step4(
    auth_session: AuthSession,
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<RecordActionForm>,
) -> Response {
    let _ = space_id;
    post_action(auth_session, mr_id, TeamSide::Away, form, state).await
}

async fn post_action(
    auth_session: AuthSession,
    mr_id: String,
    side: TeamSide,
    form: RecordActionForm,
    state: AppState,
) -> Response {
    let user = match auth_session.user {
        Some(u) => u,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let mr_id_vo = match MatchReportId::try_new(&mr_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let turn = match TurnNumber::try_new(form.turn) {
        Ok(t) => t,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let player = match build_player(&form.player_id, &form.player_type) {
        Some(p) => p,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };
    let action = match build_action_type(
        &form.action_type,
        form.injury_type.as_deref(),
        form.sequel_stat.as_deref(),
    ) {
        Some(a) => a,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };
    let hatred = match lire_haine(&form) {
        Ok(h) => h,
        Err(motif) => {
            tracing::warn!(action = %form.action_type, "action refusée : {motif}");
            return (StatusCode::UNPROCESSABLE_ENTITY, motif).into_response();
        }
    };
    let cmd = RecordActionCommand {
        match_report_id: mr_id_vo,
        team_side: side,
        turn,
        player,
        action,
        hatred,
        recorded_by: user.id,
    };
    match record_action_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
        state.match_report.player_data.as_ref(),
        state.match_report.keyword_catalog.as_ref(),
    )
    .await
    {
        Ok(outcome) => {
            let trigger = format!(
                r#"{{"actionRecorded": {{"action_id": "{}"}}}}"#,
                outcome.action_id
            );
            Response::builder()
                .header("HX-Trigger", trigger)
                .body(axum::body::Body::empty())
                .unwrap()
        }
        Err(e) => match refus_client(&e) {
            Some(motif) => {
                tracing::warn!("action refusée : {motif} ({e:?})");
                (StatusCode::UNPROCESSABLE_ENTITY, motif).into_response()
            }
            None => {
                tracing::error!("post_action: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
    }
}

/// Les échecs qui viennent de la requête, et non du serveur.
///
/// Extraite pour être testable sans HTTP ni base. `None` signifie « ce n'est
/// pas la faute du client » : la réponse reste un 500, comme avant cette carte.
/// On n'y a **rien élargi** — les autres variantes gardent leur traitement,
/// même si certaines mériteraient mieux.
fn refus_client(e: &RecordActionError) -> Option<&'static str> {
    match e {
        RecordActionError::UnknownKeyword(_) => Some(HAINE_REFUSEE),
        RecordActionError::Domain(DomainError::HatredNotAllowedForInjury) => Some(HAINE_INTERDITE),
        _ => None,
    }
}

const HAINE_REFUSEE: &str = "Ce mot-clef ne peut pas être haï.";
const HAINE_INTERDITE: &str = "Cette blessure ne permet pas de désigner une Haine.";

/// Traduit les deux champs du formulaire en un mot-clef, ou dit pourquoi non.
///
/// **Le gain sans mot-clef ne peut pas descendre plus bas** : la commande porte
/// un `Option<HatredKeyword>`, donc l'état « oui, sans lequel » n'y est pas
/// représentable. Le typage a déplacé la validation là où on ne peut plus
/// l'oublier — ce refus-ci est le dernier endroit où elle a un sens.
fn lire_haine(form: &RecordActionForm) -> Result<Option<HatredKeyword>, &'static str> {
    if form.hate_gained != Some(true) {
        return Ok(None);
    }
    let uid = form
        .hate_keyword
        .as_deref()
        .filter(|u| !u.trim().is_empty())
        .ok_or("Une Haine a été déclarée sans mot-clef.")?;
    HatredKeyword::try_new(uid)
        .map(Some)
        .map_err(|_| HAINE_REFUSEE)
}

pub async fn delete_action(
    auth_session: AuthSession,
    Path((space_id, mr_id, action_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    let _ = space_id;
    let user = match auth_session.user {
        Some(u) => u,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let mr_id_vo = match MatchReportId::try_new(&mr_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let cmd = DeleteActionCommand {
        match_report_id: mr_id_vo,
        action_id: ActionId(action_id),
        deleted_by: user.id,
    };
    match delete_action_use_case::execute(cmd, state.match_report.match_report_repo.as_ref()).await
    {
        Ok(_) => Response::builder()
            .header("HX-Trigger", r#"{"actionDeleted": {}}"#)
            .body(axum::body::Body::empty())
            .unwrap(),
        Err(e) => {
            tracing::error!("delete_action: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn build_player(player_id: &str, player_type: &str) -> Option<ActionPlayer> {
    match player_type {
        "regular" => Some(ActionPlayer::Regular(PlayerId::try_new(player_id).ok()?)),
        "temp" => Some(ActionPlayer::Temp(TempPlayerId(player_id.to_string()))),
        _ => None,
    }
}

fn build_action_type(
    action_type: &str,
    injury: Option<&str>,
    sequel: Option<&str>,
) -> Option<MatchActionType> {
    match action_type {
        "TOUCHDOWN" => Some(MatchActionType::Touchdown),
        "PASSE" => Some(MatchActionType::Passe),
        "INTERCEPTION" => Some(MatchActionType::Interception),
        "AGRESSION" => Some(MatchActionType::Agression),
        "LANCER" => Some(MatchActionType::Lancer),
        "SORTIE" => Some(MatchActionType::Sortie),
        "MVP" => Some(MatchActionType::Mvp),
        // La Haine reste vide ici : sa saisie et sa validation arrivent avec la
        // carte 401, qui a besoin du port de référence que ce parseur n'a pas.
        "BLESSE" => Some(MatchActionType::Blesse {
            injury: build_injury(injury?, sequel)?,
            hatred: None,
            hatred_skill_uid: None,
        }),
        _ => None,
    }
}

fn build_injury(injury: &str, sequel: Option<&str>) -> Option<InjuryType> {
    match injury {
        "COMMOTION" => Some(InjuryType::Commotion),
        "AMOCHE" => Some(InjuryType::Amoche),
        "BLESSURE_SERIEUSE" => Some(InjuryType::BlessureSerieuse),
        "MORT" => Some(InjuryType::Mort),
        "SEQUEL" => Some(InjuryType::Sequel {
            stat: build_sequel(sequel?)?,
        }),
        _ => None,
    }
}

fn build_sequel(stat: &str) -> Option<SequelStat> {
    match stat {
        "AV" => Some(SequelStat::MinusAv),
        "MA" => Some(SequelStat::MinusMa),
        "PA" => Some(SequelStat::MinusPa),
        "AG" => Some(SequelStat::MinusAg),
        "ST" => Some(SequelStat::MinusSt),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(gagnee: Option<bool>, mot: Option<&str>) -> RecordActionForm {
        RecordActionForm {
            turn: 1,
            player_id: "P1".into(),
            player_type: "REGULAR".into(),
            action_type: "BLESSE".into(),
            injury_type: Some("AMOCHE".into()),
            sequel_stat: None,
            hate_gained: gagnee,
            hate_keyword: mot.map(str::to_string),
        }
    }

    /// Le refus que le typage a rendu inévitable ici : la commande porte un
    /// `Option<HatredKeyword>`, donc « oui, sans lequel » n'y est pas
    /// représentable. C'est le dernier endroit où ce contrôle a un sens.
    #[test]
    fn une_haine_declaree_sans_mot_clef_est_refusee() {
        let e = lire_haine(&form(Some(true), None)).expect_err("doit être refusée");
        assert!(e.contains("sans mot-clef"), "motif inattendu : {e}");
        assert!(lire_haine(&form(Some(true), Some("   "))).is_err());
    }

    /// `None` dit « la question n'a pas été posée », `Some(false)` « posée, et
    /// la réponse est non ». Les deux ne donnent pas de Haine, et c'est bien la
    /// seule chose qu'ils ont en commun.
    #[test]
    fn sans_gain_declare_aucune_haine_n_est_lue() {
        assert!(lire_haine(&form(None, Some("DARK_ELF"))).unwrap().is_none());
        assert!(lire_haine(&form(Some(false), Some("DARK_ELF")))
            .unwrap()
            .is_none());
    }

    #[test]
    fn un_mot_clef_bien_forme_est_lu() {
        let mot = lire_haine(&form(Some(true), Some("DARK_ELF")))
            .unwrap()
            .expect("un mot-clef bien formé doit être lu");
        assert_eq!(mot.to_string(), "DARK_ELF");
    }

    #[test]
    fn un_mot_clef_mal_forme_est_refuse_des_le_formulaire() {
        assert!(lire_haine(&form(Some(true), Some("elfe noir"))).is_err());
    }

    #[test]
    fn les_trois_refus_de_la_haine_sont_des_422() {
        assert_eq!(
            refus_client(&RecordActionError::UnknownKeyword("X".into())),
            Some(HAINE_REFUSEE)
        );
        assert_eq!(
            refus_client(&RecordActionError::Domain(
                DomainError::HatredNotAllowedForInjury
            )),
            Some(HAINE_INTERDITE)
        );
    }

    /// Ce qui n'est pas la faute du client reste un 500 : cette carte n'élargit
    /// pas le traitement des autres erreurs.
    #[test]
    fn les_autres_erreurs_restent_des_500() {
        assert_eq!(refus_client(&RecordActionError::NotFound), None);
        assert_eq!(refus_client(&RecordActionError::NotInPreMatchPhase), None);
        assert_eq!(
            refus_client(&RecordActionError::Domain(DomainError::SameTeam)),
            None
        );
    }
}
