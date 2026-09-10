//! La barre latérale des journées, et le conteneur du panneau.
//!
//! # `etat` et `resume` viennent du domaine, pas du gabarit
//!
//! Le gabarit choisit une pastille et imprime une phrase ; il ne décide pas
//! laquelle. `etat` sort de `statut_de(...)` — la fonction **libre** de la carte
//! 511, et c'est précisément pourquoi elle est libre : cette barre latérale lit
//! des DTO de la base, pas des agrégats, et doit répondre à la même question que
//! `PresenceSurvey::statut`. Une règle calculée à deux endroits finit par l'être
//! de deux façons, et celle-ci croise deux champs et une horloge.
//!
//! # La garde vaut aussi sur les fragments
//!
//! `require_admin_access` sur chaque handler, celui-ci compris : sans quoi le
//! chemin htmx du changement d'onglet contournerait le contrôle. `space_scope`
//! garantit qu'une ressource appartient à l'espace de l'URL, **pas** que
//! l'appelant en est administrateur — et il n'a aucun résolveur pour `round_id`,
//! qui passe donc librement (carte 416).

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::match_day::{MatchDay, MatchDayType};
use crate::app::competitions::domain::presence_survey::Desaccord;
use crate::app::competitions::domain::presence_survey::{
    statut_de, Appariement, EtatJournee, Fermeture, PresenceSurvey, SurveyDeadline, SurveyStatus,
};
use crate::app::competitions::domain::presence_survey_repository_port::SurveySummaryDto;
use crate::app::competitions::domain::tirage::{DrawProposal, Historique, ProposedPairing};
use crate::app::competitions::io::web::admin::admin_page::require_admin_access;
use crate::app::competitions::io::web::admin::admin_scope::journee_de_la_saison;
use crate::app::competitions::use_cases::presences::etat_journee::etat_de_la_journee;
use crate::app::competitions::use_cases::presences::propose_repair_use_case::RepairProposal;
use crate::app::competitions::use_cases::presences::survey_roster_service;
use crate::app::competitions::use_cases::presences::survey_roster_service::{
    LignePresence, RosterDeCampagne,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use time::macros::format_description;
use time::OffsetDateTime;

// ── La barre latérale ────────────────────────────────────────────────────────

pub struct PresenceRoundItemVm {
    pub round_id: String,
    pub name: String,
    /// `repos` · `aucun` · `en_cours` · `clos` · `apparie`
    ///
    /// **Pas de `defection`** : le désaccord se calcule par `desaccord`, qui exige
    /// les appariements de la journée — que ce DTO ne porte pas. La barre latérale
    /// dit donc « appariée », et c'est le panneau qui dira « défection à traiter »
    /// (cartes 520 et 521). Enrichir `list_summaries` pour l'afficher ici coûterait
    /// une jointure par journée, pour une nuance que l'écran voisin porte déjà.
    pub etat: String,
    pub resume: String,
    pub is_rest: bool,
}

impl PresenceRoundItemVm {
    pub fn all_from_domain(resumes: &[SurveySummaryDto], aujourd_hui: &DateString) -> Vec<Self> {
        resumes
            .iter()
            .map(|dto| Self::from_domain(dto, aujourd_hui))
            .collect()
    }

    fn from_domain(dto: &SurveySummaryDto, aujourd_hui: &DateString) -> Self {
        let etat = etat_de(dto, aujourd_hui);
        Self {
            round_id: dto.round_id.clone(),
            name: dto.round_name.clone(),
            resume: resume_de(dto, &etat),
            etat,
            is_rest: dto.is_rest,
        }
    }
}

fn etat_de(dto: &SurveySummaryDto, aujourd_hui: &DateString) -> String {
    if dto.is_rest {
        return "repos".to_string();
    }
    let Some(deadline) = dto.deadline.as_ref() else {
        return "aucun".to_string();
    };
    if dto.appariee == Some(true) {
        return "apparie".to_string();
    }
    match statut_de(
        &fermeture_de(dto),
        &echeance(deadline, &dto.round_id),
        aujourd_hui,
    ) {
        SurveyStatus::Ouverte => "en_cours".to_string(),
        SurveyStatus::Close(_) => "clos".to_string(),
    }
}

/// Une échéance illisible **est signalée**, pas escamotée : elle ne peut venir
/// que d'une écriture hors du domaine, et un `unwrap_or_default` silencieux ferait
/// disparaître une campagne de la barre latérale sans une ligne de journal.
fn echeance(brut: &str, round_id: &str) -> SurveyDeadline {
    SurveyDeadline::try_new(brut.to_string()).unwrap_or_else(|_| {
        tracing::error!(round_id, deadline = brut, "échéance de campagne illisible");
        SurveyDeadline::try_new("1970-01-01".to_string()).expect("date de repli valide")
    })
}

fn fermeture_de(dto: &SurveySummaryDto) -> Fermeture {
    match dto.close_le.as_ref() {
        None => Fermeture::Aucune,
        Some(le) => {
            crate::app::competitions::domain::presence_survey::FermeeLe::try_new(le.to_string())
                .map(|le| Fermeture::Decidee { le })
                .unwrap_or_else(|_| {
                    tracing::error!(round_id = dto.round_id, close_le = le, "clôture illisible");
                    Fermeture::Aucune
                })
        }
    }
}

/// **Aucun nombre inventé.** Le résumé d'une journée appariée ne dit pas combien
/// de matchs elle porte : `SurveySummaryDto` ne le sait pas, et le déduire de
/// `presents / 2` serait faux dès qu'une équipe est exemptée. La maquette annonce
/// « 4 matchs créés » ; l'afficher demandera que le DTO compte les appariements.
fn resume_de(dto: &SurveySummaryDto, etat: &str) -> String {
    match etat {
        "repos" => "Journée de repos".to_string(),
        "aucun" => "Aucun sondage".to_string(),
        "apparie" => "Journée appariée".to_string(),
        "clos" => format!("{} présents sur {}", dto.presents, dto.attendues),
        _ => format!("{} réponses sur {}", dto.reponses, dto.attendues),
    }
}

#[derive(Template)]
#[template(path = "admin/widgets/presences-rounds.html")]
pub struct PresenceRoundsTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub rounds: Vec<PresenceRoundItemVm>,
}

impl IntoResponse for PresenceRoundsTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("presences rounds render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn presences_rounds_widget(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin_access(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &state,
    )
    .await
    {
        return resp;
    }

    // Une seule requête pour toute la saison. Les compter journée par journée
    // ferait vingt allers-retours pour une colonne.
    let resumes = match state
        .competitions
        .presence_survey_repository
        .list_summaries(&season_id)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("list_summaries: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    PresenceRoundsTemplate {
        app_routes: AppRoutes::default(),
        space_id,
        competition_id,
        season_id,
        rounds: PresenceRoundItemVm::all_from_domain(&resumes, &aujourd_hui()),
    }
    .into_response()
}

/// L'horloge est lue **ici**, au bord IO, et non dans un use case : la convention
/// de `send_due_notifications_use_case` — « `today` est une entrée, pas une
/// lecture d'horloge » — vise les use cases, qu'elle rend testables. Un rendu, lui,
/// doit bien la lire quelque part.
fn aujourd_hui() -> DateString {
    let brut = OffsetDateTime::now_utc()
        .date()
        .format(format_description!("[year]-[month]-[day]"))
        .unwrap_or_default();
    DateString::try_new(brut).unwrap_or_default()
}

// ── Le panneau : ce qu'un GET peut voir ──────────────────────────────────────

/// **Cinq états, pas six.** `Tirage` n'en est pas : l'aperçu ne persiste rien —
/// la réponse *est* le fragment, et un rechargement revient au sondage clos.
/// Rien dans `(campagne, journée, aujourd'hui)` ne peut donc dire qu'un tirage
/// vient d'être proposé, et le garder ici créerait une branche inatteignable dans
/// le `match` du GET : le genre de code que personne n'ose supprimer parce qu'il
/// a l'air prévu.
///
/// Le gabarit du tirage existe et son VM aussi ; c'est l'action `draw` de la
/// carte 521 qui le rend, par `rendre_tirage`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panneau {
    Aucun,
    EnCours,
    Clos,
    Appariee,
    Defection,
}

/// Le choix est une **fonction**, pas une suite de `if` dans le handler.
///
/// Elle se nourrit de trois questions déjà répondues par le domaine : `statut()`,
/// les appariements réels de la journée, et `desaccord(...)`. L'écrire dans le
/// handler l'aurait dupliquée entre le GET du panneau et les neuf actions qui
/// rendent un refus — et c'est la seconde copie qui aurait dérivé.
///
/// **L'ordre des branches est la règle** : une défection prime sur « appariée »,
/// qui prime sur le statut de la campagne. Un désaccord non traité est ce que
/// l'organisateur doit voir en premier ; l'annoncer « appariée » lui cacherait le
/// travail qui reste.
pub fn etat_du_panneau(
    survey: Option<&PresenceSurvey>,
    journee: &EtatJournee,
    aujourd_hui: &DateString,
) -> Panneau {
    let Some(survey) = survey else {
        return Panneau::Aucun;
    };
    if survey.desaccord(journee).is_some() {
        return Panneau::Defection;
    }
    if matches!(survey.appariement(), Appariement::Fait { .. }) {
        return Panneau::Appariee;
    }
    match survey.statut(aujourd_hui) {
        SurveyStatus::Ouverte => Panneau::EnCours,
        SurveyStatus::Close(_) => Panneau::Clos,
    }
}

// ── Les view models ──────────────────────────────────────────────────────────

/// L'en-tête de journée, commun aux six états.
pub struct RoundHeadVm {
    pub name: String,
    pub dates: String,
    pub badge: String,
}

impl RoundHeadVm {
    pub fn from_domain(round: &MatchDay) -> Self {
        Self {
            name: round.name.to_string(),
            dates: dates_de(round),
            badge: match round.day_type {
                MatchDayType::FixedDate => "Date fixe".to_string(),
                MatchDayType::TimeFrame => "Plage".to_string(),
                MatchDayType::Rest => "Repos".to_string(),
            },
        }
    }
}

/// **Rien n'est inventé** : une journée sans date rend une chaîne vide, et le
/// gabarit n'affiche alors pas la ligne. Un libellé de repli — « date à
/// définir » — se confondrait avec une date saisie.
fn dates_de(round: &MatchDay) -> String {
    let debut = round.date_start.as_ref().map(|d| d.to_string());
    let fin = round.date_end.as_ref().map(|d| d.to_string());
    match (round.day_type.clone(), debut, fin) {
        (MatchDayType::TimeFrame, Some(d), Some(f)) if d != f => format!("Du {d} au {f}"),
        (_, Some(d), _) => d,
        _ => String::new(),
    }
}

/// R3 — ce que l'écran annonce **avant** le lancement. `sans_adresse` n'est pas un
/// refus : un coach sans adresse entre dans la campagne comme les autres, seul
/// l'e-mail manque.
pub struct DestinatairesVm {
    pub equipes: usize,
    pub coachs: usize,
    pub sans_adresse: usize,
    pub joignables: usize,
}

impl DestinatairesVm {
    /// Les quatre nombres viennent des **méthodes** du roster, jamais d'un `len()`
    /// sur une liste reconstituée ici.
    pub fn from_domain(roster: &RosterDeCampagne) -> Self {
        let equipes = roster.equipes().len();
        let sans_adresse = roster.sans_adresse();
        Self {
            equipes,
            coachs: roster.coachs(),
            sans_adresse,
            joignables: equipes - sans_adresse,
        }
    }
}

/// La barre segmentée. **Quatre comptes reçus du domaine, aucun dérivé.**
///
/// Le réflexe serait de faire `rows.len()` sur chaque colonne : c'est exactement
/// le défaut de la carte 495, où la vue recomptait ce que le domaine savait
/// compter — et comptait autre chose, parce que la règle des journaliers ne
/// retient que les alignables.
pub struct AvancementVm {
    pub presents: usize,
    pub absents: usize,
    pub sans_reponse: usize,
    pub engagees: usize,
}

impl AvancementVm {
    pub fn from_domain(survey: &PresenceSurvey) -> Self {
        Self {
            presents: survey.compte_presents(),
            absents: survey.compte_absents(),
            sans_reponse: survey.compte_sans_reponse(),
            engagees: survey.engagees().len(),
        }
    }
}

/// Une ligne des trois colonnes.
pub struct AnswerRowVm {
    pub team_id: String,
    pub team_name: String,
    pub coach_label: String,
    pub initiales: String,
    pub repondu_le: String,
    /// R6 — le badge « saisi par vous ». Après le tirage, quand une rencontre est
    /// contestée, « qui a dit qu'il venait » a deux réponses possibles et elles
    /// n'engagent pas les mêmes personnes.
    pub saisi_par_admin: bool,
}

impl AnswerRowVm {
    pub fn all_from_domain(lignes: &[LignePresence]) -> Vec<Self> {
        lignes.iter().map(Self::from_domain).collect()
    }

    fn from_domain(l: &LignePresence) -> Self {
        Self {
            team_id: l.team_id.clone(),
            team_name: l.team_name.clone(),
            coach_label: l.coach_label.clone(),
            initiales: initiales_de(&l.team_name),
            repondu_le: l.repondu_le.clone().unwrap_or_default(),
            saisi_par_admin: l.saisi_par_admin,
        }
    }
}

/// Les mots que l'avatar saute — articles et prépositions.
///
/// Sans eux, « Les Crocs du Chaos » donnerait `LC` : c'est ce que fait
/// `initials_from` de `teams/domain/team.rs`, qui prend les deux premiers mots
/// tels quels. La maquette veut `CC`, et elle a raison — l'avatar doit désigner
/// l'équipe, pas sa grammaire.
const MOTS_SAUTES: [&str; 10] = ["le", "la", "les", "l", "de", "du", "des", "d", "et", "aux"];

/// Deux lettres tirées du nom d'équipe, mots-liens sautés.
///
/// **Les accents sont gardés** : « Étoiles de Naggaroth » donne `ÉN`. La maquette
/// y affiche `EN` mais `GÉ` ailleurs — une incohérence d'écriture à la main, et
/// rien ne justifie de retirer un accent que le nom porte.
///
/// Limite assumée : « FC Barcelone » donne `FB`. Une liste de mots français sur
/// une ligue francophone ; le cas se corrigera s'il se présente.
fn initiales_de(nom: &str) -> String {
    let lettres: String = nom
        .split(|c: char| c.is_whitespace() || c == '\'' || c == '’')
        .filter(|mot| !mot.is_empty())
        .filter(|mot| !MOTS_SAUTES.contains(&mot.to_lowercase().as_str()))
        .filter_map(|mot| mot.chars().next())
        .take(2)
        .collect();
    lettres.to_uppercase()
}

/// L'aperçu du tirage, ou la journée appariée.
pub struct DrawVm {
    pub rencontres: Vec<DrawRowVm>,
    /// Le **nom** de l'exemptée, pour l'écran.
    pub exemptee: Option<String>,
    /// Son identifiant, pour que la validation le renvoie. Deux champs et non un :
    /// afficher un identifiant est le défaut de la carte 506, et renvoyer un nom
    /// ne désignerait rien.
    pub exemptee_id: String,
    /// Comptés par le domaine, qui a produit les `Historique`. Les recompter en
    /// parcourant `rencontres` marcherait aujourd'hui et deviendrait faux le jour
    /// où une rencontre porterait un troisième motif.
    pub inedites: usize,
    pub revanches: usize,
    /// R18 — les équipes écartées du tirage, et l'écran le dit.
    pub ecartees: Vec<String>,
    /// R8 — faux quand le budget de nœuds a coupé la recherche. L'écran le
    /// signale plutôt que de laisser croire à un optimum.
    pub optimum_prouve: bool,
}

impl DrawVm {
    /// **Le roster nomme les équipes.** Sans lui, l'aperçu afficherait des
    /// identifiants de vingt-six caractères — le défaut de la carte 506, où « Affecté
    /// par » imprimait un ULID au lieu du nom du commissaire.
    pub fn from_domain(prop: &DrawProposal, roster: &RosterDeCampagne) -> Self {
        let rencontres = DrawRowVm::all_from_domain(&prop.rencontres, roster);
        Self {
            inedites: rencontres.iter().filter(|r| !r.est_revanche).count(),
            revanches: rencontres.iter().filter(|r| r.est_revanche).count(),
            rencontres,
            exemptee: prop.exemptee.map(|t| nom_de(&t, roster)),
            exemptee_id: prop.exemptee.map(|t| t.to_string()).unwrap_or_default(),
            ecartees: prop.ecartees.iter().map(|t| nom_de(t, roster)).collect(),
            optimum_prouve: prop.optimum_prouve,
        }
    }
}

/// Le nom d'une équipe, ou une mention explicite si le roster ne la connaît plus.
///
/// « Équipe désengagée » plutôt que l'identifiant : R18 l'a écartée de la saison,
/// et le dire est plus utile qu'un ULID que personne ne sait lire.
pub fn nom_de(team_id: &TeamId, roster: &RosterDeCampagne) -> String {
    roster
        .equipe(team_id)
        .map(|e| e.team_name.clone())
        .unwrap_or_else(|| "Équipe désengagée".to_string())
}

pub struct DrawRowVm {
    pub home: String,
    pub away: String,
    /// Les identifiants, que la validation renvoie au serveur — l'écran, lui,
    /// n'affiche que les noms.
    pub home_id: String,
    pub away_id: String,
    /// « 1re rencontre » ou « 2e rencontre · Journée 1 ». **Vient d'`Historique`**,
    /// jamais d'une relecture des journées par le VM : le tirage sait pourquoi il
    /// a concédé, et c'est lui qui le dit.
    pub tag: String,
    pub est_revanche: bool,
}

impl DrawRowVm {
    fn all_from_domain(rencontres: &[ProposedPairing], roster: &RosterDeCampagne) -> Vec<Self> {
        rencontres
            .iter()
            .map(|r| Self::from_domain(r, roster))
            .collect()
    }

    fn from_domain(r: &ProposedPairing, roster: &RosterDeCampagne) -> Self {
        let (tag, est_revanche) = match &r.historique {
            Historique::Inedite => ("1re rencontre".to_string(), false),
            Historique::Revanche { fois, derniere } => {
                (format!("{}e rencontre · {}", fois.0 + 1, derniere), true)
            }
        };
        Self {
            home: nom_de(&r.home, roster),
            away: nom_de(&r.away, roster),
            home_id: r.home.to_string(),
            away_id: r.away.to_string(),
            tag,
            est_revanche,
        }
    }
}

/// Les URL des neuf actions, résolues **une fois** au rendu du panneau.
///
/// Un porteur d'URL plutôt qu'`app_routes` et ses trois identifiants répétés dans
/// chaque `hx-post` : le gabarit écrit `{{ actions.answer }}`, et les fragments
/// inclus — la carte d'équipe, les rencontres — y ont accès sans recevoir les
/// identifiants de chemin qu'ils n'utiliseraient que pour ça.
pub struct ActionsVm {
    /// Le GET du panneau — c'est lui qui « abandonne » un aperçu : l'aperçu ne
    /// persistant rien, recharger le panneau suffit à revenir au sondage clos.
    pub panel: String,
    pub launch: String,
    pub answer: String,
    pub remind: String,
    pub close: String,
    pub reopen: String,
    pub draw: String,
    pub confirm_draw: String,
    pub undo_draw: String,
    pub propose_repair: String,
    pub repair: String,
}

impl ActionsVm {
    pub fn new(space_id: &str, competition_id: &str, season_id: &str) -> Self {
        let r = AppRoutes::default();
        let c = &r.competitions;
        Self {
            panel: c.admin_presences_panel(space_id, competition_id, season_id),
            launch: c.admin_presences_launch(space_id, competition_id, season_id),
            answer: c.admin_presences_answer(space_id, competition_id, season_id),
            remind: c.admin_presences_remind(space_id, competition_id, season_id),
            close: c.admin_presences_close(space_id, competition_id, season_id),
            reopen: c.admin_presences_reopen(space_id, competition_id, season_id),
            draw: c.admin_presences_draw(space_id, competition_id, season_id),
            confirm_draw: c.admin_presences_confirm_draw(space_id, competition_id, season_id),
            undo_draw: c.admin_presences_undo_draw(space_id, competition_id, season_id),
            propose_repair: c.admin_presences_propose_repair(space_id, competition_id, season_id),
            repair: c.admin_presences_repair(space_id, competition_id, season_id),
        }
    }
}

// ── Le conteneur du panneau ──────────────────────────────────────────────────

/// **Six structs et non un gabarit à branches.** Le `{% include %}` d'Askama
/// n'accepte qu'un chemin littéral : un fichier unique aurait voulu dire cinq
/// `{% if %}` imbriqués. `admin_page.rs` procède déjà par structs.

#[derive(Template)]
#[template(path = "admin/widgets/presences-panel-empty.html")]
pub struct PanelInviteTemplate {}

#[derive(Template)]
#[template(path = "admin/widgets/presences-panel-launch.html")]
pub struct PanelLaunchTemplate {
    pub head: RoundHeadVm,
    pub actions: ActionsVm,
    pub round_id: String,
    /// L'échéance **proposée**, pas imposée : la veille de la journée si elle a
    /// une date, sinon vide — et le gabarit laisse alors le champ à remplir plutôt
    /// que d'inventer un délai.
    pub deadline_defaut: String,
    /// Le motif d'un refus, vide sur un affichage ordinaire. Il vit dans le
    /// panneau et non dans une boîte du navigateur — cf. le fragment.
    pub motif: String,
    pub destinataires: DestinatairesVm,
}

#[derive(Template)]
#[template(path = "admin/widgets/presences-panel-running.html")]
pub struct PanelRunningTemplate {
    pub head: RoundHeadVm,
    pub actions: ActionsVm,
    pub round_id: String,
    /// Le motif d'un refus, vide sur un affichage ordinaire. Il vit dans le
    /// panneau et non dans une boîte du navigateur — cf. le fragment.
    pub motif: String,
    pub clos: bool,
    pub deadline: String,
    pub avancement: AvancementVm,
    pub presents: Vec<AnswerRowVm>,
    pub absents: Vec<AnswerRowVm>,
    pub sans_reponse: Vec<AnswerRowVm>,
    /// R15 — le motif accompagne le bouton inactif. Un bouton mort sans
    /// explication passerait pour une panne.
    pub peut_tirer: bool,
    pub motif_blocage: String,
}

#[derive(Template)]
#[template(path = "admin/widgets/presences-panel-draw.html")]
pub struct PanelDrawTemplate {
    pub head: RoundHeadVm,
    pub actions: ActionsVm,
    pub round_id: String,
    /// Le motif d'un refus, vide sur un affichage ordinaire. Il vit dans le
    /// panneau et non dans une boîte du navigateur — cf. le fragment.
    pub motif: String,
    pub draw: DrawVm,
    pub presents: usize,
}

#[derive(Template)]
#[template(path = "admin/widgets/presences-panel-paired.html")]
pub struct PanelPairedTemplate {
    pub head: RoundHeadVm,
    pub actions: ActionsVm,
    pub round_id: String,
    /// Le motif d'un refus, vide sur un affichage ordinaire. Il vit dans le
    /// panneau et non dans une boîte du navigateur — cf. le fragment.
    pub motif: String,
    pub draw: DrawVm,
}

#[derive(Template)]
#[template(path = "admin/widgets/presences-panel-defection.html")]
pub struct PanelDefectionTemplate {
    pub head: RoundHeadVm,
    pub actions: ActionsVm,
    pub round_id: String,
    /// Le motif d'un refus, vide sur un affichage ordinaire. Il vit dans le
    /// panneau et non dans une boîte du navigateur — cf. le fragment.
    pub motif: String,
    pub draw: DrawVm,
    pub defection: DefectionVm,
}

/// Ce que le désaccord dit, **sans nommer qui** au-delà des orphelins : R29 veut
/// que l'encart public dise combien, jamais qui — mais l'organisateur, lui, a
/// besoin des noms pour agir.
pub struct DefectionVm {
    pub rencontres_a_refaire: usize,
    pub orphelins: Vec<String>,
}

impl DefectionVm {
    fn from_domain(d: &Desaccord) -> Self {
        Self {
            rencontres_a_refaire: d.rencontres_a_refaire.len(),
            orphelins: d.orphelins.iter().map(|t| t.to_string()).collect(),
        }
    }
}

/// **Rendue ici, appelée par la carte 521.** Le gabarit du tirage appartient à
/// cette carte — c'est elle qui possède les six vues — mais l'aperçu ne persiste
/// rien, donc seul le `POST /presences/draw` peut le rendre.
pub fn rendre_tirage(
    actions: ActionsVm,
    round: &MatchDay,
    prop: &DrawProposal,
    roster: &RosterDeCampagne,
    presents: usize,
) -> Response {
    PanelDrawTemplate {
        head: RoundHeadVm::from_domain(round),
        actions,
        round_id: round.id.to_string(),
        motif: String::new(),
        draw: DrawVm::from_domain(prop, roster),
        presents,
    }
    .into_response()
}

/// La veille de la journée quand elle porte une date, **rien** sinon.
///
/// Rien plutôt qu'un délai inventé : « dans sept jours » se confondrait avec une
/// date choisie, et l'organisateur validerait sans regarder. Un champ vide se voit.
fn echeance_proposee(round: &MatchDay) -> String {
    round
        .date_start
        .as_ref()
        .and_then(|d| {
            time::Date::parse(d.as_ref(), format_description!("[year]-[month]-[day]")).ok()
        })
        .and_then(|jour| jour.previous_day())
        .and_then(|veille| {
            veille
                .format(format_description!("[year]-[month]-[day]"))
                .ok()
        })
        .unwrap_or_default()
}

fn html_ou_500(rendu: Result<String, askama::Error>, quoi: &str) -> Response {
    match rendu {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("{quoi} render: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

macro_rules! panneau_en_reponse {
    ($($t:ty => $quoi:literal),+ $(,)?) => {
        $(impl IntoResponse for $t {
            fn into_response(self) -> Response {
                html_ou_500(self.render(), $quoi)
            }
        })+
    };
}

panneau_en_reponse!(
    PanelInviteTemplate => "presences panel invite",
    PanelLaunchTemplate => "presences panel launch",
    PanelRunningTemplate => "presences panel running",
    PanelDrawTemplate => "presences panel draw",
    PanelPairedTemplate => "presences panel paired",
    PanelDefectionTemplate => "presences panel defection",
);

#[derive(Template)]
#[template(path = "admin/widgets/presences-panel-repair.html")]
pub struct PanelRepairTemplate {
    pub head: RoundHeadVm,
    pub actions: ActionsVm,
    pub round_id: String,
    pub motif: String,
    pub draw: DrawVm,
    pub a_defaire: Vec<String>,
    /// Les identifiants à défaire, déjà sérialisés : le gabarit les renvoie tels
    /// quels au POST de validation. Sérialisés ici plutôt que reconstruits en JS —
    /// ils ne sont pas dans le DOM, contrairement aux rencontres.
    pub a_defaire_json: String,
    pub vivier_vide: bool,
}

impl IntoResponse for PanelRepairTemplate {
    fn into_response(self) -> Response {
        html_ou_500(self.render(), "presences panel repair")
    }
}

/// **Rendue ici, appelée par `propose-repair`.** Même raison que `rendre_tirage` :
/// la proposition ne persiste rien, donc seul un POST peut la produire — le GET du
/// panneau la recalculerait, et `tirer` départageant au sort, elle changerait à
/// chaque affichage.
pub fn rendre_reparation(
    actions: ActionsVm,
    round: &MatchDay,
    prop: &RepairProposal,
    roster: &RosterDeCampagne,
) -> Response {
    let a_defaire: Vec<String> = prop.a_defaire.iter().map(|p| p.to_string()).collect();
    PanelRepairTemplate {
        head: RoundHeadVm::from_domain(round),
        actions,
        round_id: round.id.to_string(),
        motif: String::new(),
        draw: DrawVm::from_domain(&prop.proposition, roster),
        a_defaire_json: serde_json::to_string(&a_defaire).unwrap_or_else(|_| "[]".to_string()),
        a_defaire,
        vivier_vide: prop.vivier.is_empty(),
    }
    .into_response()
}

#[derive(Deserialize)]
pub struct PanelQuery {
    #[serde(default)]
    pub round_id: String,
}

pub async fn presences_panel_widget(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    Query(q): Query<PanelQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin_access(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &state,
    )
    .await
    {
        return resp;
    }

    // Aucune journée choisie : l'invite est un vrai état de l'écran, pas un
    // bouche-trou — c'est ce que voit l'organisateur au premier chargement.
    if q.round_id.is_empty() {
        return PanelInviteTemplate {}.into_response();
    }

    // `round_id` arrive par la chaîne de requête, et `space_scope` n'a pas de
    // résolveur pour lui : sans cette vérification, la journée d'une autre saison
    // répondrait (carte 416).
    let round = match journee_de_la_saison(&q.round_id, &season_id, &state).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    match charger_le_panneau(
        &space_id,
        &competition_id,
        &season_id,
        &round,
        &state,
        String::new(),
    )
    .await
    {
        Ok(resp) => resp,
        Err(resp) => resp,
    }
}

/// Charge les quatre faits dont les cinq états ont besoin, puis délègue le choix
/// à `etat_du_panneau` — jamais à une suite de `if` écrite ici.
/// `motif` est vide sur un affichage ordinaire, et porte le refus quand une action
/// en a rendu un : c'est ce qui donne **un seul vocabulaire** à tout ce que l'écran
/// répond. Le prix est quatre lectures pour dire « non » — payé sur un chemin rare.
pub async fn charger_le_panneau(
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    round: &MatchDay,
    state: &AppState,
    motif: String,
) -> Result<Response, Response> {
    let survey = state
        .competitions
        .presence_survey_repository
        .find_by_round(&round.id.to_string())
        .await
        .map_err(|e| {
            tracing::error!("find_by_round: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;

    let journee = etat_de_la_journee(round, state.competitions.match_report_status_port.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("etat_de_la_journee: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;

    let space = SpaceId::try_new(space_id).map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
    let roster = survey_roster_service::charger(
        season_id,
        &space,
        state.competitions.team_info_port.as_ref(),
        state.competitions.space_member_port.as_ref(),
    )
    .await
    .map_err(|e| {
        tracing::error!("survey_roster_service: {e}");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    let head = RoundHeadVm::from_domain(round);
    let actions = ActionsVm::new(space_id, competition_id, season_id);
    let round_id = round.id.to_string();
    let maintenant = aujourd_hui();
    Ok(
        match etat_du_panneau(survey.as_ref(), &journee, &maintenant) {
            Panneau::Aucun => PanelLaunchTemplate {
                head,
                actions,
                round_id,
                motif,
                deadline_defaut: echeance_proposee(round),
                destinataires: DestinatairesVm::from_domain(&roster),
            }
            .into_response(),
            Panneau::EnCours | Panneau::Clos => {
                // `survey` est `Some` dès que l'état n'est pas `Aucun` — c'est ce
                // que `etat_du_panneau` garantit, et le `else` ne peut pas arriver.
                let Some(s) = survey.as_ref() else {
                    return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
                };
                panneau_en_cours(head, actions, round_id, motif, s, &roster, &maintenant)
            }
            Panneau::Appariee => {
                let Some(s) = survey.as_ref() else {
                    return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
                };
                PanelPairedTemplate {
                    head,
                    actions,
                    round_id,
                    motif,
                    draw: journee_appariee(s, &journee, &roster),
                }
                .into_response()
            }
            Panneau::Defection => {
                let Some(s) = survey.as_ref() else {
                    return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
                };
                let Some(d) = s.desaccord(&journee) else {
                    return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
                };
                PanelDefectionTemplate {
                    head,
                    actions,
                    round_id,
                    motif,
                    draw: journee_appariee(s, &journee, &roster),
                    defection: DefectionVm::from_domain(&d),
                }
                .into_response()
            }
        },
    )
}

fn panneau_en_cours(
    head: RoundHeadVm,
    actions: ActionsVm,
    round_id: String,
    motif: String,
    survey: &PresenceSurvey,
    roster: &RosterDeCampagne,
    maintenant: &DateString,
) -> Response {
    let cols = survey_roster_service::colonnes(roster, survey);
    let clos = !survey.statut(maintenant).est_ouverte();
    let inscrites: std::collections::HashSet<TeamId> =
        roster.equipes().iter().map(|e| e.team_id).collect();

    // R15 — le motif vient du domaine, qui dit **combien** sont appariables. La
    // vue ne le recompte pas : elle imprime ce que le refus lui donne.
    let (peut_tirer, motif_blocage) = match survey.peut_tirer(&inscrites) {
        Ok(()) => (true, String::new()),
        Err(e) => (false, e.to_string()),
    };

    PanelRunningTemplate {
        head,
        actions,
        round_id,
        motif,
        clos,
        deadline: survey.deadline().to_string(),
        avancement: AvancementVm::from_domain(survey),
        presents: AnswerRowVm::all_from_domain(&cols.presents),
        absents: AnswerRowVm::all_from_domain(&cols.absents),
        sans_reponse: AnswerRowVm::all_from_domain(&cols.sans_reponse),
        peut_tirer,
        motif_blocage,
    }
    .into_response()
}

/// Les rencontres **réelles** de la journée, nommées par le roster.
///
/// R24 — elles se lisent sur la journée, pas sur la campagne : celle-ci ne garde
/// que l'exemptée. Et les noms viennent du roster, jamais d'un identifiant brut à
/// l'écran — le défaut de la carte 506.
fn journee_appariee(
    survey: &PresenceSurvey,
    journee: &EtatJournee,
    roster: &RosterDeCampagne,
) -> DrawVm {
    let nom = |t: &TeamId| {
        roster
            .equipe(t)
            .map(|e| e.team_name.clone())
            .unwrap_or_else(|| "Équipe désengagée".to_string())
    };
    let (exemptee, exemptee_id) = match survey.appariement() {
        Appariement::Fait {
            exemptee: Some(e), ..
        } => (Some(nom(e)), e.to_string()),
        _ => (None, String::new()),
    };
    DrawVm {
        rencontres: journee
            .rencontres
            .iter()
            .map(|r| DrawRowVm {
                home: nom(&r.home),
                away: nom(&r.away),
                home_id: r.home.to_string(),
                away_id: r.away.to_string(),
                // L'historique d'une rencontre déjà écrite n'est pas relu ici : ce
                // serait faire dériver au VM une valeur que le tirage avait
                // produite. Le panneau appariée ne l'affiche donc pas.
                tag: String::new(),
                est_revanche: false,
            })
            .collect(),
        exemptee,
        exemptee_id,
        inedites: 0,
        revanches: 0,
        ecartees: vec![],
        optimum_prouve: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{MatchDayName, MatchDayPosition};
    use crate::app::competitions::domain::presence_survey::{
        AutoRemind, Destinataire, RencontreJournee, Repondant, SurveyId, Venue,
    };
    use crate::app::competitions::domain::tirage::NombreDeRencontres;
    use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId, SeasonId};
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::CoachId;

    fn dto(
        deadline: Option<&str>,
        close_le: Option<&str>,
        appariee: Option<bool>,
    ) -> SurveySummaryDto {
        SurveySummaryDto {
            round_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            round_name: "Journée 3".to_string(),
            round_position: 2,
            is_rest: false,
            deadline: deadline.map(str::to_string),
            close_le: close_le.map(str::to_string),
            appariee,
            attendues: 14,
            reponses: 10,
            presents: 8,
        }
    }

    fn date(s: &str) -> DateString {
        DateString::try_new(s.to_string()).unwrap()
    }

    fn vm(dto: &SurveySummaryDto, aujourd_hui: &str) -> PresenceRoundItemVm {
        PresenceRoundItemVm::from_domain(dto, &date(aujourd_hui))
    }

    // ── Les initiales de l'avatar ────────────────────────────────────────────

    /// Les cas de la maquette. Sans la liste de mots sautés, « Les Crocs du
    /// Chaos » donnerait `LC` — ce que fait `initials_from` de `teams`.
    #[test]
    fn les_initiales_sautent_les_mots_liens() {
        assert_eq!(initiales_de("Les Crocs du Chaos"), "CC");
        assert_eq!(initiales_de("Bordeciel FC"), "BF");
        assert_eq!(initiales_de("Nains Rouges"), "NR");
        assert_eq!(initiales_de("Rats des Égouts"), "RÉ");
    }

    /// Les accents sont **gardés** : la maquette affiche `EN` pour « Étoiles de
    /// Naggaroth » et `GÉ` ailleurs — une incohérence d'écriture à la main, et
    /// rien ne justifie de retirer un accent que le nom porte.
    #[test]
    fn les_initiales_gardent_les_accents() {
        assert_eq!(initiales_de("Étoiles de Naggaroth"), "ÉN");
    }

    #[test]
    fn une_apostrophe_separe_les_mots() {
        assert_eq!(initiales_de("L'Ordre du Griffon"), "OG");
    }

    /// Un nom d'un seul mot ne rend qu'une lettre, et c'est mieux qu'une seconde
    /// inventée en piochant dans les lettres suivantes.
    #[test]
    fn un_nom_d_un_seul_mot_ne_rend_qu_une_lettre() {
        assert_eq!(initiales_de("Skavenblight"), "S");
    }

    /// Un nom entièrement fait de mots sautés ne rend rien plutôt qu'un caractère
    /// arbitraire — le gabarit affichera un avatar vide, ce qui est visible.
    #[test]
    fn un_nom_sans_mot_significatif_ne_rend_rien() {
        assert_eq!(initiales_de("Le Des"), "");
    }

    // ── L'état du panneau ────────────────────────────────────────────────────

    fn vierge() -> EtatJournee {
        EtatJournee {
            figee: false,
            rencontres: vec![],
        }
    }

    fn campagne(round: &MatchDay, dests: &[Destinataire]) -> PresenceSurvey {
        PresenceSurvey::ouvrir(
            SurveyId::new(),
            SeasonId::new(),
            round,
            dests,
            SurveyDeadline::try_new("2026-10-10".to_string()).unwrap(),
            AutoRemind::new(true),
            &date("2026-10-01"),
        )
        .unwrap()
    }

    fn journee_test() -> MatchDay {
        MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("Journée 3".to_string()).unwrap(),
            day_type: MatchDayType::TimeFrame,
            date_start: Some(DateString::try_new("2026-10-12".to_string()).unwrap()),
            date_end: Some(DateString::try_new("2026-10-19".to_string()).unwrap()),
            position: MatchDayPosition::try_new(2).unwrap(),
            pairings: vec![],
        }
    }

    fn dests(n: usize) -> Vec<Destinataire> {
        (0..n)
            .map(|_| Destinataire {
                team_id: TeamId::new(),
                coach_id: CoachId::new(),
            })
            .collect()
    }

    #[test]
    fn aucune_campagne_donne_l_etat_aucun() {
        assert_eq!(
            etat_du_panneau(None, &vierge(), &date("2026-10-05")),
            Panneau::Aucun
        );
    }

    #[test]
    fn une_campagne_ouverte_donne_en_cours_et_close_donne_clos() {
        let round = journee_test();
        let survey = campagne(&round, &dests(4));

        assert_eq!(
            etat_du_panneau(Some(&survey), &vierge(), &date("2026-10-05")),
            Panneau::EnCours
        );
        assert_eq!(
            etat_du_panneau(Some(&survey), &vierge(), &date("2026-10-11")),
            Panneau::Clos,
            "R23 — close le lendemain de l'échéance, sans que rien ne l'écrive"
        );
    }

    #[test]
    fn une_journee_appariee_donne_appariee() {
        let round = journee_test();
        let mut survey = campagne(&round, &dests(4));
        survey.marquer_appariee(None);

        assert_eq!(
            etat_du_panneau(Some(&survey), &vierge(), &date("2026-10-11")),
            Panneau::Appariee
        );
    }

    /// **L'ordre des branches est la règle** : un désaccord non traité prime sur
    /// « appariée ». L'annoncer appariée cacherait à l'organisateur le travail
    /// qui reste.
    #[test]
    fn une_defection_prime_sur_l_appariement() {
        let round = journee_test();
        let d = dests(2);
        let mut survey = campagne(&round, &d);
        for dest in &d {
            survey
                .enregistrer(
                    &dest.team_id,
                    Venue::Presente,
                    Repondant::Jeton,
                    &vierge(),
                    &date("2026-10-04"),
                )
                .unwrap();
        }
        survey.marquer_appariee(None);
        // La journée porte leur rencontre, et l'un se décommande.
        let journee = EtatJournee {
            figee: false,
            rencontres: vec![RencontreJournee {
                pairing: PairingId::new(),
                home: d[0].team_id,
                away: d[1].team_id,
            }],
        };
        survey
            .enregistrer(
                &d[1].team_id,
                Venue::Absente,
                Repondant::Organisateur(CoachId::new()),
                &vierge(),
                &date("2026-10-05"),
            )
            .unwrap();

        assert_eq!(
            etat_du_panneau(Some(&survey), &journee, &date("2026-10-05")),
            Panneau::Defection
        );
    }

    // ── Les VM ne dérivent rien ──────────────────────────────────────────────

    /// Le défaut de la carte 495 : la vue recomptait ce que le domaine savait
    /// compter. Les quatre nombres viennent des méthodes de l'agrégat.
    #[test]
    fn l_avancement_reçoit_ses_quatre_comptes_du_domaine() {
        let round = journee_test();
        let d = dests(5);
        let mut survey = campagne(&round, &d);
        survey
            .enregistrer(
                &d[0].team_id,
                Venue::Presente,
                Repondant::Jeton,
                &vierge(),
                &date("2026-10-04"),
            )
            .unwrap();
        survey
            .enregistrer(
                &d[1].team_id,
                Venue::Presente,
                Repondant::Jeton,
                &vierge(),
                &date("2026-10-04"),
            )
            .unwrap();
        survey
            .enregistrer(
                &d[2].team_id,
                Venue::Absente,
                Repondant::Jeton,
                &vierge(),
                &date("2026-10-04"),
            )
            .unwrap();

        let vm = AvancementVm::from_domain(&survey);

        assert_eq!(vm.presents, 2);
        assert_eq!(vm.absents, 1);
        assert_eq!(vm.sans_reponse, 2);
        assert_eq!(vm.engagees, 5);
    }

    #[test]
    fn l_en_tete_d_une_plage_affiche_ses_deux_dates() {
        let vm = RoundHeadVm::from_domain(&journee_test());

        assert_eq!(vm.name, "Journée 3");
        assert_eq!(vm.dates, "Du 2026-10-12 au 2026-10-19");
        assert_eq!(vm.badge, "Plage");
    }

    /// Rien n'est inventé : une journée sans date rend une chaîne vide, et le
    /// gabarit n'affiche alors pas la ligne. Un « date à définir » se confondrait
    /// avec une date saisie.
    #[test]
    fn une_journee_sans_date_n_invente_pas_de_libelle() {
        let mut round = journee_test();
        round.date_start = None;
        round.date_end = None;

        assert_eq!(RoundHeadVm::from_domain(&round).dates, "");
    }

    // ── L'étiquette d'une rencontre vient du domaine ─────────────────────────

    #[test]
    fn le_tag_d_une_rencontre_vient_de_son_historique() {
        let prop = DrawProposal {
            rencontres: vec![
                ProposedPairing {
                    home: TeamId::new(),
                    away: TeamId::new(),
                    historique: Historique::Inedite,
                },
                ProposedPairing {
                    home: TeamId::new(),
                    away: TeamId::new(),
                    historique: Historique::Revanche {
                        fois: NombreDeRencontres(1),
                        derniere: MatchDayName::try_new("Journée 1".to_string()).unwrap(),
                    },
                },
            ],
            exemptee: None,
            ..Default::default()
        };

        let vm = DrawVm::from_domain(&prop, &RosterDeCampagne::default());

        assert_eq!(vm.rencontres[0].tag, "1re rencontre");
        assert!(!vm.rencontres[0].est_revanche);
        assert_eq!(vm.rencontres[1].tag, "2e rencontre · Journée 1");
        assert!(vm.rencontres[1].est_revanche);
        assert_eq!(vm.inedites, 1);
        assert_eq!(vm.revanches, 1);
    }

    /// **Le défaut que la carte 521 a fait sortir.** `DrawVm::from_domain` mettait
    /// `r.home.to_string()` dans le champ affiché — l'identifiant de vingt-six
    /// caractères, pas le nom. C'est le défaut de la carte 506, et rien ne pouvait
    /// l'attraper en 520 : le panneau d'aperçu n'était pas atteignable, l'action
    /// `draw` n'existant pas encore.
    ///
    /// Deux champs séparés désormais : le nom pour l'écran, l'identifiant pour que
    /// la validation le renvoie.
    #[test]
    fn l_apercu_affiche_les_noms_et_renvoie_les_identifiants() {
        let (a, b) = (TeamId::new(), TeamId::new());
        let prop = DrawProposal {
            rencontres: vec![ProposedPairing {
                home: a,
                away: b,
                historique: Historique::Inedite,
            }],
            exemptee: None,
            ..Default::default()
        };

        // Roster vide : le repli est une mention explicite, jamais un identifiant.
        let vm = DrawVm::from_domain(&prop, &RosterDeCampagne::default());

        assert_eq!(vm.rencontres[0].home, "Équipe désengagée");
        assert_eq!(
            vm.rencontres[0].home_id,
            a.to_string(),
            "l'identifiant voyage, mais dans son propre champ"
        );
        assert_ne!(
            vm.rencontres[0].home,
            a.to_string(),
            "un ULID ne s'affiche jamais à l'écran"
        );
    }

    #[test]
    fn une_journee_sans_campagne_n_a_pas_de_sondage() {
        let v = vm(&dto(None, None, None), "2026-10-05");

        assert_eq!(v.etat, "aucun");
        assert_eq!(v.resume, "Aucun sondage");
    }

    #[test]
    fn une_campagne_ouverte_affiche_l_avancement() {
        let v = vm(&dto(Some("2026-10-10"), None, Some(false)), "2026-10-05");

        assert_eq!(v.etat, "en_cours");
        assert_eq!(v.resume, "10 réponses sur 14");
    }

    /// R23 — la clôture est **calculée**. Le jour de l'échéance on répond encore ;
    /// le lendemain la campagne est close, sans que rien ne l'ait écrite.
    #[test]
    fn une_campagne_echue_est_close_le_lendemain() {
        let d = dto(Some("2026-10-10"), None, Some(false));

        assert_eq!(vm(&d, "2026-10-10").etat, "en_cours");
        assert_eq!(vm(&d, "2026-10-11").etat, "clos");
        assert_eq!(vm(&d, "2026-10-11").resume, "8 présents sur 14");
    }

    #[test]
    fn une_cloture_decidee_prime_sur_l_echeance() {
        let v = vm(
            &dto(Some("2026-10-10"), Some("2026-10-03"), Some(false)),
            "2026-10-05",
        );

        assert_eq!(v.etat, "clos", "close par décision, échéance à venir");
    }

    /// L'appariement prime sur le statut : une journée appariée se lit comme telle,
    /// que sa campagne soit close par décision ou par échéance.
    #[test]
    fn une_journee_appariee_le_dit_plutot_que_son_statut() {
        let v = vm(&dto(Some("2026-10-10"), None, Some(true)), "2026-10-05");

        assert_eq!(v.etat, "apparie");
        assert_eq!(
            v.resume, "Journée appariée",
            "aucun nombre inventé : le DTO ne compte pas les appariements"
        );
    }

    /// R2 — il n'y a rien à sonder une journée de repos. Elle reste listée pour ne
    /// pas faire un trou dans le calendrier.
    #[test]
    fn une_journee_de_repos_est_signalee_comme_telle() {
        let mut d = dto(None, None, None);
        d.is_rest = true;

        let v = vm(&d, "2026-10-05");

        assert_eq!(v.etat, "repos");
        assert_eq!(v.resume, "Journée de repos");
        assert!(v.is_rest);
    }

    /// Une échéance illisible ne peut venir que d'une écriture hors du domaine.
    /// Elle est journalisée, et la journée reste affichée — l'escamoter ferait
    /// disparaître une campagne de la barre latérale sans une ligne de journal.
    #[test]
    fn une_echeance_illisible_ne_fait_pas_disparaitre_la_journee() {
        let v = vm(&dto(Some("pas une date"), None, Some(false)), "2026-10-05");

        assert_eq!(v.etat, "clos", "le repli de 1970 la met dans le passé");
        assert_eq!(v.name, "Journée 3");
    }

    #[test]
    fn la_liste_conserve_l_ordre_du_depot() {
        let mut premiere = dto(None, None, None);
        premiere.round_name = "Journée 1".to_string();
        let mut seconde = dto(Some("2026-10-10"), None, Some(false));
        seconde.round_name = "Journée 2".to_string();

        let vms = PresenceRoundItemVm::all_from_domain(&[premiere, seconde], &date("2026-10-05"));

        assert_eq!(vms.len(), 2);
        assert_eq!(vms[0].name, "Journée 1");
        assert_eq!(vms[1].name, "Journée 2");
    }
}
