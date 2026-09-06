//! Les journaliers qu'un coach peut garder, à la phase de recrutement.
//!
//! # Pourquoi ce panneau appartient à `players`
//!
//! Le journalier vit ici. Son nom, ses SPP, la compétence qu'il a gagnée au
//! match : `players` les possède et sait les rendre. Le `CLAUDE.md` en tire la
//! conséquence — *« l'assemblage de données issues de plusieurs BCs se fait
//! exclusivement au niveau du frontend, par composition de widgets HTMX »*.
//!
//! La première version de la carte faisait transiter un libellé d'amélioration
//! dans un DTO de port, jusqu'au domaine de `teams`. C'était l'inverse de la
//! règle, et ça posait un second site de mise en forme du même fait.
//!
//! # Ce que l'hôte décide, et que ce widget ne peut pas savoir
//!
//! L'état du bouton. Effectif complet, trésorerie insuffisante, déjà au
//! panier : trois refus qui appartiennent au panier de `teams`. Il les calcule
//! et les passe en paramètres — `recrutables` et `motif` —, avec le gabarit du
//! POST dans `action_url`.
//!
//! **Ce widget ne connaît donc aucune route de `teams`.** Il rend ce qu'on lui
//! donne, et poste où on lui dit.
//!
//! # Un seul motif, et ce n'est pas une approximation
//!
//! Le plafond frappe tout le monde ou personne. S'il ne frappe pas, les seuls
//! bloqués sont les trop chers, et leur cause est la même. Les deux règles ne
//! produisent jamais deux motifs différents en même temps.

use crate::app::players::domain::player::{RosterMembership, TeamId};
use crate::app::players::io::app_events::player_creation::base_position_kpo;
use crate::app::players::ports::PlayerProjection;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

pub struct JourneymanRowVm {
    pub player_id: String,
    pub name: String,
    pub position_name: String,
    pub spp: u32,
    /// La compétence gagnée, ou la caractéristique améliorée. `None` quand il
    /// n'a rien gagné — le gabarit affiche « aucune », jamais une case vide,
    /// qui se lirait comme une donnée manquante.
    pub improvement: Option<String>,
    pub price_kpo: u32,
    /// Le tarif du poste. **Sert à décomposer**, pas à s'afficher seul : sans
    /// « 65 + 20 d'amélioration », un coach qui voit 85 pour un poste à 65
    /// croit à une erreur.
    pub base_price_kpo: u32,
    pub action_url: String,
    /// `None` quand il est recrutable. Sinon le motif que l'hôte a fourni.
    pub blocage: Option<String>,
}

impl JourneymanRowVm {
    /// L'écart au tarif, calculé **au rendu** : c'est une soustraction, pas une
    /// donnée à transporter.
    pub fn supplement(&self) -> u32 {
        self.price_kpo.saturating_sub(self.base_price_kpo)
    }

    /// Le prix se décompose seulement s'il y a quelque chose à expliquer.
    /// Pas de « 65 + 0 », qui ferait chercher une amélioration inexistante.
    pub fn prix_decompose(&self) -> bool {
        self.supplement() > 0
    }
}

#[derive(Template)]
#[template(path = "journeymen-widget.html")]
pub struct JourneymenTemplate {
    pub journeymen: Vec<JourneymanRowVm>,
}

/// Ce que l'hôte injecte.
///
/// `recrutables` est une liste d'identifiants séparés par des virgules —
/// l'encodage le plus court pour ce que c'est, et le seul à ne pas demander de
/// désérialisation.
#[derive(Debug, Deserialize)]
pub struct EtatInjecte {
    /// Le gabarit du POST, avec `{player_id}` à substituer.
    #[serde(default)]
    pub action_url: String,
    #[serde(default)]
    pub recrutables: String,
    #[serde(default)]
    pub motif: String,
}

pub async fn journeymen_widget(
    Path((space_id, team_id)): Path<(String, String)>,
    Query(etat): Query<EtatInjecte>,
    State(state): State<AppState>,
) -> Response {
    let _ = space_id;
    let joueurs = match state
        .players
        .projection_repository
        .find_by_team_id(&TeamId(team_id))
        .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("players journeymen_widget: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let catalog = state.players.skill_catalog.as_ref();
    let journeymen = joueurs
        .into_iter()
        .filter(est_journalier)
        .map(|p| ligne(p, &etat, catalog))
        .collect();

    match (JourneymenTemplate { journeymen }).render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("players journeymen_widget: rendu: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn est_journalier(p: &PlayerProjection) -> bool {
    RosterMembership::from_str(&p.membership) == RosterMembership::Journeyman
}

fn ligne(
    p: PlayerProjection,
    etat: &EtatInjecte,
    catalog: &dyn crate::app::players::ports::ISkillCatalogPort,
) -> JourneymanRowVm {
    let recrutable = etat
        .recrutables
        .split(',')
        .any(|id| id == p.player_id.as_str());
    JourneymanRowVm {
        action_url: etat.action_url.replace("{player_id}", &p.player_id),
        blocage: match recrutable {
            true => None,
            false => Some(etat.motif.clone()),
        },
        base_price_kpo: base_position_kpo(&p.roster_line_id, catalog),
        improvement: amelioration(&p),
        name: match p.personal_name.is_empty() {
            true => p.position_name.clone(),
            false => p.personal_name.clone(),
        },
        position_name: p.position_name,
        spp: p.spp.max(0) as u32,
        price_kpo: p.value_kpo.max(0) as u32,
        player_id: p.player_id,
    }
}

/// Ce que le journalier a gagné, en un mot.
///
/// **La compétence l'emporte** quand les deux existent, parce qu'elle se
/// nomme : « Blocage » dit plus que « +1 ST ». Le cas est aujourd'hui
/// impossible — un match ne donne pas assez de SPP pour les deux — mais la
/// règle doit être tranchée avant de le devenir.
///
/// La première compétence acquise **est** celle du match : un journalier naît à
/// l'ouverture du rapport et n'a pas de passé. Ce raccourci ne vaut que pour
/// lui, et c'est pourquoi cette fonction vit dans son widget plutôt que dans un
/// service partagé.
fn amelioration(p: &PlayerProjection) -> Option<String> {
    if let Some(skill) = p.acquired_skills.first() {
        return Some(skill.skill_name.clone());
    }
    [
        ("MA", p.ma_delta),
        ("ST", p.st_delta),
        ("AG", p.ag_delta),
        ("PA", p.pa_delta),
        ("AV", p.av_delta),
    ]
    .into_iter()
    .find(|(_, delta)| *delta != 0)
    .map(|(nom, delta)| format!("{delta:+} {nom}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::ports::AcquiredSkillProjection;

    fn projection(acquises: Vec<&str>, deltas: [i16; 5], valeur: i32) -> PlayerProjection {
        PlayerProjection {
            player_id: "j1".into(),
            team_id: "t1".into(),
            space_id: "s1".into(),
            position_name: "Piétaille".into(),
            roster_line_id: "DEMO_GRANIT__PIETAILLE".into(),
            personal_name: String::new(),
            jersey: Some(3),
            base_skills: vec![],
            acquired_skills: acquises
                .into_iter()
                .map(|nom| AcquiredSkillProjection {
                    skill_id: nom.to_lowercase(),
                    skill_name: nom.to_string(),
                    category_css: String::new(),
                    mode: "Normal".into(),
                    spp_cost: 6,
                })
                .collect(),
            spp: 6,
            value_kpo: valeur,
            participation_status: "Available".into(),
            membership: "Journeyman".into(),
            ma_delta: deltas[0],
            st_delta: deltas[1],
            ag_delta: deltas[2],
            pa_delta: deltas[3],
            av_delta: deltas[4],
        }
    }

    fn ligne_vm(p: PlayerProjection, base: u32, recrutables: &str) -> JourneymanRowVm {
        let etat = EtatInjecte {
            action_url: "/app/s/teams/t/recruitment/journeyman/{player_id}".into(),
            recrutables: recrutables.into(),
            motif: "effectif complet : 16 joueurs maximum".into(),
        };
        let recrutable = etat.recrutables.split(',').any(|id| id == p.player_id);
        JourneymanRowVm {
            action_url: etat.action_url.replace("{player_id}", &p.player_id),
            blocage: match recrutable {
                true => None,
                false => Some(etat.motif.clone()),
            },
            base_price_kpo: base,
            improvement: amelioration(&p),
            name: p.position_name.clone(),
            position_name: p.position_name,
            spp: p.spp.max(0) as u32,
            price_kpo: p.value_kpo.max(0) as u32,
            player_id: p.player_id,
        }
    }

    #[test]
    fn un_journalier_sans_amelioration_rend_none() {
        assert_eq!(amelioration(&projection(vec![], [0; 5], 65)), None);
    }

    #[test]
    fn l_amelioration_nomme_la_competence_ou_le_delta() {
        assert_eq!(
            amelioration(&projection(vec!["Blocage"], [0; 5], 85)).as_deref(),
            Some("Blocage")
        );
        assert_eq!(
            amelioration(&projection(vec![], [0, 1, 0, 0, 0], 95)).as_deref(),
            Some("+1 ST")
        );
        // La compétence l'emporte : elle se nomme.
        assert_eq!(
            amelioration(&projection(vec!["Blocage"], [0, 1, 0, 0, 0], 115)).as_deref(),
            Some("Blocage")
        );
    }

    /// « 65 + 20 d'amélioration ». Sans cette décomposition, un coach qui voit
    /// 85 pour un poste à 65 croit à une erreur.
    #[test]
    fn le_prix_se_decompose_au_dela_du_tarif() {
        let vm = ligne_vm(projection(vec!["Blocage"], [0; 5], 85), 65, "j1");
        assert!(vm.prix_decompose());
        assert_eq!(vm.supplement(), 20);
    }

    /// Pas de « 65 + 0 », qui ferait chercher une amélioration inexistante.
    #[test]
    fn le_prix_ne_se_decompose_pas_a_l_egalite() {
        let vm = ligne_vm(projection(vec![], [0; 5], 65), 65, "j1");
        assert!(!vm.prix_decompose());
        assert_eq!(vm.supplement(), 0);
    }

    /// L'état vient de l'hôte : `players` ne sait pas pourquoi il est bloqué,
    /// il affiche le motif qu'on lui donne.
    #[test]
    fn un_journalier_non_recrutable_porte_le_motif() {
        let recrutable = ligne_vm(projection(vec![], [0; 5], 65), 65, "j1");
        assert_eq!(recrutable.blocage, None);
        assert!(recrutable.action_url.ends_with("/journeyman/j1"));

        let bloque = ligne_vm(projection(vec![], [0; 5], 65), 65, "autre");
        assert_eq!(
            bloque.blocage.as_deref(),
            Some("effectif complet : 16 joueurs maximum")
        );
    }

    /// Le panneau se masque quand la liste est vide : la plupart des matchs se
    /// jouent sans journalier, et un panneau vide poserait la question
    /// « qu'est-ce que j'ai raté ? » à chaque phase de recrutement.
    #[test]
    fn la_liste_est_vide_sans_journalier() {
        let rendu = (JourneymenTemplate { journeymen: vec![] })
            .render()
            .unwrap();
        assert!(!rendu.contains("rec-journeymen"));
    }

    #[test]
    fn seuls_les_journaliers_sont_retenus() {
        let mut permanent = projection(vec![], [0; 5], 65);
        permanent.membership = "Active".into();
        assert!(!est_journalier(&permanent));
        assert!(est_journalier(&projection(vec![], [0; 5], 65)));
    }
}
