use crate::app::auth::auth_backend::AuthSession;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::teams::domain::costly_mistakes::tranches_affichables;
use crate::app::teams::domain::team::GamePhase;
use crate::app::teams::domain::value_objects::{IncidentType, Kpo};
use crate::app::teams::use_cases::apply_costly_mistakes_use_case::CostlyMistakesOutcome;
use crate::app::teams::use_cases::apply_costly_mistakes_use_case::{
    self, ApplyCostlyMistakesCommand, ApplyCostlyMistakesError,
};
use crate::app::teams::use_cases::roster_edit_access_service;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "teams-costly-mistakes.html")]
pub struct CostlyMistakesPageTemplate {
    pub app_routes: AppRoutes,
    pub team_name: String,
    pub treasury: u32,
    pub band_label: String,
    pub bands: Vec<BandVm>,
    pub roll_url: String,
    pub team_url: String,
}

#[derive(Template)]
#[template(path = "teams-costly-mistakes-result.html")]
pub struct CostlyMistakesResultTemplate {
    pub verdict_kind: &'static str,
    pub verdict_icon: &'static str,
    pub verdict_title: String,
    pub verdict_text: String,
    pub calc: Vec<CalcLineVm>,
    pub bands: Vec<BandVm>,
    pub roll: u8,
}

/// L'écran du jet.
///
/// **Hors phase, 422** — comme le fait déjà `dismissals.rs:70`. Cette famille
/// d'écrans n'a pas de sens hors de sa phase, et la conséquence est assumée :
/// un coach qui recharge après le jet ne reverra pas son résultat. Le montant
/// figure au grand livre avec le motif `CostlyMistake`.
pub async fn get_costly_mistakes_page(
    auth_session: AuthSession,
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    if auth_session.user.is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let team = match state.teams.team_repository.find_by_id(&team_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("costly_mistakes page: chargement de {team_id}: {e:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    if team.game_phase != Some(GamePhase::CostlyMistakes) {
        return StatusCode::UNPROCESSABLE_ENTITY.into_response();
    }

    let bands = BandVm::all_from_domain(team.treasury, None);
    let band_label = bands
        .iter()
        .find(|b| b.courante)
        .map(|b| b.label.clone())
        .unwrap_or_default();
    let routes = AppRoutes::default();
    let page = CostlyMistakesPageTemplate {
        app_routes: AppRoutes::default(),
        team_name: team.name.to_string(),
        treasury: team.treasury.0,
        band_label,
        bands,
        roll_url: routes.teams.costly_mistakes_roll(&space_id, &team_id),
        team_url: routes.teams.team_detail(&space_id, &team_id),
    };
    match page.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("costly_mistakes page: rendu: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Lance le dé des erreurs coûteuses et l'applique.
///
/// **Aucun extracteur de corps** : l'équipe est dans le chemin, le coach dans la
/// session. Rien n'entre, donc rien n'est à valider — et le client ne peut pas
/// proposer de jet.
pub async fn post_costly_mistakes_roll(
    auth_session: AuthSession,
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(team_id_vo) = TeamId::try_new(&team_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    // Le droit garde le **POST**, pas seulement l'affichage : l'URL est
    // devinable, et un jet a un effet financier.
    let Ok(Some(team)) = state.teams.team_repository.find_by_id(&team_id).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let autorise = roster_edit_access_service::peut_modifier_effectif(
        &team,
        &user.id,
        &user.coach_name.clone().into_inner(),
        state.teams.access_port.as_ref(),
    )
    .await;
    if !autorise {
        tracing::warn!(
            team_id = %team_id,
            user_id = %user.id,
            "jet refusé : ni propriétaire de l'équipe, ni administrateur"
        );
        return StatusCode::FORBIDDEN.into_response();
    }

    let cmd = ApplyCostlyMistakesCommand {
        team_id: team_id_vo,
    };
    match apply_costly_mistakes_use_case::execute(
        cmd,
        state.teams.team_repository.as_ref(),
        state.teams.dice.as_ref(),
    )
    .await
    {
        Ok(issue) => fragment(issue),
        Err(ApplyCostlyMistakesError::TeamNotFound) => StatusCode::NOT_FOUND.into_response(),
        // **409 et non 422** : la requête est bien formée, c'est l'état qui a
        // changé. Typiquement un second jet — `CostlyMistakesApplied` a reposé
        // `ReadyToPlay`, donc la garde de phase du domaine refuse. L'idempotence
        // ne demande ni verrou ni jeton, elle sort du modèle.
        Err(ApplyCostlyMistakesError::Domain(e)) => {
            tracing::warn!(team_id = %team_id, "jet refusé : {e:?}");
            StatusCode::CONFLICT.into_response()
        }
        Err(e) => {
            tracing::error!("post_costly_mistakes_roll: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Le résultat, rendu en fragment — le composant qui tient l'animation
/// l'affiche à l'échéance du plancher.
fn fragment(issue: CostlyMistakesOutcome) -> Response {
    let (verdict_kind, verdict_icon, verdict_title, verdict_text) = verdict(&issue);
    let vue = CostlyMistakesResultTemplate {
        verdict_kind,
        verdict_icon,
        verdict_title,
        verdict_text,
        bands: BandVm::all_from_domain(issue.treasury_before, Some(issue.incident)),
        calc: CalcLineVm::all_from_domain(&issue),
        roll: issue.roll,
    };
    match vue.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("costly_mistakes: rendu du résultat: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn verdict(issue: &CostlyMistakesOutcome) -> (&'static str, &'static str, String, String) {
    match issue.incident {
        IncidentType::None => (
            "safe",
            "😌",
            "Crise évitée".to_string(),
            "Votre trésorier a eu chaud. Rien n'est perdu.".to_string(),
        ),
        IncidentType::Minor => (
            "minor",
            "💸",
            "Incident mineur".to_string(),
            format!(
                "Quelques dépenses imprévues : {} kPo s'envolent.",
                issue.gp_lost.0
            ),
        ),
        IncidentType::Major => (
            "major",
            "🔥",
            "Incident majeur".to_string(),
            format!(
                "La moitié de la caisse y passe : {} kPo perdus.",
                issue.gp_lost.0
            ),
        ),
        IncidentType::Catastrophe => (
            "catastrophe",
            "💀",
            "Catastrophe".to_string(),
            format!(
                "Il ne reste que {} kPo. {} kPo se sont évaporés.",
                issue.treasury_after.0, issue.gp_lost.0
            ),
        ),
    }
}

// ── Vues ─────────────────────────────────────────────────────────────────────

/// Une ligne de la table de déclenchement.
///
/// `courante` met en évidence la tranche du coach : il ne tire pas lui-même, et
/// à défaut d'avoir prise sur le résultat il doit pouvoir **vérifier qu'il est
/// juste**.
pub struct BandVm {
    pub label: String,
    pub safe: String,
    pub minor: String,
    pub major: String,
    pub catastrophe: String,
    pub courante: bool,
    /// La colonne atteinte par le jet, une fois celui-ci fait — vide avant.
    pub touchee: &'static str,
}

impl BandVm {
    pub fn all_from_domain(treasury: Kpo, incident: Option<IncidentType>) -> Vec<Self> {
        tranches_affichables()
            .into_iter()
            .map(|t| {
                let courante = t.contient(treasury);
                Self {
                    label: if t.max == u32::MAX {
                        format!("{} kPo et +", t.min)
                    } else {
                        format!("{} – {} kPo", t.min, t.max)
                    },
                    safe: t.plage(IncidentType::None),
                    minor: t.plage(IncidentType::Minor),
                    major: t.plage(IncidentType::Major),
                    catastrophe: t.plage(IncidentType::Catastrophe),
                    courante,
                    touchee: match incident {
                        Some(i) if courante => colonne(i),
                        _ => "",
                    },
                }
            })
            .collect()
    }
}

fn colonne(incident: IncidentType) -> &'static str {
    match incident {
        IncidentType::None => "safe",
        IncidentType::Minor => "minor",
        IncidentType::Major => "major",
        IncidentType::Catastrophe => "catastrophe",
    }
}

/// Une ligne du calcul.
///
/// **Une liste, et non quatre champs nommés** : chaque incident a son
/// enchaînement, et un VM à champs fixes obligerait le template à savoir lequel
/// afficher. Un coach qui perd 340 kPo doit pouvoir refaire l'opération sans
/// demander à personne.
pub struct CalcLineVm {
    pub label: String,
    pub value: String,
    /// `total` pour la perte, `rest` pour le reliquat, vide pour les étapes.
    pub kind: &'static str,
    /// Les dés secondaires, affichés en petit à côté de leur ligne.
    pub dice: Vec<u8>,
}

impl CalcLineVm {
    fn etape(label: &str, value: String) -> Self {
        Self {
            label: label.to_string(),
            value,
            kind: "",
            dice: vec![],
        }
    }

    /// Le calcul en entier, du solde de départ à ce qui reste.
    pub fn all_from_domain(issue: &CostlyMistakesOutcome) -> Vec<Self> {
        let mut lignes = vec![Self::etape(
            "Trésorerie",
            format!("{} kPo", issue.treasury_before.0),
        )];
        match issue.incident {
            IncidentType::None => {}
            IncidentType::Minor => lignes.push(Self {
                label: "Dé de dégâts (1D3) × 10".to_string(),
                value: format!("{} kPo", issue.gp_lost.0),
                kind: "",
                dice: issue.damage_dice.clone(),
            }),
            IncidentType::Major => {
                let moitie = issue.treasury_before.0 as f64 / 2.0;
                lignes.push(Self::etape("La moitié", format!("{moitie} kPo")));
                // L'arrondi ne s'affiche que s'il change quelque chose : une
                // trésorerie multiple de 10 se divise rond, et répéter la même
                // valeur sur deux lignes n'apprend rien — constaté à l'écran sur
                // une équipe à 50 510 kPo.
                if moitie != issue.gp_lost.0 as f64 {
                    lignes.push(Self::etape(
                        "Arrondie au 5 kPo inférieur",
                        format!("{} kPo", issue.gp_lost.0),
                    ));
                }
            }
            IncidentType::Catastrophe => lignes.push(Self {
                label: "Il ne reste que 2D6 × 10".to_string(),
                value: format!("{} kPo", issue.treasury_after.0),
                kind: "",
                dice: issue.damage_dice.clone(),
            }),
        }
        lignes.push(Self {
            label: "Perte".to_string(),
            value: format!("− {} kPo", issue.gp_lost.0),
            kind: "total",
            dice: vec![],
        });
        lignes.push(Self {
            label: "Il vous reste".to_string(),
            value: format!("{} kPo", issue.treasury_after.0),
            kind: "rest",
            dice: vec![],
        });
        lignes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue(
        treasury: u32,
        incident: IncidentType,
        gp_lost: u32,
        dice: Vec<u8>,
    ) -> CostlyMistakesOutcome {
        CostlyMistakesOutcome {
            roll: 1,
            damage_dice: dice,
            incident,
            gp_lost: Kpo(gp_lost),
            treasury_before: Kpo(treasury),
            treasury_after: Kpo(treasury - gp_lost),
        }
    }

    /// Une seule tranche est mise en évidence, et c'est celle du coach.
    #[test]
    fn la_tranche_courante_est_celle_de_la_tresorerie() {
        let lignes = BandVm::all_from_domain(Kpo(345), None);
        assert_eq!(lignes.len(), 6);
        let courantes: Vec<&str> = lignes
            .iter()
            .filter(|l| l.courante)
            .map(|l| l.label.as_str())
            .collect();
        assert_eq!(courantes, vec!["300 – 399 kPo"]);
        assert!(
            lignes.iter().all(|l| l.touchee.is_empty()),
            "avant le jet, aucune case n'est touchée"
        );
    }

    /// La case atteinte n'est marquée que sur la ligne du coach : la même
    /// colonne existe sur les six, et les marquer toutes ne dirait rien.
    #[test]
    fn apres_le_jet_seule_la_tranche_courante_porte_la_case_touchee() {
        let lignes = BandVm::all_from_domain(Kpo(345), Some(IncidentType::Major));
        let touchees: Vec<(&str, &str)> = lignes
            .iter()
            .filter(|l| !l.touchee.is_empty())
            .map(|l| (l.label.as_str(), l.touchee))
            .collect();
        assert_eq!(touchees, vec![("300 – 399 kPo", "major")]);
    }

    #[test]
    fn la_derniere_tranche_s_affiche_sans_borne_haute() {
        let lignes = BandVm::all_from_domain(Kpo(900), None);
        assert_eq!(lignes[5].label, "600 kPo et +");
        assert!(lignes[5].courante);
    }

    /// Une crise évitée n'a pas d'étape : le solde, puis une perte nulle.
    #[test]
    fn le_calcul_d_une_crise_evitee_ne_retire_rien() {
        let lignes = CalcLineVm::all_from_domain(&issue(345, IncidentType::None, 0, vec![]));
        let valeurs: Vec<(&str, &str)> = lignes
            .iter()
            .map(|l| (l.label.as_str(), l.value.as_str()))
            .collect();
        assert_eq!(
            valeurs,
            vec![
                ("Trésorerie", "345 kPo"),
                ("Perte", "− 0 kPo"),
                ("Il vous reste", "345 kPo"),
            ]
        );
    }

    /// Le calcul d'un majeur montre l'arrondi, qui est le point contestable.
    #[test]
    fn le_calcul_d_un_majeur_montre_la_moitie_puis_l_arrondi() {
        let lignes = CalcLineVm::all_from_domain(&issue(345, IncidentType::Major, 170, vec![]));
        let valeurs: Vec<(&str, &str)> = lignes
            .iter()
            .map(|l| (l.label.as_str(), l.value.as_str()))
            .collect();
        assert_eq!(
            valeurs,
            vec![
                ("Trésorerie", "345 kPo"),
                ("La moitié", "172.5 kPo"),
                ("Arrondie au 5 kPo inférieur", "170 kPo"),
                ("Perte", "− 170 kPo"),
                ("Il vous reste", "175 kPo"),
            ]
        );
    }

    /// Une trésorerie multiple de 10 se divise rond : la ligne d'arrondi
    /// répéterait la précédente, et n'apprendrait rien. Constaté à l'écran sur
    /// une équipe à 50 510 kPo.
    #[test]
    fn l_arrondi_ne_s_affiche_pas_quand_il_ne_change_rien() {
        let lignes = CalcLineVm::all_from_domain(&issue(500, IncidentType::Major, 250, vec![]));
        let labels: Vec<&str> = lignes.iter().map(|l| l.label.as_str()).collect();
        assert_eq!(
            labels,
            vec!["Trésorerie", "La moitié", "Perte", "Il vous reste"],
            "500 / 2 = 250 tout rond : pas de ligne d'arrondi"
        );
    }

    #[test]
    fn le_calcul_d_un_mineur_porte_son_de() {
        let lignes = CalcLineVm::all_from_domain(&issue(150, IncidentType::Minor, 20, vec![2]));
        assert_eq!(lignes[1].dice, vec![2]);
        assert_eq!(lignes[1].value, "20 kPo");
        assert_eq!(lignes.last().unwrap().value, "130 kPo");
    }

    #[test]
    fn le_calcul_d_une_catastrophe_dit_ce_qui_reste() {
        let lignes =
            CalcLineVm::all_from_domain(&issue(560, IncidentType::Catastrophe, 490, vec![3, 4]));
        assert_eq!(lignes[1].dice, vec![3, 4]);
        assert_eq!(lignes[1].value, "70 kPo", "la catastrophe dit ce qui reste");
        assert_eq!(lignes.last().unwrap().value, "70 kPo");
    }

    /// Les deux dernières lignes sont toujours la perte puis le reliquat : c'est
    /// ce que le template met en évidence.
    #[test]
    fn le_calcul_finit_toujours_par_la_perte_et_le_reliquat() {
        for (incident, perte, des) in [
            (IncidentType::None, 0, vec![]),
            (IncidentType::Minor, 20, vec![2]),
            (IncidentType::Major, 170, vec![]),
        ] {
            let lignes = CalcLineVm::all_from_domain(&issue(345, incident, perte, des));
            let n = lignes.len();
            assert_eq!(lignes[n - 2].kind, "total", "{incident:?}");
            assert_eq!(lignes[n - 1].kind, "rest", "{incident:?}");
        }
    }
}
