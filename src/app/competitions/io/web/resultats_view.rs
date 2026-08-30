use crate::app::auth::domain::user::User;
use crate::app::competitions::domain::match_day_repository_port::PairingDisplayDto;
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::collections::{BTreeMap, HashSet};

pub struct MatchResultatVm {
    pub home_name: String,
    pub home_roster: String,
    pub home_coach: String,
    pub home_logo: Option<String>,
    pub home_initials: String,
    pub away_name: String,
    pub away_roster: String,
    pub away_coach: String,
    pub away_logo: Option<String>,
    pub away_initials: String,
    pub is_in_progress: bool,
    pub is_completed: bool,
    pub home_score: Option<u32>,
    pub away_score: Option<u32>,
    pub home_cas: Option<u32>,
    pub away_cas: Option<u32>,
    pub report_url: Option<String>,
    /// « Journée 3 », quand la liste n'a pas d'en-tête de groupe pour le dire.
    ///
    /// `None` sur l'onglet compétition, où l'en-tête de journée le porte déjà —
    /// le répéter dans chacun des six blocs du groupe serait du bruit.
    pub round_label: Option<String>,
    /// Victoire, nul ou défaite, **du point de vue d'une équipe de référence**.
    ///
    /// `None` sur l'onglet compétition, où la question n'a pas de sens : il n'y
    /// a pas d'équipe de référence. `None` aussi sur un match non joué.
    ///
    /// **Une `Option` et non un booléen ni une chaîne vide** : « pas d'équipe de
    /// référence » et « match nul » sont deux choses, qu'un `String::new()`
    /// confondrait.
    pub outcome: Option<MatchOutcome>,
}

/// Ce que le match **est** pour l'équipe consultée, pas comment il s'affiche.
/// Le gabarit en tire sa lettre et sa classe ; c'est lui qui décide de la
/// couleur, pas le view model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchOutcome {
    Win,
    Draw,
    Loss,
}

pub struct JourneeResultatsVm {
    pub label: String,
    pub matches: Vec<MatchResultatVm>,
}

/// Autorisation à cliquer sur une ligne de résultat pour naviguer vers le
/// rapport de match. `is_admin` (admin d'espace ou de compétition) autorise
/// toutes les lignes ; sinon seules les lignes où l'utilisateur coache l'une
/// des deux équipes sont autorisées.
pub struct ResultAuthorization {
    pub is_admin: bool,
    pub my_team_ids: HashSet<String>,
}

impl ResultAuthorization {
    /// Utilisée par l'onglet admin — l'accès à la page admin est déjà
    /// conditionné à être admin d'espace ou de compétition en amont
    /// (`render_admin_page`), donc toutes les lignes y sont autorisées.
    pub fn unrestricted() -> Self {
        Self {
            is_admin: true,
            my_team_ids: HashSet::new(),
        }
    }

    pub fn allows(&self, home_team_id: &str, away_team_id: &str) -> bool {
        self.is_admin
            || self.my_team_ids.contains(home_team_id)
            || self.my_team_ids.contains(away_team_id)
    }
}

/// Calcule l'autorisation de l'utilisateur courant pour la page publique des
/// résultats : admin d'espace, admin de compétition (par id ou par nom,
/// même règle que `admin_page.rs`), ou coach d'une des équipes inscrites
/// cette saison.
pub async fn compute_authorization(
    state: &AppState,
    user: &User,
    space_id: &SpaceId,
    competition_id: &CompetitionId,
    season_id: &str,
) -> ResultAuthorization {
    let is_space_admin = matches!(
        state
            .competitions
            .space_member_port
            .find_member_profile(&user.id, space_id)
            .await,
        Some(SpaceProfile::SpaceAdmin)
    );

    let is_comp_admin = match state
        .competitions
        .competition_repository
        .find_base_info(competition_id)
        .await
    {
        Ok(Some(info)) => {
            let user_id_str = user.id.to_string();
            let coach_name_str = user.coach_name.clone().into_inner();
            info.admin_ids.contains(&user_id_str) || info.admin_names.contains(&coach_name_str)
        }
        _ => false,
    };

    let is_admin = is_space_admin || is_comp_admin;

    let my_team_ids = if is_admin {
        HashSet::new()
    } else {
        let user_id_str = user.id.to_string();
        state
            .competitions
            .team_info_port
            .find_enrolled_teams(season_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|t| t.coach_id == user_id_str)
            .map(|t| t.team_id)
            .collect()
    };

    ResultAuthorization {
        is_admin,
        my_team_ids,
    }
}

pub async fn load_resultats(
    state: &AppState,
    season_id: &str,
    cursor: Option<i32>,
) -> Result<Vec<PairingDisplayDto>, Response> {
    state
        .competitions
        .match_day_repository
        .list_resultats(season_id, cursor, 3)
        .await
        .map_err(|e| {
            tracing::error!("resultats_view: list_resultats: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })
}

/// Les matchs d'une équipe, **à plat** et dans l'ordre que la requête a posé.
///
/// Pas de `build_journees` ici : une équipe joue un match par journée, et
/// grouper donnerait quinze groupes d'un match, chacun titré « 1 match ». Le
/// libellé de journée entre dans le bloc à la place, par le `round_label` de la
/// carte 476.
///
/// L'ordre vient du `ORDER BY` et n'est pas retouché — le retrier ici en
/// ferait une seconde vérité, qui divergerait de la requête le jour où l'une
/// des deux changerait.
pub fn build_team_matches(
    rows: Vec<PairingDisplayDto>,
    authz: &ResultAuthorization,
    team_id: &str,
) -> Vec<MatchResultatVm> {
    rows.into_iter()
        .map(|row| to_resultat_vm(row, authz, Some(team_id)))
        .collect()
}

pub fn build_journees(
    rows: Vec<PairingDisplayDto>,
    max_rounds: usize,
    authz: &ResultAuthorization,
) -> (Vec<JourneeResultatsVm>, Option<i32>) {
    let mut by_round: BTreeMap<i32, (String, Vec<MatchResultatVm>)> = BTreeMap::new();
    for row in rows {
        let entry = by_round
            .entry(row.round_position)
            .or_insert_with(|| (row.round_name.clone(), Vec::new()));
        entry.1.push(to_resultat_vm(row, authz, None));
    }

    let mut journees: Vec<(i32, JourneeResultatsVm)> = by_round
        .into_iter()
        .map(|(pos, (label, matches))| (pos, JourneeResultatsVm { label, matches }))
        .collect();

    journees.sort_by(|a, b| b.0.cmp(&a.0));
    journees.truncate(max_rounds);

    let next_cursor = if journees.len() == max_rounds {
        journees.last().map(|(pos, _)| *pos)
    } else {
        None
    };

    (journees.into_iter().map(|(_, j)| j).collect(), next_cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_allows_any_match() {
        let authz = ResultAuthorization::unrestricted();
        assert!(authz.allows("any-home", "any-away"));
    }

    #[test]
    fn coach_of_home_team_is_allowed() {
        let authz = ResultAuthorization {
            is_admin: false,
            my_team_ids: HashSet::from(["team-a".to_string()]),
        };
        assert!(authz.allows("team-a", "team-b"));
    }

    #[test]
    fn coach_of_away_team_is_allowed() {
        let authz = ResultAuthorization {
            is_admin: false,
            my_team_ids: HashSet::from(["team-b".to_string()]),
        };
        assert!(authz.allows("team-a", "team-b"));
    }

    // ── La pastille V/N/D (carte 477) ────────────────────────────────────────

    fn rencontre(
        domicile: &str,
        exterieur: &str,
        score: Option<(i32, i32)>,
        statut: &str,
    ) -> PairingDisplayDto {
        PairingDisplayDto {
            pairing_id: "p1".into(),
            round_id: "r1".into(),
            round_name: "Journée 3".into(),
            round_position: 3,
            round_date_start: None,
            round_date_end: None,
            round_day_type: "fixed_date".into(),
            home_team_id: domicile.into(),
            home_team_name: "Domicile".into(),
            home_roster_name: "Nains".into(),
            home_coach_name: "Castor".into(),
            home_logo_url: None,
            home_initials: "DO".into(),
            away_team_id: exterieur.into(),
            away_team_name: "Extérieur".into(),
            away_roster_name: "Elfes".into(),
            away_coach_name: "Brume".into(),
            away_logo_url: None,
            away_initials: "EX".into(),
            match_status: statut.into(),
            home_score: score.map(|(h, _)| h),
            away_score: score.map(|(_, a)| a),
            home_casualties: Some(0),
            away_casualties: Some(0),
            match_report_url: None,
        }
    }

    fn vm_pour(row: PairingDisplayDto, equipe: &str) -> MatchResultatVm {
        to_resultat_vm(row, &ResultAuthorization::unrestricted(), Some(equipe))
    }

    #[test]
    fn une_victoire_a_domicile_donne_la_pastille_v() {
        let vm = vm_pour(rencontre("A", "B", Some((3, 1)), "completed"), "A");
        assert_eq!(vm.outcome, Some(MatchOutcome::Win));
    }

    /// **Le test qui compte.**
    ///
    /// Une inversion de `is_home` donne une pastille fausse **une fois sur
    /// deux** — la moitié où l'équipe se déplace. Le défaut ressemble alors à
    /// une donnée corrompue et non à une erreur de code, et se cherche du
    /// mauvais côté. C'est le seul test que le précédent ne couvre pas : un
    /// `home_score` lu quel que soit le camp passerait l'autre.
    #[test]
    fn une_victoire_a_l_exterieur_donne_aussi_la_pastille_v() {
        let vm = vm_pour(rencontre("B", "A", Some((1, 3)), "completed"), "A");
        assert_eq!(vm.outcome, Some(MatchOutcome::Win));
    }

    #[test]
    fn une_defaite_se_lit_des_deux_cotes() {
        assert_eq!(
            vm_pour(rencontre("A", "B", Some((1, 3)), "completed"), "A").outcome,
            Some(MatchOutcome::Loss)
        );
        assert_eq!(
            vm_pour(rencontre("B", "A", Some((3, 1)), "completed"), "A").outcome,
            Some(MatchOutcome::Loss)
        );
    }

    #[test]
    fn un_score_egal_donne_la_pastille_n() {
        let vm = vm_pour(rencontre("A", "B", Some((2, 2)), "completed"), "A");
        assert_eq!(vm.outcome, Some(MatchOutcome::Draw));
    }

    /// `None`, et surtout **pas « N »** : un match qui n'a pas eu lieu n'est pas
    /// un match nul.
    #[test]
    fn un_match_a_venir_n_a_pas_de_pastille() {
        let vm = vm_pour(rencontre("A", "B", None, "upcoming"), "A");
        assert_eq!(vm.outcome, None);
        // Ni un match en cours, dont le score n'est pas encore acquis.
        let vm = vm_pour(rencontre("A", "B", Some((1, 0)), "in_progress"), "A");
        assert_eq!(vm.outcome, None);
    }

    /// Sans équipe de référence — l'onglet compétition — la question n'a pas de
    /// sens, et le libellé de journée est déjà porté par l'en-tête de groupe.
    #[test]
    fn sans_equipe_de_reference_ni_pastille_ni_libelle_de_journee() {
        let vm = to_resultat_vm(
            rencontre("A", "B", Some((3, 1)), "completed"),
            &ResultAuthorization::unrestricted(),
            None,
        );
        assert_eq!(vm.outcome, None);
        assert_eq!(vm.round_label, None);
    }

    #[test]
    fn avec_une_equipe_de_reference_le_libelle_de_journee_entre_dans_le_bloc() {
        let vm = vm_pour(rencontre("A", "B", Some((3, 1)), "completed"), "A");
        assert_eq!(vm.round_label.as_deref(), Some("Journée 3"));
    }

    /// La liste plate garde l'ordre de la requête : le retrier ici créerait une
    /// seconde vérité, qui divergerait le jour où l'une des deux changerait.
    #[test]
    fn la_liste_plate_garde_l_ordre_de_la_requete() {
        let rows = vec![
            rencontre("A", "B", Some((1, 0)), "in_progress"),
            rencontre("C", "A", None, "upcoming"),
            rencontre("A", "D", Some((2, 2)), "completed"),
        ];
        let vms = build_team_matches(rows, &ResultAuthorization::unrestricted(), "A");

        assert_eq!(vms.len(), 3);
        assert_eq!(
            vms.iter().map(|v| v.outcome).collect::<Vec<_>>(),
            vec![None, None, Some(MatchOutcome::Draw)]
        );
    }

    #[test]
    fn coach_of_neither_team_is_not_allowed() {
        let authz = ResultAuthorization {
            is_admin: false,
            my_team_ids: HashSet::from(["team-c".to_string()]),
        };
        assert!(!authz.allows("team-a", "team-b"));
    }
}

/// Le view model d'un match.
///
/// `reference` est l'équipe dont on regarde la fiche, ou `None` sur une page de
/// compétition. Elle décide des deux champs de la carte 476 : la pastille se
/// dérive de son point de vue, et le libellé de journée entre dans le bloc
/// parce qu'une liste d'équipe est plate.
///
/// **Un seul paramètre pour deux décisions**, alors qu'elles sont distinctes en
/// principe — la pastille dépend d'une équipe de référence, le libellé du fait
/// que la liste n'a pas d'en-tête de groupe. Les deux coïncident chez les deux
/// seuls appelants ; les séparer inventerait un relevé d'équipe groupé par
/// journée, que rien ne demande.
fn to_resultat_vm(
    row: PairingDisplayDto,
    authz: &ResultAuthorization,
    reference: Option<&str>,
) -> MatchResultatVm {
    let is_completed = row.match_status == "completed";
    let is_in_progress = row.match_status == "in_progress";
    let outcome = reference.and_then(|equipe| issue_pour(&row, equipe, is_completed));
    let round_label = reference.map(|_| row.round_name.clone());
    let report_url = if authz.allows(&row.home_team_id, &row.away_team_id) {
        row.match_report_url
    } else {
        None
    };
    MatchResultatVm {
        home_name: row.home_team_name,
        home_roster: row.home_roster_name,
        home_coach: row.home_coach_name,
        home_logo: row.home_logo_url,
        home_initials: row.home_initials,
        away_name: row.away_team_name,
        away_roster: row.away_roster_name,
        away_coach: row.away_coach_name,
        away_logo: row.away_logo_url,
        away_initials: row.away_initials,
        is_in_progress,
        is_completed,
        home_score: row.home_score.map(|v| v as u32),
        away_score: row.away_score.map(|v| v as u32),
        home_cas: row.home_casualties.map(|v| v as u32),
        away_cas: row.away_casualties.map(|v| v as u32),
        report_url,
        round_label,
        outcome,
    }
}

/// Victoire, nul ou défaite **du point de vue de `equipe`**.
///
/// # Le sens de `is_home` est ce qui compte ici
///
/// L'inverser donne une pastille fausse une fois sur deux — la moitié où
/// l'équipe reçoit. Le défaut ressemble alors à une donnée corrompue et non à
/// une erreur de code, et se cherche du mauvais côté.
///
/// Un match non joué n'a pas d'issue : ni `None` par défaut ni « nul », mais
/// l'absence — et c'est la même absence que celle d'une page de compétition,
/// où la question n'a pas d'équipe pour se poser.
fn issue_pour(row: &PairingDisplayDto, equipe: &str, is_completed: bool) -> Option<MatchOutcome> {
    if !is_completed {
        return None;
    }
    let recoit = row.home_team_id == equipe;
    let (pour, contre) = match recoit {
        true => (row.home_score?, row.away_score?),
        false => (row.away_score?, row.home_score?),
    };
    Some(match pour.cmp(&contre) {
        std::cmp::Ordering::Greater => MatchOutcome::Win,
        std::cmp::Ordering::Equal => MatchOutcome::Draw,
        std::cmp::Ordering::Less => MatchOutcome::Loss,
    })
}
