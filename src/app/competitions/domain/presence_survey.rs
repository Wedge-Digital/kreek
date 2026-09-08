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
//! La carte 511 a posé les types, la construction et les lectures ; la 512
//! `enregistrer` et `desaccord` ; la 513 `valider_proposition`, `clore`,
//! `rouvrir`, `marquer_appariee` et `defaire_appariement`. L'agrégat est complet.
//!
//! # Aucun champ n'est `pub`
//!
//! Le seul chemin d'écriture d'une `Presence` est `enregistrer`, qui porte R19,
//! R13, R21 et R28 ensemble. Un `pub` sur `reponses` les rendrait
//! contournables par un `survey.reponses[0].presence = …` que ni le
//! compilateur, ni `check-arch`, ni la revue ne signaleraient.

use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::match_day::MatchDay;
use crate::app::competitions::domain::tirage::{paire, DrawProposal};
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{CoachId, EntityId};
use crate::app::shared_kernel::identity::sulid::SUlid;
use nutype::nutype;
use std::collections::HashSet;

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

// ── Les faits que le use case apporte ────────────────────────────────────────

/// Une rencontre de la journée, **telle qu'elle existe en base**.
///
/// R24 — les appariements appartiennent à la journée, pas à la campagne. Ce type
/// est donc un fait de passage : le use case le lit sur la journée et le donne à
/// l'agrégat, qui décide. Il n'en garde rien.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RencontreJournee {
    pub pairing: PairingId,
    pub home: TeamId,
    pub away: TeamId,
}

impl RencontreJournee {
    fn contient(&self, team: &TeamId) -> bool {
        &self.home == team || &self.away == team
    }
}

/// Ce que le dehors sait de la journée, et que la campagne ne sait pas.
///
/// **Des faits, pas des ports.** Le use case interroge `IMatchReportStatusPort`
/// et le dépôt de journées une fois chacun et passe le résultat ; l'agrégat
/// tranche. La question « est-ce autorisé ? » reste dans le domaine, les faits
/// viennent du dehors — même patron que `&MatchDay` pour `ouvrir`.
///
/// **Les rencontres entières, pas la seule rencontre touchée.** Un paramètre
/// `rencontre_de: Option<PairingId>` calculé par le use case aurait sorti du
/// domaine la question « quelle rencontre est touchée ? », qui est métier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtatJournee {
    /// R13 — un rapport de match est publié sur cette journée.
    ///
    /// Primitif nu assumé : `EtatJournee` n'est ni un agrégat, ni une commande,
    /// ni un événement — c'est un fait de passage, que rien ne persiste.
    pub figee: bool, // arch:ok — fait fourni par le use case, pas un champ d'agrégat
    pub rencontres: Vec<RencontreJournee>,
}

impl EtatJournee {
    /// R24 — « appariée » n'est plus un champ de la campagne : c'est une
    /// observation faite sur la journée, à chaque affichage. Vider la journée au
    /// Calendrier ramène donc la campagne à son état d'avant tirage sans qu'aucune
    /// réconciliation n'ait à tourner : il n'y a plus rien à réconcilier.
    fn appariee(&self) -> bool {
        !self.rencontres.is_empty()
    }

    fn rencontre_de(&self, team: &TeamId) -> Option<&RencontreJournee> {
        self.rencontres.iter().find(|r| r.contient(team))
    }
}

/// Ce qu'`enregistrer` rend — **elle signale, elle ne répare pas**.
///
/// La maquette montre une proposition que l'organisateur valide, jamais un fait
/// accompli. Un `Result<(), _>` aurait obligé le use case à redécouvrir tout seul
/// qu'une rencontre est touchée.
///
/// **Une arrivée tardive ne rend jamais `EnregistreeRencontreARefaire`** (R16) :
/// un arrivant n'a aucune rencontre à refaire, il rejoint le vivier des
/// orphelins. C'est `desaccord` qui le fait voir, et c'est le bénéfice de R24 —
/// l'état se recalcule à chaque affichage au lieu de se lire. Cet enum dit la
/// conséquence immédiate pour l'appelant ; `desaccord` dit l'état complet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffetReponse {
    Enregistree,
    EnregistreeRencontreARefaire { pairing: PairingId },
}

/// Ce qui ne concorde plus entre les présences et les appariements réels.
///
/// **Se recalcule, ne se lit pas** — c'est R24. La forme précédente,
/// `rencontre_a_refaire()`, lisait un état stocké : elle ne savait qu'un
/// désaccord existe que parce que la campagne se souvenait des rencontres, ce
/// que quatre chemins du Calendrier rendaient faux sans rien lui dire. Ici l'état
/// « défection à traiter » survit à un rechargement de page.
///
/// **Les deux listes sont disjointes.** Un présent dont la rencontre est à
/// refaire n'est pas un orphelin : il *est* apparié, dans une rencontre qu'il
/// faut casser. Le vivier à réapparier est l'union des deux — les survivants des
/// rencontres cassées et les orphelins — et c'est le panneau de réparation qui la
/// fait, sur décision de l'organisateur. Les mêler ici ferait porter à
/// `orphelins` deux sens que rien ne distinguerait ensuite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Desaccord {
    /// R12 — une rencontre dont un des deux camps n'est plus présent.
    pub rencontres_a_refaire: Vec<PairingId>,
    /// R16 — un présent que **rien** n'apparie, l'exemptée exceptée.
    pub orphelins: Vec<TeamId>,
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

    /// Les réponses, **en lecture seule**. Le dépôt écrit par là, comme
    /// `save_notifications` reçoit un `&CompetitionNotifications` : un `&mut`
    /// rendrait contournable le seul chemin d'écriture d'une `Presence`.
    pub fn reponses(&self) -> &[Reponse] {
        &self.reponses
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

    /// R5 — les refus explicites, distincts des silencieux.
    ///
    /// Existe pour la même raison que `presents()` et `sans_reponse()` : c'est
    /// **l'agrégat qui classe**, et le service d'hydratation qui joint. Filtrer
    /// sur `Presence` dans la couche applicative ferait dire R5 à deux endroits.
    pub fn absents(&self) -> Vec<&Reponse> {
        self.filtrer(Presence::est_absente)
    }

    /// Les quatre comptes existent pour que `AvancementVm` **ne dérive rien**.
    /// Le réflexe serait de faire `rows.len()` sur chaque colonne : c'est le
    /// défaut de la carte 495, où la vue recomptait ce que le domaine savait
    /// compter, et comptait autre chose.
    pub fn compte_presents(&self) -> usize {
        self.presents().len()
    }

    pub fn compte_absents(&self) -> usize {
        self.absents().len()
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

    /// R15 — le tirage refuse en dessous de deux **appariables**, et le dit.
    ///
    /// Sans cette garde, l'aperçu s'afficherait vide et l'organisateur croirait à
    /// une panne. Même motif que `skipped_group_names` dans `generate_pairings` :
    /// signaler explicitement plutôt que laisser croire à un échec.
    ///
    /// **`inscrites` est indispensable, et son absence rendait la garde
    /// inopérante.** Une équipe désinscrite ne participe pas au tirage : R18 la
    /// refuse à la validation, et l'aperçu la filtre. Compter les présences seules
    /// laissait donc passer deux présents dont une désinscrite — le bouton
    /// s'activait, l'aperçu revenait vide, et l'organisateur tombait exactement
    /// sur le symptôme que R15 existe pour éviter.
    ///
    /// C'est le même défaut de forme que `verifier_l_exemption` a rencontré un
    /// étage plus haut : « combien peuvent réellement jouer ? » croise deux faits,
    /// la présence et l'inscription, et un seul des deux vit dans l'agrégat.
    pub fn peut_tirer(&self, inscrites: &HashSet<TeamId>) -> Result<(), DomainError> {
        let presents = self.compte_appariables(inscrites);
        if presents < 2 {
            return Err(DomainError::NotEnoughPresent { presents });
        }
        Ok(())
    }

    /// Les équipes que le tirage retiendra vraiment : présentes (R5) **et**
    /// toujours inscrites (R18).
    pub fn compte_appariables(&self, inscrites: &HashSet<TeamId>) -> usize {
        self.presents()
            .iter()
            .filter(|r| inscrites.contains(r.team_id()))
            .count()
    }

    // ── Ce que la campagne accepte de changer ────────────────────────────────

    /// **Le seul chemin d'écriture d'une `Presence`.**
    ///
    /// Quatre gardes, dans cet ordre : on identifie (R19), on autorise (R28),
    /// puis on regarde l'état (R13, R21). R19 précède R28 parce que R28 se pose
    /// *sur la réponse trouvée* — c'est son `coach_id` qui sert de référence.
    ///
    /// `par` est conservé tel quel dans la présence : c'est R6, l'identifiant de
    /// l'organisateur survit, et c'est lui qui distingue une réponse posée d'une
    /// réponse reçue.
    pub fn enregistrer(
        &mut self,
        team: &TeamId,
        venue: Venue,
        par: Repondant,
        journee: &EtatJournee,
        maintenant: &DateString,
    ) -> Result<EffetReponse, DomainError> {
        let rang = self.rang_de(team)?;
        self.verifier_le_proprietaire(rang, team, par)?;
        self.verifier_l_etat(journee, par, maintenant)?;

        let le = ReponduLe::try_new(maintenant.to_string())
            .map_err(|_| DomainError::InvalidReponduLe)?;
        self.reponses[rang].presence = Presence::Declaree { venue, le, par };

        Ok(effet(team, venue, journee))
    }

    /// Ce qui ne concorde plus entre les présences et les appariements **réels**
    /// de la journée — R24.
    ///
    /// Rend `None` sur une journée non appariée : il n'y a rien à contredire.
    pub fn desaccord(&self, journee: &EtatJournee) -> Option<Desaccord> {
        if !journee.appariee() {
            return None;
        }
        let d = Desaccord {
            rencontres_a_refaire: self.rencontres_a_refaire(journee),
            orphelins: self.orphelins(journee),
        };
        if d.rencontres_a_refaire.is_empty() && d.orphelins.is_empty() {
            return None;
        }
        Some(d)
    }

    /// R22 — **la validation revérifie la proposition, elle ne l'écrit pas sur
    /// parole.**
    ///
    /// L'aperçu ne persistant rien, la proposition part au client et revient
    /// modifiable. Ce n'est pas de la défiance envers l'organisateur : l'état a pu
    /// changer entre l'aperçu et la validation — une équipe désengagée, une réponse
    /// modifiée dans un autre onglet. La proposition était juste quand elle a été
    /// calculée, et ne l'est plus.
    ///
    /// **`inscrites` et non `engagees`** : l'agrégat a déjà `engagees()`, qui veut
    /// dire « sollicitée à l'ouverture ». R18 parle d'autre chose — toujours
    /// inscrite dans la saison — et c'est un fait du dehors. Deux notions sous un
    /// nom, c'est le défaut de la carte 495.
    pub fn valider_proposition(
        &self,
        prop: &DrawProposal,
        inscrites: &HashSet<TeamId>,
        interdites: &HashSet<(TeamId, TeamId)>,
    ) -> Result<(), DomainError> {
        for team in equipes_de(prop) {
            self.verifier_l_equipe(&team, inscrites)?;
        }
        for r in &prop.rencontres {
            verifier_la_paire(&r.home, &r.away, interdites)?;
        }
        verifier_l_unicite(prop)?;
        self.verifier_l_exemption(prop, inscrites, interdites)
    }

    /// R5 puis R18 — présente, et toujours inscrite. `est_presente` est le prédicat
    /// que `desaccord` utilise déjà : R5 dit ainsi la même chose aux deux endroits.
    fn verifier_l_equipe(
        &self,
        team: &TeamId,
        inscrites: &HashSet<TeamId>,
    ) -> Result<(), DomainError> {
        if !self.est_presente(team) {
            return Err(DomainError::TeamNotPresent {
                team: team.to_string(),
            });
        }
        if !inscrites.contains(team) {
            return Err(DomainError::TeamNoLongerEnrolled {
                team: team.to_string(),
            });
        }
        Ok(())
    }

    /// R22 — une exemptée n'est justifiée que si elle n'avait **personne** à jouer.
    ///
    /// C'est la règle de parité — impair ⇒ une exemptée, pair ⇒ aucune — écrite de
    /// façon à survivre à R10. Le critère strict refuserait une proposition
    /// légitime : quatre présents dont trois équipes d'un même coach donnent une
    /// rencontre, une exemptée et une équipe qui rentre sans jouer, soit un effectif
    /// pair **avec** une exemptée. `Cout::perdues` existe précisément pour ce cas.
    ///
    /// Sur un effectif où R10 ne mord pas, la vérification se réduit exactement à
    /// « pair ⇒ pas d'exemptée ».
    fn verifier_l_exemption(
        &self,
        prop: &DrawProposal,
        inscrites: &HashSet<TeamId>,
        interdites: &HashSet<(TeamId, TeamId)>,
    ) -> Result<(), DomainError> {
        let Some(exemptee) = prop.exemptee else {
            return Ok(());
        };
        let appariees: HashSet<TeamId> = prop
            .rencontres
            .iter()
            .flat_map(|r| [r.home, r.away])
            .collect();

        for r in self.presents() {
            let t = *r.team_id();
            if t == exemptee || appariees.contains(&t) || !inscrites.contains(&t) {
                continue;
            }
            if !interdites.contains(&paire(&exemptee, &t)) {
                return Err(DomainError::InconsistentProposal {
                    motif: "une équipe est exemptée alors qu'une autre reste sans match",
                });
            }
        }
        Ok(())
    }

    // ── Le cycle de la campagne ──────────────────────────────────────────────

    /// R23 — la clôture décidée, celle qui prime sur l'échéance.
    ///
    /// **Idempotente** : sur une campagne déjà close par décision, elle garde la
    /// première date. C'est la décision qui compte, et sa date est celle où elle a
    /// été prise — pas celle du second clic.
    pub fn clore(&mut self, maintenant: &DateString) -> Result<(), DomainError> {
        if let Fermeture::Decidee { .. } = self.fermeture {
            return Ok(());
        }
        let le =
            FermeeLe::try_new(maintenant.to_string()).map_err(|_| DomainError::InvalidFermeeLe)?;
        self.fermeture = Fermeture::Decidee { le };
        Ok(())
    }

    /// R13 et R23 — rouvrir **exige une nouvelle échéance**.
    ///
    /// La clôture étant calculée, rouvrir sans repousser la date rouvrirait sur une
    /// campagne close dans la seconde. Le seuil est `nouvelle < aujourd'hui` et non
    /// `<=` : `statut_de` ne clôt que **le lendemain** de l'échéance, donc une
    /// échéance au jour même laisse encore la journée pour répondre.
    ///
    /// Elle refuse une journée figée par un rapport publié : rouvrir réarme les
    /// anciens liens (R7), donc rouvrir une journée jouée ouvrirait la porte à des
    /// réponses sur un fait accompli. Une journée **appariée mais non jouée** reste
    /// permise — c'est le chemin normal quand une défection arrive après le tirage.
    ///
    /// Elle n'exige pas que la campagne soit close : rouvrir une campagne ouverte,
    /// c'est la prolonger, et rien ne s'y oppose.
    pub fn rouvrir(
        &mut self,
        nouvelle: SurveyDeadline,
        journee: &EtatJournee,
        maintenant: &DateString,
    ) -> Result<(), DomainError> {
        if journee.figee {
            return Err(DomainError::RoundFrozenByReport);
        }
        if nouvelle.as_ref() < maintenant.as_ref() {
            return Err(DomainError::DeadlineInThePast {
                deadline: nouvelle.to_string(),
            });
        }
        self.deadline = nouvelle;
        self.fermeture = Fermeture::Aucune;
        Ok(())
    }

    /// R9 — **enregistre l'exemption qui a eu lieu, pas celle qui était proposée.**
    ///
    /// La nuance vient de R9 croisée à R12 : quand l'exemptée reprend du service
    /// après une défection, elle n'a finalement pas été exemptée, et la compter
    /// comme telle la ferait passer devant à la journée suivante pour une exemption
    /// qu'elle n'a pas subie. C'est aussi pourquoi la réparation rappelle cette
    /// méthode avec la nouvelle exemptée au lieu d'un `remplacer_rencontre`
    /// distinct : les rencontres ayant quitté l'agrégat (R24), il n'y resterait rien
    /// à remplacer.
    ///
    /// Sans `Result` : aucune invariante à défendre ici, `valider_proposition` a
    /// déjà tranché.
    pub fn marquer_appariee(&mut self, exemptee: Option<TeamId>) {
        self.appariement = Appariement::Fait { exemptee };
    }

    pub fn defaire_appariement(&mut self) {
        self.appariement = Appariement::Aucun;
    }

    /// R19 — une réponse ne vaut que pour une équipe de la campagne : désengagée
    /// depuis l'ouverture, ou jamais engagée.
    fn rang_de(&self, team: &TeamId) -> Result<usize, DomainError> {
        self.reponses
            .iter()
            .position(|r| &r.team_id == team)
            .ok_or_else(|| DomainError::TeamNotInSurvey {
                team: team.to_string(),
            })
    }

    /// R28 — `Coach(id)` répond pour ses équipes, et pour elles seules.
    ///
    /// `Jeton` n'est pas contrôlé, et ce n'est pas un oubli : le jeton *est*
    /// l'autorisation (R7), et lui opposer un `CoachId` comparerait la réponse à
    /// elle-même. L'organisateur non plus : `require_admin_access` l'a déjà fait.
    fn verifier_le_proprietaire(
        &self,
        rang: usize,
        team: &TeamId,
        par: Repondant,
    ) -> Result<(), DomainError> {
        match par {
            Repondant::Coach(id) if self.reponses[rang].coach_id != id => {
                Err(DomainError::TeamNotOwnedByCoach {
                    team: team.to_string(),
                })
            }
            _ => Ok(()),
        }
    }

    /// R13 puis R21 — l'état de la journée, puis celui de la campagne.
    ///
    /// R13 vaut pour tout le monde, l'organisateur compris : une journée dont un
    /// rapport est publié ne se rejoue pas. R21 ne vaut que pour les chemins du
    /// coach — l'organisateur passe, c'est lui qui rattrape le coup de fil reçu
    /// après l'échéance (R6).
    fn verifier_l_etat(
        &self,
        journee: &EtatJournee,
        par: Repondant,
        maintenant: &DateString,
    ) -> Result<(), DomainError> {
        if journee.figee {
            return Err(DomainError::RoundFrozenByReport);
        }
        let chemin_du_coach = matches!(par, Repondant::Jeton | Repondant::Coach(_));
        if chemin_du_coach && !self.statut(maintenant).est_ouverte() {
            return Err(DomainError::SurveyClosedForCoach);
        }
        Ok(())
    }

    /// R12 — une rencontre dont un camp n'est plus présent. Une équipe désengagée
    /// depuis le tirage n'a plus de réponse du tout : elle n'est pas présente, et
    /// sa rencontre est donc à refaire, ce qui est le comportement voulu.
    fn rencontres_a_refaire(&self, journee: &EtatJournee) -> Vec<PairingId> {
        journee
            .rencontres
            .iter()
            .filter(|r| !self.est_presente(&r.home) || !self.est_presente(&r.away))
            .map(|r| r.pairing)
            .collect()
    }

    /// R16 — un présent que rien n'apparie, **l'exemptée exceptée** : elle n'est
    /// pas un orphelin à traiter, c'est le tirage qui l'a mise de côté. Sans cette
    /// exception, chaque journée impaire afficherait un désaccord permanent.
    ///
    /// C'est là que se paie l'exemption gardée dans l'agrégat quand les
    /// rencontres l'ont quitté (R24) : elle n'existe nulle part ailleurs.
    fn orphelins(&self, journee: &EtatJournee) -> Vec<TeamId> {
        self.presents()
            .iter()
            .map(|r| r.team_id)
            .filter(|t| journee.rencontre_de(t).is_none() && !self.est_exemptee(t))
            .collect()
    }

    fn est_presente(&self, team: &TeamId) -> bool {
        self.reponse_de(team)
            .is_some_and(|r| r.presence.compte_pour_le_tirage())
    }

    fn est_exemptee(&self, team: &TeamId) -> bool {
        matches!(&self.appariement, Appariement::Fait { exemptee: Some(e) } if e == team)
    }

    fn filtrer(&self, predicat: impl Fn(&Presence) -> bool) -> Vec<&Reponse> {
        self.reponses
            .iter()
            .filter(|r| predicat(&r.presence))
            .collect()
    }
}

/// Toutes les équipes que la proposition nomme — rencontres **et** exemptée.
/// L'exemptée doit subir R5 et R18 comme les autres : exempter une absente ou une
/// désinscrite est aussi faux qu'apparier l'une des deux.
fn equipes_de(prop: &DrawProposal) -> Vec<TeamId> {
    prop.rencontres
        .iter()
        .flat_map(|r| [r.home, r.away])
        .chain(prop.exemptee)
        .collect()
}

/// R10 — normalisée par `paire`, la fonction du tirage. Sans elle, `(a, b)` et
/// `(b, a)` seraient deux entrées et l'interdiction ne tiendrait que dans un sens.
fn verifier_la_paire(
    home: &TeamId,
    away: &TeamId,
    interdites: &HashSet<(TeamId, TeamId)>,
) -> Result<(), DomainError> {
    if interdites.contains(&paire(home, away)) {
        return Err(DomainError::ForbiddenPair {
            home: home.to_string(),
            away: away.to_string(),
        });
    }
    Ok(())
}

/// R22 — une équipe au plus une fois dans toute la proposition, exemption comprise.
fn verifier_l_unicite(prop: &DrawProposal) -> Result<(), DomainError> {
    let equipes = equipes_de(prop);
    let uniques: HashSet<&TeamId> = equipes.iter().collect();
    if uniques.len() == equipes.len() {
        return Ok(());
    }
    Err(DomainError::InconsistentProposal {
        motif: "une équipe y figure deux fois",
    })
}

/// R12 / R16 — la conséquence immédiate d'une réponse, jamais sa réparation.
///
/// Fonction libre : elle ne lit rien de la campagne, seulement la journée et ce
/// qui vient d'être écrit. Une méthode aurait suggéré qu'elle consulte un état
/// interne, ce qui est exactement ce que R24 lui retire.
fn effet(team: &TeamId, venue: Venue, journee: &EtatJournee) -> EffetReponse {
    match (venue, journee.rencontre_de(team)) {
        (Venue::Absente, Some(r)) => {
            EffetReponse::EnregistreeRencontreARefaire { pairing: r.pairing }
        }
        _ => EffetReponse::Enregistree,
    }
}

impl Reponse {
    /// Reconstruit une réponse depuis la base. **Réservé au dépôt** — le seul
    /// autre chemin vers une `Presence` est `enregistrer`.
    pub fn rehydrater(
        team_id: TeamId,
        coach_id: CoachId,
        token: SurveyToken,
        presence: Presence,
    ) -> Self {
        Self {
            team_id,
            coach_id,
            token,
            presence,
        }
    }

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
    use crate::app::competitions::domain::tirage::{Historique, ProposedPairing};

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

    fn vierge() -> EtatJournee {
        EtatJournee {
            figee: false,
            rencontres: vec![],
        }
    }

    /// Passe par le vrai chemin d'écriture — c'est tout l'objet de la carte 512 :
    /// les tests posés en 511 traversent désormais les règles.
    fn declarer(survey: &mut PresenceSurvey, rang: usize, venue: Venue) {
        let team = *survey.reponses[rang].team_id();
        survey
            .enregistrer(
                &team,
                venue,
                Repondant::Jeton,
                &vierge(),
                &date("2026-10-05"),
            )
            .expect("réponse acceptée");
    }

    fn rencontre(a: &TeamId, b: &TeamId) -> RencontreJournee {
        RencontreJournee {
            pairing: PairingId::new(),
            home: *a,
            away: *b,
        }
    }

    fn toutes_presentes(dests: &[Destinataire]) -> PresenceSurvey {
        let mut survey = ouvrir(dests);
        for rang in 0..dests.len() {
            declarer(&mut survey, rang, Venue::Presente);
        }
        survey
    }

    fn inscrites(dests: &[Destinataire]) -> HashSet<TeamId> {
        dests.iter().map(|d| d.team_id).collect()
    }

    fn proposee(a: &TeamId, b: &TeamId) -> ProposedPairing {
        ProposedPairing {
            home: *a,
            away: *b,
            historique: Historique::Inedite,
        }
    }

    fn proposition(rencontres: Vec<ProposedPairing>, exemptee: Option<TeamId>) -> DrawProposal {
        DrawProposal {
            rencontres,
            exemptee,
            ..Default::default()
        }
    }

    fn interdite(a: &TeamId, b: &TeamId) -> HashSet<(TeamId, TeamId)> {
        HashSet::from([paire(a, b)])
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
        let dests = destinataires(4);
        let mut survey = ouvrir(&dests);
        declarer(&mut survey, 0, Venue::Presente);

        assert_eq!(
            survey.peut_tirer(&inscrites(&dests)).unwrap_err(),
            DomainError::NotEnoughPresent { presents: 1 }
        );
    }

    #[test]
    fn deux_presents_suffisent() {
        let dests = destinataires(4);
        let mut survey = ouvrir(&dests);
        declarer(&mut survey, 0, Venue::Presente);
        declarer(&mut survey, 1, Venue::Presente);

        assert!(survey.peut_tirer(&inscrites(&dests)).is_ok());
    }

    /// Une désinscrite ne participe pas au tirage : la compter activerait le
    /// bouton pour un aperçu qui reviendrait vide — le symptôme même que R15
    /// existe pour éviter.
    #[test]
    fn une_desinscrite_ne_compte_pas_dans_les_appariables() {
        let dests = destinataires(4);
        let mut survey = ouvrir(&dests);
        declarer(&mut survey, 0, Venue::Presente);
        declarer(&mut survey, 1, Venue::Presente);
        let mut encore_inscrites = inscrites(&dests);
        encore_inscrites.remove(&dests[1].team_id);

        assert_eq!(survey.compte_presents(), 2, "deux présences déclarées");
        assert_eq!(
            survey.peut_tirer(&encore_inscrites).unwrap_err(),
            DomainError::NotEnoughPresent { presents: 1 },
            "mais une seule appariable, et le motif dit le nombre qui compte"
        );
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

    // ── R6 — l'identifiant de l'organisateur est conservé ─────────────────────

    #[test]
    fn une_presence_posee_par_l_organisateur_garde_son_identifiant() {
        let dests = destinataires(3);
        let mut survey = ouvrir(&dests);
        let admin = CoachId::new();

        survey
            .enregistrer(
                &dests[1].team_id,
                Venue::Presente,
                Repondant::Organisateur(admin),
                &vierge(),
                &date("2026-10-05"),
            )
            .expect("l'organisateur pose");

        let posee = survey.reponse_de(&dests[1].team_id).expect("réponse");
        assert!(
            matches!(posee.presence(), Presence::Declaree { par: Repondant::Organisateur(id), le, .. }
                     if id == &admin && le.as_ref() == "2026-10-05"),
            "R6 — qui a répondu, et quand, ne se perd pas"
        );
    }

    // ── R19 — une réponse ne vaut que pour une équipe de la campagne ──────────

    #[test]
    fn une_equipe_hors_campagne_est_refusee() {
        let mut survey = ouvrir(&destinataires(4));
        let etrangere = TeamId::new();

        let refus = survey.enregistrer(
            &etrangere,
            Venue::Presente,
            Repondant::Organisateur(CoachId::new()),
            &vierge(),
            &date("2026-10-05"),
        );

        assert_eq!(
            refus.unwrap_err(),
            DomainError::TeamNotInSurvey {
                team: etrangere.to_string()
            }
        );
    }

    // ── R28 — trois chemins, trois autorisations ─────────────────────────────

    #[test]
    fn un_coach_ne_repond_pas_pour_l_equipe_d_un_autre() {
        let dests = destinataires(3);
        let mut survey = ouvrir(&dests);

        let refus = survey.enregistrer(
            &dests[0].team_id,
            Venue::Presente,
            Repondant::Coach(dests[1].coach_id),
            &vierge(),
            &date("2026-10-05"),
        );

        assert_eq!(
            refus.unwrap_err(),
            DomainError::TeamNotOwnedByCoach {
                team: dests[0].team_id.to_string()
            },
            "R28 — R19 vérifie que l'équipe est dans la campagne, pas qu'elle est la sienne"
        );
    }

    /// Le jeton *est* l'autorisation (R7) : lui opposer un `CoachId` comparerait
    /// la réponse à elle-même. Il répond donc pour l'équipe qu'il désigne, sans
    /// que le domaine ait à savoir qui le détient.
    #[test]
    fn un_jeton_n_est_pas_confronte_a_un_proprietaire() {
        let dests = destinataires(3);
        let mut survey = ouvrir(&dests);

        let effet = survey.enregistrer(
            &dests[0].team_id,
            Venue::Presente,
            Repondant::Jeton,
            &vierge(),
            &date("2026-10-05"),
        );

        assert_eq!(effet.unwrap(), EffetReponse::Enregistree);
    }

    #[test]
    fn un_coach_repond_pour_sa_propre_equipe() {
        let dests = destinataires(3);
        let mut survey = ouvrir(&dests);

        let effet = survey.enregistrer(
            &dests[2].team_id,
            Venue::Absente,
            Repondant::Coach(dests[2].coach_id),
            &vierge(),
            &date("2026-10-05"),
        );

        assert_eq!(effet.unwrap(), EffetReponse::Enregistree);
        assert_eq!(survey.compte_absents(), 1);
    }

    // ── R13 — une journée figée par un rapport ne bouge plus ─────────────────

    /// R13 vaut pour tout le monde, l'organisateur compris : une journée dont un
    /// rapport est publié ne se rejoue pas.
    #[test]
    fn une_journee_figee_refuse_meme_l_organisateur() {
        let dests = destinataires(3);
        let mut survey = ouvrir(&dests);
        let figee = EtatJournee {
            figee: true,
            rencontres: vec![],
        };

        for par in [
            Repondant::Jeton,
            Repondant::Coach(dests[0].coach_id),
            Repondant::Organisateur(CoachId::new()),
        ] {
            let refus = survey.enregistrer(
                &dests[0].team_id,
                Venue::Absente,
                par,
                &figee,
                &date("2026-10-05"),
            );
            assert_eq!(refus.unwrap_err(), DomainError::RoundFrozenByReport);
        }
    }

    // ── R21 — la clôture arrête le coach, jamais l'organisateur ──────────────

    #[test]
    fn une_campagne_close_refuse_le_coach_et_accepte_l_organisateur() {
        let dests = destinataires(3);
        let mut survey = ouvrir(&dests);
        // Échéance au 2026-10-10 : le 11, la campagne est close par R23, sans
        // que rien ne l'ait écrite.
        let apres = date("2026-10-11");
        assert_eq!(survey.statut(&apres), SurveyStatus::Close(Motif::Echeance));

        for par in [Repondant::Jeton, Repondant::Coach(dests[0].coach_id)] {
            let refus =
                survey.enregistrer(&dests[0].team_id, Venue::Presente, par, &vierge(), &apres);
            assert_eq!(refus.unwrap_err(), DomainError::SurveyClosedForCoach);
        }

        let rattrapage = survey.enregistrer(
            &dests[0].team_id,
            Venue::Presente,
            Repondant::Organisateur(CoachId::new()),
            &vierge(),
            &apres,
        );
        assert!(
            rattrapage.is_ok(),
            "R6 — c'est l'organisateur qui rattrape le coup de fil reçu après l'échéance"
        );
    }

    // ── R12 — une défection après le tirage signale la rencontre touchée ─────

    #[test]
    fn un_present_qui_se_desiste_apres_le_tirage_designe_sa_rencontre() {
        let dests = destinataires(4);
        let mut survey = ouvrir(&dests);
        for rang in 0..4 {
            declarer(&mut survey, rang, Venue::Presente);
        }
        let journee = EtatJournee {
            figee: false,
            rencontres: vec![
                rencontre(&dests[0].team_id, &dests[1].team_id),
                rencontre(&dests[2].team_id, &dests[3].team_id),
            ],
        };
        let touchee = journee.rencontres[1].pairing;

        let effet = survey
            .enregistrer(
                &dests[3].team_id,
                Venue::Absente,
                Repondant::Jeton,
                &journee,
                &date("2026-10-05"),
            )
            .expect("la défection est enregistrée");

        assert_eq!(
            effet,
            EffetReponse::EnregistreeRencontreARefaire { pairing: touchee },
            "R12 — elle signale la rencontre touchée, et ne la répare pas"
        );
        assert_eq!(
            survey.desaccord(&journee),
            Some(Desaccord {
                rencontres_a_refaire: vec![touchee],
                orphelins: vec![],
            }),
            "l'adversaire du désistant n'est pas un orphelin : il est dans la \
             rencontre à casser, et le vivier à réapparier est l'union des deux"
        );
    }

    // ── R16 — une arrivée tardive n'a aucune rencontre à refaire ──────────────

    /// Le sens inverse de R12 existe autant, et il ne rend pas
    /// `EnregistreeRencontreARefaire` : un arrivant n'apparaît dans aucune
    /// rencontre. C'est `desaccord` qui le fait voir, et c'est exactement le
    /// bénéfice de R24 — l'état se recalcule au lieu de se lire.
    #[test]
    fn une_arrivee_tardive_rejoint_le_vivier_des_orphelins() {
        let dests = destinataires(3);
        let mut survey = ouvrir(&dests);
        declarer(&mut survey, 0, Venue::Presente);
        declarer(&mut survey, 1, Venue::Presente);
        declarer(&mut survey, 2, Venue::Absente);
        let journee = EtatJournee {
            figee: false,
            rencontres: vec![rencontre(&dests[0].team_id, &dests[1].team_id)],
        };
        assert_eq!(survey.desaccord(&journee), None, "le tirage concorde");

        let effet = survey
            .enregistrer(
                &dests[2].team_id,
                Venue::Presente,
                Repondant::Jeton,
                &journee,
                &date("2026-10-05"),
            )
            .expect("l'arrivant est enregistré");

        assert_eq!(effet, EffetReponse::Enregistree);
        assert_eq!(
            survey.desaccord(&journee),
            Some(Desaccord {
                rencontres_a_refaire: vec![],
                orphelins: vec![dests[2].team_id],
            })
        );
    }

    /// R9 croisée à R16 : l'exemptée est un présent que rien n'apparie, et ce
    /// n'est pas un désaccord. Sans cette exception, chaque journée impaire
    /// afficherait une défection permanente à traiter.
    #[test]
    fn l_exemptee_n_est_pas_un_orphelin() {
        let dests = destinataires(3);
        let mut survey = ouvrir(&dests);
        for rang in 0..3 {
            declarer(&mut survey, rang, Venue::Presente);
        }
        survey.appariement = Appariement::Fait {
            exemptee: Some(dests[2].team_id),
        };
        let journee = EtatJournee {
            figee: false,
            rencontres: vec![rencontre(&dests[0].team_id, &dests[1].team_id)],
        };

        assert_eq!(survey.desaccord(&journee), None);
    }

    // ── R24 — vider la journée au Calendrier ne laisse rien à réconcilier ─────

    #[test]
    fn une_journee_videe_au_calendrier_ne_produit_aucun_desaccord() {
        let dests = destinataires(4);
        let mut survey = ouvrir(&dests);
        for rang in 0..4 {
            declarer(&mut survey, rang, Venue::Presente);
        }
        survey.appariement = Appariement::Fait { exemptee: None };

        assert_eq!(
            survey.desaccord(&vierge()),
            None,
            "R24 — « appariée » se lit sur la journée : plus de rencontres, plus de désaccord"
        );
    }

    // ── R22 — la validation revérifie tout, elle n'écrit pas sur parole ───────

    #[test]
    fn une_proposition_juste_passe() {
        let dests = destinataires(5);
        let survey = toutes_presentes(&dests);
        let prop = proposition(
            vec![
                proposee(&dests[0].team_id, &dests[1].team_id),
                proposee(&dests[2].team_id, &dests[3].team_id),
            ],
            Some(dests[4].team_id),
        );

        assert!(survey
            .valider_proposition(&prop, &inscrites(&dests), &HashSet::new())
            .is_ok());
    }

    // ── R5 — le tirage ne retient que les présences confirmées ───────────────

    #[test]
    fn une_equipe_sans_reponse_ne_peut_pas_etre_appariee() {
        let dests = destinataires(3);
        let mut survey = ouvrir(&dests);
        declarer(&mut survey, 0, Venue::Presente);
        // dests[1] reste silencieuse — R5 : sans réponse vaut absent pour le tirage.
        let prop = proposition(vec![proposee(&dests[0].team_id, &dests[1].team_id)], None);

        let refus = survey.valider_proposition(&prop, &inscrites(&dests), &HashSet::new());

        assert_eq!(
            refus.unwrap_err(),
            DomainError::TeamNotPresent {
                team: dests[1].team_id.to_string()
            }
        );
    }

    /// L'exemptée subit R5 comme les autres : exempter une absente est aussi faux
    /// qu'apparier une absente, et rien dans la forme de `DrawProposal` ne le dit.
    #[test]
    fn une_exemptee_absente_est_refusee() {
        let dests = destinataires(3);
        let mut survey = ouvrir(&dests);
        declarer(&mut survey, 0, Venue::Presente);
        declarer(&mut survey, 1, Venue::Presente);
        declarer(&mut survey, 2, Venue::Absente);
        let prop = proposition(
            vec![proposee(&dests[0].team_id, &dests[1].team_id)],
            Some(dests[2].team_id),
        );

        let refus = survey.valider_proposition(&prop, &inscrites(&dests), &HashSet::new());

        assert_eq!(
            refus.unwrap_err(),
            DomainError::TeamNotPresent {
                team: dests[2].team_id.to_string()
            }
        );
    }

    // ── R18 — la présence ne dit pas que l'équipe est toujours inscrite ──────

    #[test]
    fn une_equipe_desinscrite_depuis_sa_reponse_est_refusee() {
        let dests = destinataires(4);
        let survey = toutes_presentes(&dests);
        let mut encore_inscrites = inscrites(&dests);
        encore_inscrites.remove(&dests[3].team_id);
        let prop = proposition(
            vec![
                proposee(&dests[0].team_id, &dests[1].team_id),
                proposee(&dests[2].team_id, &dests[3].team_id),
            ],
            None,
        );

        let refus = survey.valider_proposition(&prop, &encore_inscrites, &HashSet::new());

        assert_eq!(
            refus.unwrap_err(),
            DomainError::TeamNoLongerEnrolled {
                team: dests[3].team_id.to_string()
            },
            "R18 — les deux faits vieillissent séparément"
        );
    }

    // ── R10 — deux équipes d'un même coach ne se rencontrent jamais ──────────

    /// L'interdiction tient dans les deux sens : `paire` normalise, donc une
    /// proposition qui inverse home et away ne la contourne pas.
    #[test]
    fn une_paire_interdite_est_refusee_dans_les_deux_sens() {
        let dests = destinataires(2);
        let survey = toutes_presentes(&dests);
        let interdites = interdite(&dests[0].team_id, &dests[1].team_id);

        for (a, b) in [(0, 1), (1, 0)] {
            let prop = proposition(vec![proposee(&dests[a].team_id, &dests[b].team_id)], None);
            let refus = survey.valider_proposition(&prop, &inscrites(&dests), &interdites);
            assert_eq!(
                refus.unwrap_err(),
                DomainError::ForbiddenPair {
                    home: dests[a].team_id.to_string(),
                    away: dests[b].team_id.to_string(),
                }
            );
        }
    }

    // ── R22 — la proposition ne se contredit pas elle-même ───────────────────

    #[test]
    fn une_equipe_en_double_est_refusee() {
        let dests = destinataires(3);
        let survey = toutes_presentes(&dests);
        let prop = proposition(
            vec![
                proposee(&dests[0].team_id, &dests[1].team_id),
                proposee(&dests[0].team_id, &dests[2].team_id),
            ],
            None,
        );

        assert_eq!(
            survey
                .valider_proposition(&prop, &inscrites(&dests), &HashSet::new())
                .unwrap_err(),
            DomainError::InconsistentProposal {
                motif: "une équipe y figure deux fois"
            }
        );
    }

    #[test]
    fn une_exemptee_qui_joue_aussi_est_refusee() {
        let dests = destinataires(3);
        let survey = toutes_presentes(&dests);
        let prop = proposition(
            vec![proposee(&dests[0].team_id, &dests[1].team_id)],
            Some(dests[1].team_id),
        );

        assert_eq!(
            survey
                .valider_proposition(&prop, &inscrites(&dests), &HashSet::new())
                .unwrap_err(),
            DomainError::InconsistentProposal {
                motif: "une équipe y figure deux fois"
            }
        );
    }

    /// La règle de parité : quatre présents, une seule rencontre, une exemptée —
    /// la quatrième rentrerait sans jouer alors que rien ne l'en empêche.
    #[test]
    fn une_exemptee_est_refusee_quand_une_autre_reste_sans_match() {
        let dests = destinataires(4);
        let survey = toutes_presentes(&dests);
        let prop = proposition(
            vec![proposee(&dests[0].team_id, &dests[1].team_id)],
            Some(dests[2].team_id),
        );

        assert_eq!(
            survey
                .valider_proposition(&prop, &inscrites(&dests), &HashSet::new())
                .unwrap_err(),
            DomainError::InconsistentProposal {
                motif: "une équipe est exemptée alors qu'une autre reste sans match"
            }
        );
    }

    /// La seule exception, et c'est R10 qui la force : quatre présents dont trois
    /// équipes d'un même coach donnent une rencontre, une exemptée et une équipe
    /// qui rentre sans jouer — un effectif **pair** avec une exemptée légitime. Le
    /// critère strict la refuserait, alors que `Cout::perdues` existe pour ce cas.
    #[test]
    fn une_exemptee_est_justifiee_quand_la_paire_restante_est_interdite() {
        let dests = destinataires(4);
        let survey = toutes_presentes(&dests);
        let prop = proposition(
            vec![proposee(&dests[0].team_id, &dests[1].team_id)],
            Some(dests[2].team_id),
        );
        let interdites = interdite(&dests[2].team_id, &dests[3].team_id);

        assert!(survey
            .valider_proposition(&prop, &inscrites(&dests), &interdites)
            .is_ok());
    }

    /// Une désinscrite laissée sans match ne rend pas l'exemption incohérente :
    /// R18 l'a déjà écartée du tirage, elle n'attend pas d'adversaire.
    #[test]
    fn une_desinscrite_sans_match_ne_condamne_pas_l_exemption() {
        let dests = destinataires(4);
        let survey = toutes_presentes(&dests);
        let mut encore_inscrites = inscrites(&dests);
        encore_inscrites.remove(&dests[3].team_id);
        let prop = proposition(
            vec![proposee(&dests[0].team_id, &dests[1].team_id)],
            Some(dests[2].team_id),
        );

        assert!(survey
            .valider_proposition(&prop, &encore_inscrites, &HashSet::new())
            .is_ok());
    }

    // ── R23 — clore, et rouvrir sans se refermer aussitôt ────────────────────

    /// La date de la clôture est celle où la décision a été prise, pas celle du
    /// second clic.
    #[test]
    fn clore_est_idempotente() {
        let mut survey = ouvrir(&destinataires(4));

        survey.clore(&date("2026-10-03")).expect("clôture");
        survey.clore(&date("2026-10-07")).expect("re-clôture");

        assert_eq!(
            survey.fermeture(),
            &Fermeture::Decidee {
                le: FermeeLe::try_new("2026-10-03".to_string()).unwrap()
            }
        );
        assert_eq!(
            survey.statut(&date("2026-10-05")),
            SurveyStatus::Close(Motif::Decision)
        );
    }

    #[test]
    fn rouvrir_refuse_une_echeance_deja_passee() {
        let mut survey = ouvrir(&destinataires(4));
        survey.clore(&date("2026-10-03")).unwrap();

        let refus = survey.rouvrir(echeance("2026-10-02"), &vierge(), &date("2026-10-05"));

        assert_eq!(
            refus.unwrap_err(),
            DomainError::DeadlineInThePast {
                deadline: "2026-10-02".to_string()
            },
            "R23 — la clôture étant calculée, elle se refermerait dans la seconde"
        );
    }

    /// Le seuil est `<` et non `<=` : `statut_de` ne clôt que le lendemain de
    /// l'échéance, donc une échéance au jour même laisse la journée pour répondre.
    #[test]
    fn rouvrir_accepte_l_echeance_du_jour_meme() {
        let mut survey = ouvrir(&destinataires(4));
        survey.clore(&date("2026-10-03")).unwrap();

        survey
            .rouvrir(echeance("2026-10-05"), &vierge(), &date("2026-10-05"))
            .expect("le jour même, on répond encore");

        assert_eq!(survey.statut(&date("2026-10-05")), SurveyStatus::Ouverte);
    }

    // ── R13 — rouvrir réarme les jetons, donc pas sur une journée jouée ──────

    #[test]
    fn rouvrir_refuse_une_journee_figee_par_un_rapport() {
        let mut survey = ouvrir(&destinataires(4));
        survey.clore(&date("2026-10-03")).unwrap();
        let figee = EtatJournee {
            figee: true,
            rencontres: vec![],
        };

        let refus = survey.rouvrir(echeance("2026-10-20"), &figee, &date("2026-10-05"));

        assert_eq!(refus.unwrap_err(), DomainError::RoundFrozenByReport);
    }

    /// Une journée **appariée mais non jouée** reste rouvrable : c'est le chemin
    /// normal quand une défection arrive après le tirage.
    #[test]
    fn rouvrir_une_journee_appariee_mais_non_jouee_reste_permis() {
        let dests = destinataires(2);
        let mut survey = toutes_presentes(&dests);
        survey.clore(&date("2026-10-03")).unwrap();
        let appariee = EtatJournee {
            figee: false,
            rencontres: vec![rencontre(&dests[0].team_id, &dests[1].team_id)],
        };

        survey
            .rouvrir(echeance("2026-10-20"), &appariee, &date("2026-10-05"))
            .expect("le chemin normal après une défection");

        assert_eq!(survey.statut(&date("2026-10-05")), SurveyStatus::Ouverte);
    }

    // ── R9 — l'exemption qui a eu lieu, pas celle qui était proposée ─────────

    #[test]
    fn marquer_appariee_garde_la_derniere_exemptee() {
        let dests = destinataires(3);
        let mut survey = ouvrir(&dests);

        survey.marquer_appariee(Some(dests[2].team_id));
        // La réparation : l'exemptée reprend du service, une autre est exemptée.
        survey.marquer_appariee(Some(dests[0].team_id));

        assert_eq!(
            survey.appariement(),
            &Appariement::Fait {
                exemptee: Some(dests[0].team_id)
            },
            "R9 — compter comme exemptée celle qui a repris du service la ferait \
             passer devant à la journée suivante pour une exemption non subie"
        );
    }

    #[test]
    fn defaire_appariement_ramene_la_campagne_avant_le_tirage() {
        let mut survey = ouvrir(&destinataires(4));
        survey.marquer_appariee(None);

        survey.defaire_appariement();

        assert_eq!(survey.appariement(), &Appariement::Aucun);
    }

    #[test]
    fn une_campagne_neuve_n_est_pas_appariee() {
        assert_eq!(ouvrir(&destinataires(4)).appariement(), &Appariement::Aucun);
    }
}
