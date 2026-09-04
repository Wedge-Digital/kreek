use crate::app::teams::ports::SquadMemberDto;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct TeamMatchContextJson {
    pub team_id: String,
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
    pub dedicated_fans: u32,
    /// Les joueurs **alignables** au prochain match, pas l'effectif.
    ///
    /// C'est ce dont l'écran a besoin : il en déduit les journaliers, et la
    /// règle réelle — `count_available_by_team_id`, qu'interroge
    /// `match_report` — ne compte que `participation_status = 'Available'`.
    ///
    /// Le champ s'appelait `player_count` et rendait `find_squad(...).len()`,
    /// soit l'effectif entier : l'écran annonçait donc moins de journaliers que
    /// le rapport n'en ajouterait, pour tout blessé et, depuis la carte 488,
    /// pour tout mort — dont on a pourtant décidé qu'il en appelle un.
    pub available_player_count: u32,
    pub ctv: u32,
    pub treasury: u32,
    pub journeyman_type: String,
}

/// Ceux qui peuvent tenir une place au coup d'envoi.
///
/// Le prédicat est celui du domaine (`SquadPresence::alignable`) et non une
/// comparaison de chaîne : l'ACL a traduit le vocabulaire de `players` une fois
/// pour toutes, et le refaire ici en ferait deux vérités à tenir.
fn alignables(membres: &[SquadMemberDto]) -> u32 {
    membres.iter().filter(|m| m.presence.alignable()).count() as u32
}

#[derive(Deserialize)]
pub struct TeamMatchContextQuery {
    pub team_id: String,
}

pub async fn get_team_match_context_json(
    Path(_space_id): Path<String>,
    Query(query): Query<TeamMatchContextQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team = match state.teams.team_repository.find_by_id(&query.team_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    // `ISquadPort` a remplacé le port de comptage : on charge l'effectif pour
    // le compter ici. Un `COUNT(*)` était plus direct, mais deux ports vers la
    // même source coûtaient plus cher à maintenir que ce chargement — seize
    // joueurs au plus.
    //
    // Le port rend l'effectif **entier**, et c'est son contrat : la valeur
    // d'équipe ne somme que les disponibles quand les quotas comptent tous les
    // occupants. C'est donc ici que l'on choisit, et le prédicat vient du
    // domaine.
    let available_player_count =
        alignables(&state.teams.squad_port.find_squad(&query.team_id).await);

    let journeyman_type = state
        .teams
        .journeyman_type_port
        .journeyman_type_for_roster(&team.roster_id.to_string())
        .position_name;

    Json(TeamMatchContextJson {
        team_id: team.id.to_string(),
        team_name: team.name.to_string(),
        coach_name: team.coach_name.clone(),
        roster_name: team.roster_name.to_string(),
        dedicated_fans: team.dedicated_fans.into_inner() as u32,
        available_player_count,
        ctv: team.team_value.0,
        treasury: team.treasury.0,
        journeyman_type,
    })
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::teams::domain::basket::SquadPresence;

    fn membre(id: &str, presence: SquadPresence) -> SquadMemberDto {
        SquadMemberDto {
            player_id: id.into(),
            roster_line_id: "L".into(),
            jersey: None,
            personal_name: id.into(),
            position_name: "Poste".into(),
            spp: 0,
            value_kpo: 50,
            presence,
        }
    }

    /// **Le défaut d'origine** : `find_squad(...).len()` comptait tout le monde,
    /// donc l'écran annonçait moins de journaliers que le rapport n'en
    /// ajouterait. La règle réelle — `count_available_by_team_id` — ne retient
    /// que les alignables.
    #[test]
    fn seuls_les_alignables_sont_comptes() {
        let effectif = vec![
            membre("a", SquadPresence::Alignable),
            membre("b", SquadPresence::Alignable),
            membre("c", SquadPresence::Empeche),
            membre("d", SquadPresence::Perdu),
        ];
        assert_eq!(alignables(&effectif), 2, "quatre membres, deux alignables");
    }

    /// Les deux indisponibilités comptent pareil **ici**, et c'est ce qui
    /// distingue ce comptage de celui des quotas : un blessé garde sa place
    /// dans l'effectif, mais il ne tient pas une place sur le terrain — comme
    /// le mort, il appelle un journalier.
    #[test]
    fn un_blesse_appelle_un_journalier_autant_qu_un_mort() {
        assert_eq!(alignables(&[membre("a", SquadPresence::Empeche)]), 0);
        assert_eq!(alignables(&[membre("a", SquadPresence::Perdu)]), 0);
    }

    #[test]
    fn un_effectif_entierement_alignable_est_compte_en_entier() {
        let effectif: Vec<_> = (0..11)
            .map(|i| membre(&i.to_string(), SquadPresence::Alignable))
            .collect();
        assert_eq!(alignables(&effectif), 11);
    }

    #[test]
    fn un_effectif_vide_ne_compte_personne() {
        assert_eq!(alignables(&[]), 0);
    }
}
