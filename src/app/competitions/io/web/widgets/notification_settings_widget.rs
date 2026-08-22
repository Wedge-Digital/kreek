//! Le widget de réglage des quatre notifications, et ses deux endpoints.
//!
//! # Le piège du POST : une case décochée n'est pas envoyée
//!
//! `hx-post` poste un **formulaire**, et une case non cochée n'apparaît pas
//! dans le corps. D'où `Form` et non `Json`, et `#[serde(default)]` sur les
//! quatre champs. Le symptôme d'une erreur ici serait trompeur : on pourrait
//! activer une notification et jamais la désactiver, ce qui ressemble à un
//! défaut de persistance alors que c'est le corps qui est incomplet.
//!
//! # Pourquoi `204` et pas un fragment
//!
//! Re-rendre le widget après chaque clic ferait clignoter les cases et perdrait
//! le focus clavier, pour réafficher exactement ce qui est déjà à l'écran.

use crate::app::competitions::domain::competition_notifications::{
    applicability, CompetitionNotifications, Inapplicable, NotificationApplicability,
    NotifyRegistrationDeadline, NotifyRegistrationOpen, NotifyRoundClosing, NotifyRoundEve,
};
use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::app::competitions::use_cases::save_competition_notifications::{
    self, SaveCompetitionNotificationsCommand,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Form, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

// ── Vue ──────────────────────────────────────────────────────────────────────

pub struct NotificationRowVm {
    /// Le nom du champ dans l'évènement DOM et dans le payload — « round_eve ».
    /// C'est aussi la clé sur laquelle Alpine reconnaît la ligne à griser.
    pub key: &'static str,
    pub label: String,
    pub description: String,
    pub when: String,
    pub checked: bool,
    /// `None` = applicable.
    ///
    /// **Pour la ligne `registration_deadline`, ce n'est qu'un état de départ**,
    /// écrasé par Alpine dès la première frappe dans le champ de date de
    /// l'étape 4. Jamais une vérité côté serveur.
    pub inapplicable_reason: Option<String>,
}

#[derive(Template)]
#[template(path = "widgets/notification-settings-widget.html")]
pub struct NotificationSettingsVm {
    pub rows: Vec<NotificationRowVm>,
    /// « deferred » | « autosave » — pilote la pose des attributs HTMX.
    pub mode: &'static str,
    /// Vide en mode différé, où le widget ne POSTe pas.
    pub post_url: String,
    /// Le motif qu'Alpine affiche quand l'utilisateur efface la date limite.
    /// Présent **même quand la ligne démarre applicable** : sinon le client
    /// n'aurait rien à afficher au moment où il en a besoin.
    pub deadline_cleared_reason: String,
}

fn motif(raison: Inapplicable) -> String {
    match raison {
        Inapplicable::NoSchedule => "Cette compétition n'a pas de calendrier".to_string(),
        Inapplicable::NoTimeFrameRound => "Aucune journée n'a de fenêtre à clore".to_string(),
        Inapplicable::NoDeadline => "Aucune date limite d'inscription n'est fixée".to_string(),
    }
}

impl NotificationSettingsVm {
    pub fn from_domain(
        notifications: &CompetitionNotifications,
        applicabilite: &NotificationApplicability,
        mode: &'static str,
        post_url: String,
    ) -> Self {
        Self {
            rows: vec![
                Self::ouverture(notifications),
                Self::veille(notifications, applicabilite),
                Self::cloture(notifications, applicabilite),
                Self::date_limite(notifications, applicabilite),
            ],
            mode,
            post_url,
            deadline_cleared_reason: motif(Inapplicable::NoDeadline),
        }
    }

    fn ouverture(n: &CompetitionNotifications) -> NotificationRowVm {
        NotificationRowVm {
            key: "registration_open",
            label: "Ouverture des inscriptions".to_string(),
            description: "Prévenir les coachs que la compétition accepte les inscriptions."
                .to_string(),
            when: "à l'ouverture des inscriptions".to_string(),
            checked: n.registration_open.0,
            // Toujours applicable : une compétition a par construction une
            // ouverture.
            inapplicable_reason: None,
        }
    }

    fn veille(n: &CompetitionNotifications, a: &NotificationApplicability) -> NotificationRowVm {
        NotificationRowVm {
            key: "round_eve",
            label: "Veille de journée".to_string(),
            description: "Rappeler à chaque coach les matchs qui l'attendent.".to_string(),
            when: "la veille du début de chaque journée".to_string(),
            checked: n.round_eve.0,
            inapplicable_reason: a.round_eve.map(motif),
        }
    }

    fn cloture(n: &CompetitionNotifications, a: &NotificationApplicability) -> NotificationRowVm {
        NotificationRowVm {
            key: "round_closing",
            label: "Fin de journée imminente".to_string(),
            description: "Alerter les coachs dont le match n'est pas encore saisi.".to_string(),
            when: "la veille de la clôture de chaque journée".to_string(),
            checked: n.round_closing.0,
            inapplicable_reason: a.round_closing.map(motif),
        }
    }

    fn date_limite(
        n: &CompetitionNotifications,
        a: &NotificationApplicability,
    ) -> NotificationRowVm {
        NotificationRowVm {
            key: "registration_deadline",
            label: "Date limite d'inscription".to_string(),
            description: "Prévenir de l'approche de la fermeture des inscriptions.".to_string(),
            when: "à l'approche de la date limite".to_string(),
            // R6 : cochée puis rendue inapplicable, elle **reste cochée**.
            // `checked` ne consulte donc jamais l'applicabilité.
            checked: n.registration_deadline.0,
            inapplicable_reason: a.registration_deadline.map(motif),
        }
    }
}

impl IntoResponse for NotificationSettingsVm {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("notification_settings_widget render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Entrées ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WidgetModeQuery {
    pub mode: String,
}

/// Une case cochée envoie **`on`**, pas `true` : c'est la valeur par défaut
/// d'un `<input type="checkbox">` sans attribut `value`, et `serde` ne sait pas
/// en tirer un `bool`. Sa seule présence vaut donc vrai, quelle que soit la
/// valeur ; son absence est traitée par `#[serde(default)]`.
///
/// Posé ici plutôt que résolu par un `value="true"` dans le gabarit : le DTO de
/// transport doit lire ce qu'un formulaire HTML envoie réellement, sans
/// dépendre d'un attribut qu'un remaniement du template pourrait retirer.
fn case_cochee<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    use serde::Deserialize as _;
    let _ = String::deserialize(d)?;
    Ok(true)
}

#[derive(Debug, Deserialize)]
pub struct NotificationSettingsPayload {
    #[serde(default, deserialize_with = "case_cochee")]
    pub registration_open: bool,
    #[serde(default, deserialize_with = "case_cochee")]
    pub round_eve: bool,
    #[serde(default, deserialize_with = "case_cochee")]
    pub round_closing: bool,
    #[serde(default, deserialize_with = "case_cochee")]
    pub registration_deadline: bool,
}

/// Un mode inconnu vaut `400` et non un repli silencieux sur `deferred` : mal
/// orthographié, il rendrait un widget muet, sans rien pour le signaler.
fn mode_connu(brut: &str) -> Option<&'static str> {
    match brut {
        "deferred" => Some("deferred"),
        "autosave" => Some("autosave"),
        _ => None,
    }
}

// ── Handlers ─────────────────────────────────────────────────────────────────

pub async fn get_notification_settings_widget(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    Query(q): Query<WidgetModeQuery>,
    State(state): State<AppState>,
) -> Response {
    let (Some(mode), Ok(sid)) = (mode_connu(&q.mode), SeasonId::try_new(&season_id)) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let repo = state.competitions.season_repository.as_ref();
    let Some(vm) = charger(repo, &sid, mode, &space_id, &competition_id, &season_id).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    vm.into_response()
}

/// Trois lectures : les réglages, la structure et les invitations. Les deux
/// dernières ne servent qu'à `applicability()` — les motifs de journée viennent
/// de la structure, celui de la date limite des invitations.
async fn charger(
    repo: &dyn ISeasonRepository,
    sid: &SeasonId,
    mode: &'static str,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
) -> Option<NotificationSettingsVm> {
    let structure = repo.find_structure(sid).await.ok()??;
    let invitations = repo.find_invitations(sid).await.ok()?;
    // `None` = colonne jamais écrite : le défaut du domaine s'applique.
    let reglages = repo.find_notifications(sid).await.ok()?.unwrap_or_default();

    let post_url = match mode {
        "autosave" => AppRoutes::default().competitions.notification_settings(
            space_id,
            competition_id,
            season_id,
        ),
        _ => String::new(),
    };
    let a = applicability(&structure, invitations.as_ref());
    Some(NotificationSettingsVm::from_domain(
        &reglages, &a, mode, post_url,
    ))
}

pub async fn post_notification_settings(
    Path((_space_id, _competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Form(payload): Form<NotificationSettingsPayload>,
) -> Response {
    let Ok(sid) = SeasonId::try_new(&season_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let cmd = SaveCompetitionNotificationsCommand {
        season_id: sid,
        notifications: CompetitionNotifications {
            registration_open: NotifyRegistrationOpen(payload.registration_open),
            round_eve: NotifyRoundEve(payload.round_eve),
            round_closing: NotifyRoundClosing(payload.round_closing),
            registration_deadline: NotifyRegistrationDeadline(payload.registration_deadline),
        },
    };

    match save_competition_notifications::execute(
        cmd,
        state.competitions.season_repository.as_ref(),
    )
    .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::error!("save notifications: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tout_coche() -> CompetitionNotifications {
        CompetitionNotifications::default()
    }

    fn rien_d_applicable() -> NotificationApplicability {
        NotificationApplicability {
            round_eve: Some(Inapplicable::NoSchedule),
            round_closing: Some(Inapplicable::NoSchedule),
            registration_deadline: Some(Inapplicable::NoDeadline),
        }
    }

    fn tout_applicable() -> NotificationApplicability {
        NotificationApplicability {
            round_eve: None,
            round_closing: None,
            registration_deadline: None,
        }
    }

    fn ligne<'a>(vm: &'a NotificationSettingsVm, cle: &str) -> &'a NotificationRowVm {
        vm.rows
            .iter()
            .find(|r| r.key == cle)
            .expect("ligne absente")
    }

    /// **Le test qui garde R6.** Sans lui, une future « simplification »
    /// décochant les lignes grisées passerait sans que rien ne proteste — et
    /// détruirait un choix explicite de l'organisateur en réaction à un geste
    /// qui n'a rien à voir.
    #[test]
    fn une_notification_cochee_puis_rendue_inapplicable_reste_cochee() {
        let vm = NotificationSettingsVm::from_domain(
            &tout_coche(),
            &rien_d_applicable(),
            "autosave",
            "/post".to_string(),
        );

        for cle in ["round_eve", "round_closing", "registration_deadline"] {
            let l = ligne(&vm, cle);
            assert!(l.checked, "{cle} devait rester cochée");
            assert!(
                l.inapplicable_reason.is_some(),
                "{cle} devait porter son motif"
            );
        }
    }

    /// `registration_open` est toujours applicable : une compétition a par
    /// construction une ouverture, et elle n'a donc pas de motif à afficher même
    /// quand tout le reste est grisé.
    #[test]
    fn l_ouverture_des_inscriptions_n_est_jamais_grisee() {
        let vm = NotificationSettingsVm::from_domain(
            &tout_coche(),
            &rien_d_applicable(),
            "autosave",
            "/post".to_string(),
        );

        assert_eq!(ligne(&vm, "registration_open").inapplicable_reason, None);
    }

    /// Le seul champ que le serveur envoie pour un cas qui n'existe pas encore
    /// à l'instant du rendu : sans lui, le client n'aurait rien à afficher au
    /// moment où l'utilisateur efface la date limite.
    #[test]
    fn le_motif_de_date_effacee_est_envoye_meme_quand_la_ligne_demarre_applicable() {
        let vm = NotificationSettingsVm::from_domain(
            &tout_coche(),
            &tout_applicable(),
            "autosave",
            "/post".to_string(),
        );

        assert_eq!(
            ligne(&vm, "registration_deadline").inapplicable_reason,
            None
        );
        assert!(!vm.deadline_cleared_reason.is_empty());
    }

    /// Le mode différé ne POSTe pas : lui donner une URL laisserait croire le
    /// contraire au template, qui pose ses attributs HTMX d'après elle.
    #[test]
    fn un_mode_inconnu_est_refuse_et_les_deux_connus_acceptes() {
        assert_eq!(mode_connu("autosave"), Some("autosave"));
        assert_eq!(mode_connu("deferred"), Some("deferred"));
        assert_eq!(mode_connu("defered"), None);
        assert_eq!(mode_connu(""), None);
    }
}
