use crate::app::players::domain::match_impact::PlayerParticipationStatus;
use crate::app::players::domain::player::TeamId;
use crate::app::players::io::app_events::team_created_listener::skill_category_css;
use crate::app::players::ports::{AcquiredSkillProjection, ISkillCatalogPort, PlayerProjection};
use crate::app::players::use_cases::player_stats_service::{self, ResolvedPlayerStats};
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

// ── View models ───────────────────────────────────────────────────────────────

pub struct SkillTagVm {
    pub name: String,
    pub category_css: String,
}

impl std::fmt::Display for SkillTagVm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Pourquoi un joueur ne jouera pas le prochain match (carte 489).
///
/// **Un `enum` et non un booléen.** Le gabarit n'a pas à connaître les quatre
/// statuts du domaine ni à décider lesquels comptent comme une absence — c'est
/// une règle métier. Mais un `bool` ne distinguerait pas les deux repères
/// affichés. L'énumération dit à la vue exactement ce qu'elle doit savoir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Absence {
    ProchainMatch,
    Retraite,
}

impl Absence {
    /// Le libellé du repère posé à droite du nom. Un barré nu n'explique rien :
    /// un coach qui ouvre la feuille sans contexte ne sait pas s'il s'agit d'une
    /// blessure, d'une suspension ou d'un défaut d'affichage.
    pub fn libelle(self) -> &'static str {
        match self {
            Self::ProchainMatch => "Manque le prochain match",
            Self::Retraite => "A pris sa retraite",
        }
    }

    pub fn icone(self) -> &'static str {
        match self {
            Self::ProchainMatch => "\u{1FA79}",
            Self::Retraite => "\u{2691}",
        }
    }

    /// Depuis le statut porté par la projection.
    ///
    /// `Dead` n'apparaît pas : le tableau lit `find_alive_by_team_id`, et la
    /// carte 488 a sorti les morts de l'effectif visible. `Retired` n'est posé
    /// par aucun code du domaine aujourd'hui — mais `squad_adapter.rs` le range
    /// déjà parmi les indisponibles, et l'écran ne doit pas dire l'inverse de ce
    /// que `teams` calcule.
    fn depuis_le_statut(statut: &str) -> Option<Self> {
        match statut {
            "MissingNextGame" => Some(Self::ProchainMatch),
            "Retired" => Some(Self::Retraite),
            _ => None,
        }
    }
}

pub struct PlayerRowVm {
    pub player_id: String,
    pub jersey: Option<i16>,
    pub personal_name: String,
    pub position_name: String,
    pub base_skills: Vec<SkillTagVm>,
    pub acquired_skills: Vec<AcquiredSkillProjection>,
    /// SPP **encore disponibles**, pas le cumul gagné (carte 492).
    ///
    /// `players_proj.spp` porte le cumul : `PlayerSkillPurchased` et
    /// `PlayerStatIncreased` n'en retirent rien. Le solde est projeté à part,
    /// `spp_remaining`, recalculé depuis `Player::spp_remaining` dans la
    /// transaction de chaque gain et de chaque dépense (carte 569) — la feuille
    /// ne rejoue plus l'agrégat pour le lire.
    pub spp: u32,
    pub value_kpo: i32,
    /// Caractéristiques résolues — base du poste plus les deltas projetés :
    /// séquelles, augmentations achetées en SPP, customisations (carte 569).
    /// `None` si le poste est introuvable au catalogue : la table affiche alors
    /// un tiret plutôt qu'une valeur fausse.
    pub stats: Option<ResolvedPlayerStats>,
    /// Le statut de participation, **tel que le domaine le dit**.
    ///
    /// Il sert au sous-total des disponibles (carte 460) — le gabarit n'y
    /// touche pas. Un booléen aurait fait de la vue le lieu où l'on décide qui
    /// compte ; ici elle transporte la donnée et pose la question à
    /// `disponible()`.
    pub participation: PlayerParticipationStatus,
    /// L'absence au prochain match, quand il y en a une (carte 489).
    ///
    /// La donnée existait déjà : `participation_status` vit dans la projection
    /// et le dépôt le lit — il s'arrêtait ici.
    pub absence: Option<Absence>,
    /// Le nombre de blessures persistantes — nées de blessures sérieuses, et
    /// d'elles seules (carte 568). Lu dans la projection ; zéro rend un tiret.
    pub persistent_injuries: u16,
    /// « Elfe, Blitzer » — les mots-clefs du poste, déjà joints.
    ///
    /// Vide quand le poste n'en porte pas : le template n'affiche alors rien du
    /// tout, plutôt qu'une ligne vide sous le badge.
    pub keywords: String,
}

/// Les mots-clefs du poste, joints pour l'affichage.
///
/// Aucune requête de plus : `find_position` est déjà appelée pour résoudre les
/// compétences de base, et l'adapter y a joint les libellés (carte 405).
fn mots_clefs_du_poste(roster_line_id: &str, catalog: &dyn ISkillCatalogPort) -> String {
    catalog
        .find_position(roster_line_id)
        .map(|p| p.keywords.join(", "))
        .unwrap_or_default()
}

fn build_base_skills(p: &PlayerProjection, catalog: &dyn ISkillCatalogPort) -> Vec<SkillTagVm> {
    let Some(position) = catalog.find_position(&p.roster_line_id) else {
        return p
            .base_skills
            .iter()
            .map(|n| SkillTagVm {
                name: n.clone(),
                category_css: "type-general".to_string(),
            })
            .collect();
    };
    position
        .base_skills
        .iter()
        .filter_map(|uid| catalog.find_skill(uid))
        .map(|s| SkillTagVm {
            name: s.name.clone(),
            category_css: skill_category_css(&s.category).to_string(),
        })
        .collect()
}

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "player-table-fragment.html")]
pub struct PlayerTableTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub team_id: String,
    pub players: Vec<PlayerRowVm>,
    /// Motif d'un enregistrement refusé. Sa présence fait rendre le fragment
    /// **déjà en mode édition** : le coach revient à sa saisie pour la corriger,
    /// au lieu de la perdre et de tout recommencer.
    pub save_error: Option<String>,
    /// Les trois nombres du pied (carte 460). Trois nombres et non une
    /// structure : le gabarit les affiche, il ne les manipule pas.
    pub available_count: usize,
    /// Sert à la mention « n absent, hors du compte », qui n'apparaît que s'il
    /// y en a — sinon la phrase serait un bruit permanent pour un cas rare.
    pub unavailable_count: usize,
    pub available_value_kpo: i32,
}

impl PlayerTableTemplate {
    /// **Le sous-total se calcule ici, une fois**, et non aux trois sites qui
    /// rendent ce tableau : deux d'entre eux servent l'édition de l'effectif,
    /// où un compte divergent serait invisible à la relecture.
    pub fn new(
        space_id: String,
        team_id: String,
        players: Vec<PlayerRowVm>,
        save_error: Option<String>,
    ) -> Self {
        let disponibles = || players.iter().filter(|p| p.participation.disponible());
        let available_count = disponibles().count();
        Self {
            app_routes: AppRoutes::default(),
            space_id,
            team_id,
            available_count,
            unavailable_count: players.len() - available_count,
            available_value_kpo: disponibles().map(|p| p.value_kpo).sum(),
            save_error,
            players,
        }
    }
}

impl IntoResponse for PlayerTableTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("player_table template render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn player_table_widget(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team = TeamId(team_id);
    let players = build_player_rows(&state, &team).await;

    PlayerTableTemplate::new(space_id, team.0, players, None).into_response()
}

/// Lignes de l'effectif actif, prêtes à rendre. Extrait du handler pour que
/// l'endpoint de sauvegarde (carte 294) rende le même tableau sans dupliquer la
/// résolution des caractéristiques ni celle des compétences.
pub async fn build_player_rows(state: &AppState, team: &TeamId) -> Vec<PlayerRowVm> {
    let projections = state
        .players
        .projection_repository
        .find_alive_by_team_id(team)
        .await
        .unwrap_or_default();

    let catalog = state.players.skill_catalog.as_ref();

    projections
        .into_iter()
        .map(|p| {
            let base_skills = build_base_skills(&p, catalog);
            let keywords = mots_clefs_du_poste(&p.roster_line_id, catalog);
            let stats = stats_de_la_projection(&p, catalog);
            PlayerRowVm {
                player_id: p.player_id,
                jersey: p.jersey,
                personal_name: p.personal_name,
                position_name: p.position_name,
                base_skills,
                acquired_skills: p.acquired_skills,
                // Un solde négatif n'existe pas : `Player::spp_remaining` sature.
                spp: u32::try_from(p.spp_remaining).unwrap_or(0),
                value_kpo: p.value_kpo,
                stats,
                participation: PlayerParticipationStatus::from_str(&p.participation_status),
                absence: Absence::depuis_le_statut(&p.participation_status),
                // Un compteur négatif n'existe pas : la colonne est reposée
                // depuis un `u16` de l'agrégat.
                persistent_injuries: u16::try_from(p.persistent_injuries).unwrap_or(0),
                keywords,
            }
        })
        .collect()
}

/// Base du poste plus deltas de la projection — la feuille d'équipe ne
/// rejoue plus aucun agrégat (carte 569).
fn stats_de_la_projection(
    p: &PlayerProjection,
    catalog: &dyn ISkillCatalogPort,
) -> Option<ResolvedPlayerStats> {
    player_stats_service::resolve_stats_from_deltas(
        &p.roster_line_id,
        [p.ma_delta, p.st_delta, p.ag_delta, p.pa_delta, p.av_delta],
        catalog,
    )
}

#[cfg(test)]
mod tests_sous_total {
    use super::*;

    fn joueur(valeur: i32, statut: PlayerParticipationStatus) -> PlayerRowVm {
        PlayerRowVm {
            player_id: "p".to_string(),
            jersey: None,
            personal_name: String::new(),
            position_name: String::new(),
            base_skills: vec![],
            acquired_skills: vec![],
            spp: 0,
            value_kpo: valeur,
            stats: None,
            participation: statut,
            absence: Absence::depuis_le_statut(match statut {
                PlayerParticipationStatus::Available => "Available",
                PlayerParticipationStatus::MissingNextGame => "MissingNextGame",
                PlayerParticipationStatus::Retired => "Retired",
                PlayerParticipationStatus::Dead => "Dead",
            }),
            persistent_injuries: 0,
            keywords: String::new(),
        }
    }

    fn pied(joueurs: Vec<PlayerRowVm>) -> PlayerTableTemplate {
        PlayerTableTemplate::new("s".to_string(), "t".to_string(), joueurs, None)
    }

    /// Le cœur de la carte : un absent est dans la liste, hors du compte.
    #[test]
    fn le_sous_total_exclut_les_indisponibles() {
        use PlayerParticipationStatus::*;
        let t = pied(vec![
            joueur(100, Available),
            joueur(50, MissingNextGame),
            joueur(80, Available),
            joueur(70, Retired),
        ]);
        assert_eq!(t.available_count, 2);
        assert_eq!(t.unavailable_count, 2);
        assert_eq!(t.available_value_kpo, 180, "50 et 70 ne comptent pas");
    }

    /// Le cas le plus fréquent — la mention d'absence ne doit pas s'afficher.
    #[test]
    fn sans_indisponible_la_mention_n_apparait_pas() {
        let t = pied(vec![
            joueur(100, PlayerParticipationStatus::Available),
            joueur(60, PlayerParticipationStatus::Available),
        ]);
        assert_eq!(t.unavailable_count, 0);
        assert_eq!(t.available_value_kpo, 160);
    }

    /// **Un journalier disponible compte** (épic E15). Il est un joueur de
    /// l'effectif, il apparaît dans la liste, et la valeur d'équipe le compte
    /// aussi — l'exclure ici ferait diverger le sous-total de ce qu'il vérifie.
    #[test]
    fn un_journalier_disponible_entre_dans_le_compte() {
        let t = pied(vec![
            joueur(100, PlayerParticipationStatus::Available),
            joueur(50, PlayerParticipationStatus::Available), // le journalier
        ]);
        assert_eq!((t.available_count, t.available_value_kpo), (2, 150));
    }

    /// Un effectif vide ne rend pas de pied — le gabarit rend
    /// `players-widget-empty` à la place du tableau. Les compteurs restent
    /// cohérents pour autant : zéro partout, jamais une soustraction négative.
    #[test]
    fn un_effectif_vide_ne_compte_rien() {
        let t = pied(vec![]);
        assert_eq!(
            (
                t.available_count,
                t.unavailable_count,
                t.available_value_kpo
            ),
            (0, 0, 0)
        );
    }

    /// **Deux défauts opposés, tous deux justes.**
    ///
    /// La vue échoue *ouvert* : un statut inconnu ne barre pas la ligne, parce
    /// que barrer ferait disparaître un effectif entier sur une faute de
    /// frappe (cf. `un_statut_inconnu_ne_barre_rien`).
    ///
    /// Le compte échoue *fermé* : le même statut inconnu ne compte pas, parce
    /// qu'un total gonflé passerait pour juste alors qu'on le regarde
    /// précisément pour en vérifier un autre.
    ///
    /// Un joueur peut donc être affiché sans repère et hors du compte. C'est le
    /// moindre mal des deux côtés, et ce test tient l'asymétrie pour qu'elle ne
    /// soit pas « corrigée » par mégarde.
    #[test]
    fn un_statut_inconnu_est_affiche_sans_repere_mais_hors_du_compte() {
        let inconnu = PlayerParticipationStatus::from_str("Suspendu");
        let mut p = joueur(90, inconnu);
        p.absence = Absence::depuis_le_statut("Suspendu");

        assert_eq!(p.absence, None, "la ligne n'est pas barrée");
        let t = pied(vec![p]);
        assert_eq!(
            (t.available_count, t.available_value_kpo),
            (0, 0),
            "mais il ne gonfle pas le total"
        );
    }
    // ── La colonne BP (carte 568) ─────────────────────────────────────────

    fn ligne_bp(n: u16, statut: PlayerParticipationStatus) -> String {
        let mut j = joueur(50, statut);
        j.persistent_injuries = n;
        let html = pied(vec![j]).render().unwrap();
        let debut = html.find("<td class=\"player-bp").expect("cellule BP");
        html[debut..debut + html[debut..].find("</td>").unwrap()].to_string()
    }

    #[test]
    fn aucune_blessure_persistante_rend_un_tiret() {
        let cellule = ligne_bp(0, PlayerParticipationStatus::Available);
        assert!(cellule.contains('—'), "{cellule}");
        assert!(!cellule.contains("player-bp--some"), "{cellule}");
    }

    #[test]
    fn des_blessures_persistantes_rendent_leur_nombre() {
        let cellule = ligne_bp(2, PlayerParticipationStatus::Available);
        assert!(cellule.contains("player-bp--some"), "{cellule}");
        assert!(cellule.trim_end().ends_with('2'), "{cellule}");
    }

    #[test]
    fn l_en_tete_et_le_pied_portent_la_colonne() {
        let html = pied(vec![joueur(50, PlayerParticipationStatus::Available)])
            .render()
            .unwrap();
        assert!(html.contains("title=\"Blessures persistantes\">BP</th>"));
        assert!(html.contains("<td class=\"player-bp player-foot-dash\">"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Les quatre statuts, et ce que la vue en fait** (carte 489).
    ///
    /// `Dead` rend `None` sans que ce soit une décision d'affichage : le tableau
    /// lit `find_alive_by_team_id`, et un mort n'atteint jamais cette fonction.
    /// La correspondance le dit quand même, pour qu'un futur appelant qui
    /// oublierait le filtre n'obtienne pas un joueur barré « à la retraite ».
    #[test]
    fn chaque_statut_donne_son_absence() {
        assert_eq!(Absence::depuis_le_statut("Available"), None);
        assert_eq!(
            Absence::depuis_le_statut("MissingNextGame"),
            Some(Absence::ProchainMatch)
        );
        assert_eq!(
            Absence::depuis_le_statut("Retired"),
            Some(Absence::Retraite)
        );
        assert_eq!(Absence::depuis_le_statut("Dead"), None);
    }

    /// Un statut inconnu ne barre pas la ligne.
    ///
    /// **Échouer ouvert, ici, est le bon sens** : un statut que la vue ne
    /// connaît pas viendrait d'un domaine qui a évolué sans elle. Barrer par
    /// défaut ferait disparaître visuellement un effectif entier sur une valeur
    /// mal orthographiée ; ne rien barrer laisse la liste lisible et le défaut
    /// se voit au premier joueur blessé qui cesse d'être signalé.
    #[test]
    fn un_statut_inconnu_ne_barre_rien() {
        assert_eq!(Absence::depuis_le_statut(""), None);
        assert_eq!(Absence::depuis_le_statut("missingnextgame"), None);
        assert_eq!(Absence::depuis_le_statut("Suspendu"), None);
    }

    /// Les deux repères disent des choses différentes — c'est la raison d'être
    /// de l'`enum` plutôt que d'un booléen.
    #[test]
    fn les_deux_absences_ne_se_disent_pas_pareil() {
        assert_ne!(
            Absence::ProchainMatch.libelle(),
            Absence::Retraite.libelle()
        );
        assert_ne!(Absence::ProchainMatch.icone(), Absence::Retraite.icone());
        assert!(Absence::ProchainMatch.libelle().contains("prochain match"));
        assert!(Absence::Retraite.libelle().contains("retraite"));
    }
}
