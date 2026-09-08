//! La campagne de présence — l'agrégat que les trois unités du sondage
//! appellent.
//!
//! # Conçu d'un bloc, livré en trois cartes
//!
//! Sa forme couvre dès maintenant les trois chemins de réponse — l'organisateur
//! depuis l'onglet, le coach depuis son jeton, le coach connecté depuis
//! l'encart — et non le seul écran d'administration. Le workflow le demande :
//! *la troisième méthode qu'on greffe révèle souvent que les deux premières
//! avaient la mauvaise signature*.
//!
//! Cette carte pose les types, la construction et les lectures. Les commandes
//! arrivent en 512 (`enregistrer`, `desaccord`) et 513 (`valider_proposition`,
//! `clore`, `rouvrir`, `marquer_appariee`).
//!
//! # Aucun champ n'est `pub`
//!
//! Le seul chemin d'écriture d'une `Presence` sera `enregistrer`, qui portera
//! R19, R13, R21 et R28 ensemble. Un `pub` sur `reponses` les rendrait
//! contournables par un `survey.reponses[0].presence = …` que ni le
//! compilateur, ni `check-arch`, ni la revue ne signaleraient.

use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::match_day::MatchDay;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{CoachId, EntityId};
use crate::app::shared_kernel::identity::sulid::SUlid;
use nutype::nutype;

// ── Les value objects ────────────────────────────────────────────────────────

pub type SurveyId = EntityId;

/// Le jeton d'un lien de réponse.
///
/// **Ce n'est pas un `EntityId`**, bien qu'il en ait la forme. Un identifiant
/// technique et un secret d'URL n'ont ni le même cycle de vie ni les mêmes
/// règles de divulgation ; les confondre autoriserait à écrire `survey.id` là où
/// le jeton est attendu, et le compilateur ne dirait rien.
///
/// Il n'a **aucune durée de vie propre** (R7) : sa validité se lit sur l'état de
/// la campagne. Rouvrir un sondage réarme donc les anciens liens sans rien
/// réémettre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurveyToken(SUlid);

impl SurveyToken {
    pub fn new() -> Self {
        Self(SUlid::new())
    }

    pub fn try_new(s: &str) -> Result<Self, DomainError> {
        SUlid::try_new(s)
            .map(Self)
            .map_err(|_| DomainError::InvalidSurveyToken)
    }
}

impl Default for SurveyToken {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SurveyToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// L'échéance de la campagne, à la journée près.
///
/// Une **date** et non un instant, comme tout le reste du BC : une campagne
/// échue le 10 octobre l'est à partir du 11. Un horodatage donnerait une
/// fermeture à la minute, que ni l'e-mail ni l'écran n'annoncent.
#[nutype(
    sanitize(trim),
    validate(regex = r"^\d{4}-\d{2}-\d{2}$"),
    derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Display, AsRef)
)]
pub struct SurveyDeadline(String);

#[nutype(derive(Debug, Clone, Copy, PartialEq, Eq))]
pub struct AutoRemind(bool);

#[nutype(
    sanitize(trim),
    validate(regex = r"^\d{4}-\d{2}-\d{2}$"),
    derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Display, AsRef)
)]
pub struct OpenedAt(String);

#[nutype(
    sanitize(trim),
    validate(regex = r"^\d{4}-\d{2}-\d{2}$"),
    derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Display, AsRef)
)]
pub struct ReponduLe(String);

#[nutype(
    sanitize(trim),
    validate(regex = r"^\d{4}-\d{2}-\d{2}$"),
    derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Display, AsRef)
)]
pub struct FermeeLe(String);

/// Ce que le coach a répondu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Venue {
    Presente,
    Absente,
}

/// **Qui** répond, et surtout **ce qui l'autorise** — R28.
///
/// Trois chemins, trois autorisations : le jeton vaut par lui-même (R7), la
/// session doit être confrontée au propriétaire de l'équipe, et l'organisateur
/// passe par `require_admin_access`. Les fondre en deux variantes rendait
/// contournable la vérification du propriétaire, ce que seule la troisième
/// unité a fait voir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repondant {
    Jeton,
    Coach(CoachId),
    Organisateur(CoachId),
}

/// Trois états, dont deux seulement portent un horodatage et un auteur.
///
/// **Jamais un booléen nullable.** Un enum plat avec `repondu_le: Option<…>` et
/// `saisi_par: Option<…>` à côté remplacerait un `Option` par deux, et laisserait
/// construire une réponse déclarée sans horodatage — ou un horodatage sans
/// réponse. C'est le même défaut, réparti.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Presence {
    SansReponse,
    Declaree {
        venue: Venue,
        le: ReponduLe,
        par: Repondant,
    },
}

impl Presence {
    /// R5 — seule une présence déclarée compte pour le tirage.
    pub fn compte_pour_le_tirage(&self) -> bool {
        matches!(
            self,
            Presence::Declaree {
                venue: Venue::Presente,
                ..
            }
        )
    }

    pub fn est_absente(&self) -> bool {
        matches!(
            self,
            Presence::Declaree {
                venue: Venue::Absente,
                ..
            }
        )
    }

    pub fn est_sans_reponse(&self) -> bool {
        matches!(self, Presence::SansReponse)
    }
}

/// La clôture décidée, ou son absence.
///
/// Un enum et non un `Option<FermeeLe>` : croisé à l'échéance, il produit les
/// trois situations de R23, qu'un `Option` aurait laissé reconstituer à chaque
/// appelant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fermeture {
    Aucune,
    Decidee { le: FermeeLe },
}

/// Ce que la campagne retient du tirage — **l'exemption, jamais les
/// rencontres** (R24).
///
/// Les appariements appartiennent à la journée, qui les possède en base. Les
/// recopier ici créait une seconde vérité qu'aucune transaction ne tenait avec
/// la première : quatre chemins du Calendrier les suppriment sans rien savoir
/// d'une campagne. L'exemptée reste, parce qu'elle n'existe nulle part ailleurs
/// et que R9 a besoin de l'historique des exemptions de la saison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Appariement {
    Aucun,
    Fait { exemptee: Option<TeamId> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Motif {
    Echeance,
    Decision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurveyStatus {
    Ouverte,
    Close(Motif),
}

impl SurveyStatus {
    pub fn est_ouverte(&self) -> bool {
        matches!(self, SurveyStatus::Ouverte)
    }
}

/// R23 — la clôture est **calculée**, jamais stockée.
///
/// | `fermeture` | échéance | statut |
/// |---|---|---|
/// | `Aucune` | à venir | `Ouverte` |
/// | `Aucune` | passée | `Close(Echeance)` |
/// | `Decidee` | quelconque | `Close(Decision)` |
///
/// **Fonction libre, et pas seulement une méthode** : la barre latérale de
/// l'onglet lit des DTO de la base, pas des agrégats, et doit répondre à la même
/// question. Une règle calculée à deux endroits finit par l'être de deux façons,
/// et celle-ci croise deux champs et une horloge.
///
/// Écarté : une tâche planifiée qui clorait les campagnes échues. Une campagne
/// serait restée ouverte jusqu'à vingt-quatre heures après son échéance, ses
/// liens répondant pendant ce temps, contre ce que l'e-mail annonce noir sur
/// blanc. Le calcul n'a pas de retard possible.
pub fn statut_de(
    fermeture: &Fermeture,
    deadline: &SurveyDeadline,
    aujourd_hui: &DateString,
) -> SurveyStatus {
    if let Fermeture::Decidee { .. } = fermeture {
        return SurveyStatus::Close(Motif::Decision);
    }
    if aujourd_hui.as_ref() > deadline.as_ref() {
        return SurveyStatus::Close(Motif::Echeance);
    }
    SurveyStatus::Ouverte
}

// ── La réponse d'une équipe ──────────────────────────────────────────────────

/// Une ligne de la campagne : **une équipe engagée**, jamais un coach (R1).
///
/// Un coach qui engage deux équipes répond deux fois, et peut venir avec l'une
/// sans l'autre. `coach_id` est là pour R28 — savoir à qui l'équipe appartient —
/// et pour le libellé, jamais pour regrouper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reponse {
    team_id: TeamId,
    coach_id: CoachId,
    token: SurveyToken,
    presence: Presence,
}

impl Reponse {
    pub fn team_id(&self) -> &TeamId {
        &self.team_id
    }
    pub fn coach_id(&self) -> &CoachId {
        &self.coach_id
    }
    pub fn token(&self) -> &SurveyToken {
        &self.token
    }
    pub fn presence(&self) -> &Presence {
        &self.presence
    }
}

/// Ce que `ouvrir` reçoit pour chaque équipe à solliciter.
///
/// R3 — un coach sans adresse connue **n'empêche pas le lancement** : son équipe
/// entre dans la campagne comme les autres, seul l'e-mail manque. Refuser le
/// lancement ferait dépendre une campagne de quatorze coachs de la fiche
/// incomplète d'un seul.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destinataire {
    pub team_id: TeamId,
    pub coach_id: CoachId,
}

// ── L'agrégat ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceSurvey {
    id: SurveyId,
    season_id: SeasonId,
    round_id: MatchId,
    deadline: SurveyDeadline,
    auto_remind: AutoRemind,
    opened_at: OpenedAt,
    fermeture: Fermeture,
    reponses: Vec<Reponse>,
    appariement: Appariement,
}

impl PresenceSurvey {
    /// Ouvre une campagne sur une journée.
    ///
    /// Reçoit **`&MatchDay`** et non un identifiant : « peut-on sonder cette
    /// journée ? » est une question métier, et la répondre dans le use case
    /// l'aurait sortie du domaine.
    ///
    /// Engendre une `Reponse` par destinataire (R1), avec son jeton et
    /// `Presence::SansReponse`.
    pub fn ouvrir(
        id: SurveyId,
        season_id: SeasonId,
        round: &MatchDay,
        destinataires: &[Destinataire],
        deadline: SurveyDeadline,
        auto_remind: AutoRemind,
        aujourd_hui: &DateString,
    ) -> Result<Self, DomainError> {
        // R2 — il n'y a rien à apparier une journée de repos. La même garde que
        // `generate_pairings`, pour la même raison.
        if round.is_rest() {
            return Err(DomainError::SurveyOnRestDay);
        }

        let opened_at =
            OpenedAt::try_new(aujourd_hui.to_string()).map_err(|_| DomainError::InvalidOpenedAt)?;

        Ok(Self {
            id,
            season_id,
            round_id: round.id,
            deadline,
            auto_remind,
            opened_at,
            fermeture: Fermeture::Aucune,
            reponses: destinataires.iter().map(Reponse::attendue).collect(),
            appariement: Appariement::Aucun,
        })
    }

    /// Reconstruit une campagne depuis la base. **Réservé au dépôt** : elle ne
    /// vérifie aucun invariant, puisqu'ils ont été vérifiés à l'écriture.
    #[allow(clippy::too_many_arguments)]
    pub fn rehydrater(
        id: SurveyId,
        season_id: SeasonId,
        round_id: MatchId,
        deadline: SurveyDeadline,
        auto_remind: AutoRemind,
        opened_at: OpenedAt,
        fermeture: Fermeture,
        reponses: Vec<Reponse>,
        appariement: Appariement,
    ) -> Self {
        Self {
            id,
            season_id,
            round_id,
            deadline,
            auto_remind,
            opened_at,
            fermeture,
            reponses,
            appariement,
        }
    }

    // ── Ce que la campagne est ───────────────────────────────────────────────

    pub fn id(&self) -> &SurveyId {
        &self.id
    }
    pub fn season_id(&self) -> &SeasonId {
        &self.season_id
    }
    pub fn round_id(&self) -> &MatchId {
        &self.round_id
    }
    pub fn deadline(&self) -> &SurveyDeadline {
        &self.deadline
    }
    pub fn auto_remind(&self) -> AutoRemind {
        self.auto_remind
    }
    pub fn opened_at(&self) -> &OpenedAt {
        &self.opened_at
    }
    pub fn fermeture(&self) -> &Fermeture {
        &self.fermeture
    }
    pub fn appariement(&self) -> &Appariement {
        &self.appariement
    }

    // ── Ce que la campagne sait dire ─────────────────────────────────────────

    pub fn statut(&self, aujourd_hui: &DateString) -> SurveyStatus {
        statut_de(&self.fermeture, &self.deadline, aujourd_hui)
    }

    /// Les équipes qui ont confirmé leur venue — **les seules que le tirage
    /// retient** (R5).
    pub fn presents(&self) -> Vec<&Reponse> {
        self.filtrer(|p| p.compte_pour_le_tirage())
    }

    /// R5 — les silencieux ne sont pas mélangés aux refus : ce sont les seuls
    /// que l'organisateur peut encore convertir, par une relance ou un coup de
    /// fil. Les confondre lui ferait perdre la seule liste sur laquelle il a
    /// prise.
    pub fn sans_reponse(&self) -> Vec<&Reponse> {
        self.filtrer(Presence::est_sans_reponse)
    }

    /// Les quatre comptes existent pour que `AvancementVm` **ne dérive rien**.
    /// Le réflexe serait de faire `rows.len()` sur chaque colonne : c'est le
    /// défaut de la carte 495, où la vue recomptait ce que le domaine savait
    /// compter, et comptait autre chose.
    pub fn compte_presents(&self) -> usize {
        self.presents().len()
    }

    pub fn compte_absents(&self) -> usize {
        self.filtrer(Presence::est_absente).len()
    }

    pub fn compte_sans_reponse(&self) -> usize {
        self.sans_reponse().len()
    }

    /// Les équipes de la campagne — celles sollicitées à l'ouverture.
    pub fn engagees(&self) -> Vec<&TeamId> {
        self.reponses.iter().map(Reponse::team_id).collect()
    }

    /// Le chemin du coach venu de son e-mail (unité 2) et de l'encart (unité 3).
    pub fn reponse_par_jeton(&self, token: &SurveyToken) -> Option<&Reponse> {
        self.reponses.iter().find(|r| &r.token == token)
    }

    pub fn reponse_de(&self, team_id: &TeamId) -> Option<&Reponse> {
        self.reponses.iter().find(|r| &r.team_id == team_id)
    }

    /// R15 — le tirage refuse en dessous de deux présents, **et le dit**.
    ///
    /// Sans cette garde, l'aperçu s'afficherait vide et l'organisateur croirait à
    /// une panne. Même motif que `skipped_group_names` dans `generate_pairings` :
    /// signaler explicitement plutôt que laisser croire à un échec.
    pub fn peut_tirer(&self) -> Result<(), DomainError> {
        let presents = self.compte_presents();
        if presents < 2 {
            return Err(DomainError::NotEnoughPresent { presents });
        }
        Ok(())
    }

    fn filtrer(&self, predicat: impl Fn(&Presence) -> bool) -> Vec<&Reponse> {
        self.reponses
            .iter()
            .filter(|r| predicat(&r.presence))
            .collect()
    }
}

impl Reponse {
    fn attendue(d: &Destinataire) -> Self {
        Self {
            team_id: d.team_id,
            coach_id: d.coach_id,
            token: SurveyToken::new(),
            presence: Presence::SansReponse,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{
        MatchDayName, MatchDayPosition, MatchDayType,
    };

    fn journee(day_type: MatchDayType) -> MatchDay {
        MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("Journée 3".to_string()).unwrap(),
            day_type,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(2).unwrap(),
            pairings: vec![],
        }
    }

    fn date(s: &str) -> DateString {
        DateString::try_new(s.to_string()).unwrap()
    }

    fn echeance(s: &str) -> SurveyDeadline {
        SurveyDeadline::try_new(s.to_string()).unwrap()
    }

    fn destinataires(n: usize) -> Vec<Destinataire> {
        (0..n)
            .map(|_| Destinataire {
                team_id: TeamId::new(),
                coach_id: CoachId::new(),
            })
            .collect()
    }

    fn ouvrir(destinataires: &[Destinataire]) -> PresenceSurvey {
        PresenceSurvey::ouvrir(
            SurveyId::new(),
            SeasonId::new(),
            &journee(MatchDayType::FixedDate),
            destinataires,
            echeance("2026-10-10"),
            AutoRemind::new(true),
            &date("2026-10-01"),
        )
        .expect("ouverture")
    }

    /// Pose une réponse sans passer par `enregistrer`, qui n'existe pas encore
    /// (carte 512). Réservé aux tests de lecture de cette carte-ci.
    fn declarer(survey: &mut PresenceSurvey, rang: usize, venue: Venue) {
        survey.reponses[rang].presence = Presence::Declaree {
            venue,
            le: ReponduLe::try_new("2026-10-05".to_string()).unwrap(),
            par: Repondant::Jeton,
        };
    }

    // ── R1 — la réponse porte sur l'équipe, jamais sur le coach ──────────────

    #[test]
    fn un_coach_a_deux_equipes_donne_deux_reponses_et_deux_jetons() {
        let coach = CoachId::new();
        let ses_deux_equipes = vec![
            Destinataire {
                team_id: TeamId::new(),
                coach_id: coach,
            },
            Destinataire {
                team_id: TeamId::new(),
                coach_id: coach,
            },
        ];

        let survey = ouvrir(&ses_deux_equipes);

        assert_eq!(survey.engagees().len(), 2);
        assert_ne!(
            survey.reponses[0].token(),
            survey.reponses[1].token(),
            "un jeton par équipe : il désigne la réponse, pas le coach"
        );
    }

    // ── R2 — jamais sur une journée de repos ─────────────────────────────────

    #[test]
    fn une_journee_de_repos_ne_se_sonde_pas() {
        let refus = PresenceSurvey::ouvrir(
            SurveyId::new(),
            SeasonId::new(),
            &journee(MatchDayType::Rest),
            &destinataires(4),
            echeance("2026-10-10"),
            AutoRemind::new(false),
            &date("2026-10-01"),
        );

        assert_eq!(refus.unwrap_err(), DomainError::SurveyOnRestDay);
    }

    // ── R3 — un coach sans adresse n'empêche pas le lancement ────────────────

    /// L'agrégat ne connaît pas les adresses : c'est précisément ce qui fait
    /// tenir R3. Le service d'hydratation compte ceux qui n'en ont pas, et les
    /// fait entrer dans la campagne comme les autres.
    #[test]
    fn tous_les_destinataires_entrent_dans_la_campagne() {
        let survey = ouvrir(&destinataires(14));

        assert_eq!(survey.engagees().len(), 14);
        assert_eq!(survey.compte_sans_reponse(), 14);
    }

    // ── R5 — sans réponse vaut absent, mais se compte à part ─────────────────

    #[test]
    fn les_silencieux_ne_sont_ni_presents_ni_absents() {
        let mut survey = ouvrir(&destinataires(5));
        declarer(&mut survey, 0, Venue::Presente);
        declarer(&mut survey, 1, Venue::Presente);
        declarer(&mut survey, 2, Venue::Absente);

        assert_eq!(survey.compte_presents(), 2);
        assert_eq!(survey.compte_absents(), 1);
        assert_eq!(survey.compte_sans_reponse(), 2);
        assert_eq!(
            survey.presents().len(),
            2,
            "le tirage ne retient que les présences confirmées"
        );
    }

    // ── R15 — le tirage refuse en dessous de deux présents ───────────────────

    #[test]
    fn un_seul_present_ne_suffit_pas_a_tirer() {
        let mut survey = ouvrir(&destinataires(4));
        declarer(&mut survey, 0, Venue::Presente);

        assert_eq!(
            survey.peut_tirer().unwrap_err(),
            DomainError::NotEnoughPresent { presents: 1 }
        );
    }

    #[test]
    fn deux_presents_suffisent() {
        let mut survey = ouvrir(&destinataires(4));
        declarer(&mut survey, 0, Venue::Presente);
        declarer(&mut survey, 1, Venue::Presente);

        assert!(survey.peut_tirer().is_ok());
    }

    // ── R23 — la clôture est calculée, jamais subie ──────────────────────────

    #[test]
    fn une_campagne_echue_est_close_sans_que_rien_ne_l_ecrive() {
        let survey = ouvrir(&destinataires(4));

        assert_eq!(survey.statut(&date("2026-10-09")), SurveyStatus::Ouverte);
        assert_eq!(
            survey.statut(&date("2026-10-10")),
            SurveyStatus::Ouverte,
            "le jour de l'échéance, on répond encore"
        );
        assert_eq!(
            survey.statut(&date("2026-10-11")),
            SurveyStatus::Close(Motif::Echeance)
        );
        assert_eq!(survey.fermeture(), &Fermeture::Aucune, "rien n'est stocké");
    }

    #[test]
    fn une_cloture_decidee_prime_sur_l_echeance() {
        let mut survey = ouvrir(&destinataires(4));
        survey.fermeture = Fermeture::Decidee {
            le: FermeeLe::try_new("2026-10-03".to_string()).unwrap(),
        };

        assert_eq!(
            survey.statut(&date("2026-10-05")),
            SurveyStatus::Close(Motif::Decision),
            "close par décision, alors que l'échéance est à venir"
        );
    }

    // ── R7 / R28 — le jeton désigne la réponse, et l'autorisation se distingue ─

    #[test]
    fn un_jeton_retrouve_sa_reponse_et_une_seule() {
        let survey = ouvrir(&destinataires(6));
        let jeton = *survey.reponses[3].token();

        let trouvee = survey.reponse_par_jeton(&jeton).expect("réponse");

        assert_eq!(trouvee.team_id(), survey.reponses[3].team_id());
        assert!(survey.reponse_par_jeton(&SurveyToken::new()).is_none());
    }

    #[test]
    fn les_trois_repondants_sont_des_valeurs_distinctes() {
        let coach = CoachId::new();
        assert_ne!(Repondant::Jeton, Repondant::Coach(coach));
        assert_ne!(Repondant::Coach(coach), Repondant::Organisateur(coach));
    }

    // ── Ce que la forme garantit ─────────────────────────────────────────────

    #[test]
    fn une_campagne_neuve_n_est_pas_appariee() {
        assert_eq!(ouvrir(&destinataires(4)).appariement(), &Appariement::Aucun);
    }
}
