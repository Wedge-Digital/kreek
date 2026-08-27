use crate::app::shared_kernel::bloodbowl::ids::{
    CompetitionId, MatchReportId, PlayerId, RosterId, SeasonId,
};
use crate::app::shared_kernel::bloodbowl::staff_counts::{
    ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::teams::domain::basket::RosterLineId;
use crate::app::teams::domain::costly_mistakes::{incident_for, loss_for, SEUIL_ERREURS_COUTEUSES};
use crate::app::teams::domain::error::DomainError;
use crate::app::teams::domain::treasury::{MovementReason, TreasuryMovement};
use crate::app::teams::domain::value_objects::{
    DedicatedFans, IncidentType, Kpo, MatchResult, RosterName, SppGain, StaffQuantity, StaffType,
    TeamName,
};
use serde::{Deserialize, Serialize};

// ── États ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParticipationStatus {
    PendingEnrollment,
    Enrolled,
    Dismissed,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GamePhase {
    ReadyToPlay,
    MatchReporting,
    PlayerImprovement,
    Recruitment,
    Dismissals,
    /// Entre les renvois et le retour au jeu, quand la trésorerie dépasse le
    /// seuil : un jet décide de ce qu'il en reste.
    CostlyMistakes,
    TemporaryRetirement,
    OffSeason,
}

// ── Événements domaine ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TeamDomainEvent {
    // Identité et cycle de vie
    TeamCreated {
        team_id: TeamId,
        space_id: SpaceId,
        competition_id: CompetitionId,
        competition_name: String,
        season_id: SeasonId,
        season_name: String,
        name: TeamName,
        logo_url: Option<String>,
        roster_id: RosterId,
        roster_name: RosterName,
        coach_id: CoachId,
        coach_name: String,
        treasury: Kpo,
        dedicated_fans: DedicatedFans,
        rerolls: RerollCount,
        apothecaries: ApothecaryCount,
        assistants: AssistantCount,
        cheerleaders: CheerleaderCount,
    },
    TeamEnrolled {
        competition_id: CompetitionId,
        competition_name: String,
        season_id: SeasonId,
        season_name: String,
    },
    TeamDismissed,
    TeamEnrollmentRejected {
        competition_id: Option<CompetitionId>,
        season_id: Option<SeasonId>,
    },

    // Rapport de match en cours de saisie
    MatchReportingStarted {
        match_report_id: MatchReportId,
    },
    /// Libère l'équipe : le rapport qu'elle saisissait a été annulé, son
    /// pairing ayant été supprimé. Sans cet événement l'équipe resterait en
    /// `MatchReporting` sans espoir d'en sortir — la seule autre issue est la
    /// publication du rapport, qui n'aura jamais lieu.
    MatchReportingCancelled {
        match_report_id: MatchReportId,
    },

    // Séquence post-match
    // Nommé en termes domaine — déclenché par l'app event MatchPlayed (IO layer)
    PostMatchSequenceStarted {
        result: MatchResult,
        dedicated_fans: DedicatedFans,
        treasury_income: Kpo,
        spp_gains: Vec<SppGain>,
    },
    /// Défait la séquence d'après-match — le rapport a été dépublié pour
    /// correction. `dedicated_fans` est la valeur **absolue** restaurée, pas un
    /// delta : l'écrêtage à 0..20 n'est pas inversible.
    PostMatchSequenceReverted {
        match_report_id: MatchReportId,
        dedicated_fans: DedicatedFans,
        treasury_refund: Kpo,
    },
    /// Les coups de pouce d'un match sont payés. Événement distinct du revenu de
    /// match, et non un montant net : le grand livre porte une ligne par
    /// événement, et « MatchIncome 40 » masquerait un « +50 de gains, −10 de
    /// coups de pouce » que ce livre existe pour raconter.
    InducementsPaid {
        match_report_id: MatchReportId,
        amount_kpo: Kpo,
    },
    /// Le pendant, à la dépublication : sans lui, corriger un rapport rendrait
    /// les gains mais garderait l'argent des coups de pouce.
    InducementsRefunded {
        match_report_id: MatchReportId,
        amount_kpo: Kpo,
    },
    PlayerImprovementPhaseValidated,
    /// `roster_line` est l'identifiant de la ligne de roster —
    /// `DEMO_GRANIT__PIETAILLE` —, pas un ULID. C'est ce que `players` consomme
    /// pour créer le joueur : compétences de base, valeur de départ et nom de
    /// poste s'en déduisent. Le champ était typé `PositionId`, c'est-à-dire un
    /// `SUlid`, qui n'aurait pas pu porter cette valeur.
    PlayerRecruited {
        /// Frappé par `teams`, et non par `players` à la réception : l'event
        /// store devient la source d'identité. Rejouer le flux redonne les
        /// mêmes joueurs, et un app event reçu deux fois est rejeté par la
        /// contrainte d'unicité de `players` au lieu de créer un doublon.
        player_id: PlayerId,
        roster_line: RosterLineId,
        base_value_kpo: Kpo,
        cost_kpo: Kpo,
    },
    StaffBought {
        staff_type: StaffType,
        quantity: StaffQuantity,
        cost_kpo: Kpo,
    },
    /// Un renvoi ne rembourse rien — il n'a donc aucun effet sur la trésorerie.
    /// Le champ `refund_kpo` qu'il portait ne correspondait à aucune règle : ni
    /// la création d'équipe ni les renvois ne rendent de l'or (carte 255).
    StaffDismissed {
        staff_type: StaffType,
        quantity: StaffQuantity,
    },
    RecruitmentPhaseValidated,
    /// `value_at_dismissal` ne sert à aucun calcul : elle documente ce que
    /// valait le joueur au moment du renvoi, information non reconstructible
    /// une fois qu'il a quitté l'effectif.
    PlayerDismissed {
        player_id: PlayerId,
        value_at_dismissal: Kpo,
    },
    DismissalsPhaseValidated,
    PlayerRetiredTemporarily {
        player_id: PlayerId,
    },
    RetirementPhaseValidated,
    /// L'équipe entre dans la phase des erreurs coûteuses : sa trésorerie
    /// dépasse le seuil, un jet lui reste à faire.
    CostlyMistakesPhaseStarted,
    CostlyMistakesApplied {
        roll: u8,
        incident: IncidentType,
        gp_lost: Kpo,
        /// Les dés de dégâts — 1D3 pour un incident mineur, 2D6 pour une
        /// catastrophe, vide sinon.
        ///
        /// `#[serde(default)]` parce que l'événement existait avant ce champ :
        /// l'aval de cette fonctionnalité a été écrit avant son producteur, et
        /// les rares événements déjà persistés doivent rester relisibles.
        #[serde(default)]
        damage_dice: Vec<u8>,
    },

    // Off-season
    OffSeasonStarted {
        season_id: SeasonId,
    },
    PlayerReEngaged {
        player_id: PlayerId,
    },
    PlayerNotReEngaged {
        player_id: PlayerId,
        value_kpo_at_release: Kpo,
    },
    OffSeasonCompleted,

    /// Valeur d'équipe recalculée, en **absolu** — jamais un delta. Appendu à
    /// chaque entrée en `ReadyToPlay`, même si la valeur n'a pas bougé : la
    /// suite de ces événements est l'historique de progression de la TV sur la
    /// saison, qu'on n'a nulle part ailleurs.
    TeamValueRecomputed {
        value: Kpo,
    },

    // Administration
    GamePhaseOverridden {
        admin_id: CoachId,
        from_phase: Option<GamePhase>,
        to_phase: GamePhase,
        reason: Option<String>,
    },

    // Modification d'identité
    TeamRenamed {
        name: TeamName,
    },
    InitialsChanged {
        initials: String,
    },
    LogoChanged {
        logo_url: String,
    },
}

impl TeamDomainEvent {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::TeamCreated { .. } => "TeamCreated",
            Self::TeamEnrolled { .. } => "TeamEnrolled",
            Self::TeamDismissed => "TeamDismissed",
            Self::TeamEnrollmentRejected { .. } => "TeamEnrollmentRejected",
            Self::MatchReportingStarted { .. } => "MatchReportingStarted",
            Self::MatchReportingCancelled { .. } => "MatchReportingCancelled",
            Self::PostMatchSequenceStarted { .. } => "PostMatchSequenceStarted",
            Self::PostMatchSequenceReverted { .. } => "PostMatchSequenceReverted",
            Self::InducementsPaid { .. } => "InducementsPaid",
            Self::InducementsRefunded { .. } => "InducementsRefunded",
            Self::PlayerImprovementPhaseValidated => "PlayerImprovementPhaseValidated",
            Self::PlayerRecruited { .. } => "PlayerRecruited",
            Self::StaffBought { .. } => "StaffBought",
            Self::StaffDismissed { .. } => "StaffDismissed",
            Self::RecruitmentPhaseValidated => "RecruitmentPhaseValidated",
            Self::PlayerDismissed { .. } => "PlayerDismissed",
            Self::DismissalsPhaseValidated => "DismissalsPhaseValidated",
            Self::PlayerRetiredTemporarily { .. } => "PlayerRetiredTemporarily",
            Self::RetirementPhaseValidated => "RetirementPhaseValidated",
            Self::CostlyMistakesPhaseStarted => "CostlyMistakesPhaseStarted",
            Self::CostlyMistakesApplied { .. } => "CostlyMistakesApplied",
            Self::OffSeasonStarted { .. } => "OffSeasonStarted",
            Self::PlayerReEngaged { .. } => "PlayerReEngaged",
            Self::PlayerNotReEngaged { .. } => "PlayerNotReEngaged",
            Self::OffSeasonCompleted => "OffSeasonCompleted",
            Self::TeamValueRecomputed { .. } => "TeamValueRecomputed",
            Self::GamePhaseOverridden { .. } => "GamePhaseOverridden",
            Self::TeamRenamed { .. } => "TeamRenamed",
            Self::InitialsChanged { .. } => "InitialsChanged",
            Self::LogoChanged { .. } => "LogoChanged",
        }
    }

    pub fn schema_version(&self) -> &'static str {
        "1.0"
    }
}

// ── Agrégat ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Team {
    pub id: TeamId,
    pub space_id: SpaceId,
    pub name: TeamName,
    pub initials: String, // arch:ok texte libre calculé
    pub logo_url: Option<String>,
    pub roster_id: RosterId,
    pub roster_name: RosterName,
    pub coach_id: CoachId,
    pub coach_name: String, // arch:ok texte libre dénormalisé
    pub competition_id: Option<CompetitionId>,
    pub competition_name: Option<String>,
    pub season_id: Option<SeasonId>,
    pub season_name: Option<String>,
    pub participation_status: ParticipationStatus,
    pub game_phase: Option<GamePhase>,
    pub dedicated_fans: DedicatedFans,
    pub treasury: Kpo,
    pub team_value: Kpo,
    pub rerolls: RerollCount,
    pub apothecaries: ApothecaryCount,
    pub assistants: AssistantCount,
    pub cheerleaders: CheerleaderCount,
    pub current_match_report_id: Option<MatchReportId>,
    /// Ce que la dernière séquence d'après-match a changé, pour pouvoir la
    /// défaire si le rapport est dépublié pour correction.
    ///
    /// État **dérivé** : reconstruit à chaque `hydrate()` depuis les événements
    /// existants, jamais persisté. C'est ce qui évite toute migration — les
    /// deux informations qu'il capture sont encore lisibles au moment précis où
    /// `apply(PostMatchSequenceStarted)` s'exécute, et perdues juste après.
    pub last_post_match: Option<LastPostMatch>,
    pub version: u64,
}

#[derive(Debug, Clone)]
pub struct LastPostMatch {
    pub match_report_id: MatchReportId,
    /// Les fans **d'avant** le match. L'événement ne stocke que la valeur
    /// post-écrêtage : `clamp(0, 20)` n'étant pas inversible, la restauration
    /// ne peut pas se faire par soustraction du modificateur.
    pub dedicated_fans_before: DedicatedFans,
    pub treasury_income: Kpo,
    /// Ce que les coups de pouce ont coûté à la caisse. Renseigné par
    /// `InducementsPaid`, qui suit `PostMatchSequenceStarted` dans le même lot :
    /// sans lui, dépublier rendrait les gains et garderait cet argent.
    pub inducement_spending: Kpo,
}

impl Default for Team {
    fn default() -> Self {
        Self {
            id: TeamId::new(),
            space_id: SpaceId::new(),
            name: TeamName::try_new("placeholder".to_string()).expect("valid"),
            initials: String::new(),
            logo_url: None,
            roster_id: RosterId(String::new()),
            roster_name: RosterName::try_new("placeholder".to_string()).expect("valid"),
            coach_id: CoachId::new(),
            coach_name: String::new(),
            competition_id: None,
            competition_name: None,
            season_id: None,
            season_name: None,
            participation_status: ParticipationStatus::PendingEnrollment,
            game_phase: None,
            dedicated_fans: DedicatedFans::try_new(0).expect("valid"),
            treasury: Kpo(0),
            team_value: Kpo(0),
            rerolls: RerollCount::default(),
            apothecaries: ApothecaryCount::default(),
            assistants: AssistantCount::default(),
            cheerleaders: CheerleaderCount::default(),
            current_match_report_id: None,
            last_post_match: None,
            version: 0,
        }
    }
}

impl TeamDomainEvent {
    pub fn to_enveloppe(&self, team_id: &str) -> crate::common::event_envelope::EventEnvelope {
        crate::common::event_envelope::EventEnvelope {
            event_id: crate::app::shared_kernel::identity::ids::EventId::new().to_string(),
            emitter: team_id.to_string(),
            event_type: self.type_name().to_string(),
            tags: serde_json::json!([]),
            payload: serde_json::to_value(self).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }
}

impl Team {
    /// Le mouvement de trésorerie que cet événement produit **sur cet état**.
    ///
    /// Méthode de l'agrégat et non de l'événement : seul l'état courant permet
    /// de connaître le montant *effectif*. Une bourde de 50 kPo sur une caisse
    /// de 30 en retire 30 — l'événement porte le montant décidé, le mouvement
    /// porte celui qui a eu lieu.
    ///
    /// **`match` exhaustif, sans `_ =>`, et il doit le rester.** Ajouter une
    /// variante d'événement doit casser la compilation tant que son effet sur
    /// la trésorerie n'est pas déclaré. Aucun test ne peut exprimer cette
    /// garantie : c'est l'absence de joker qui l'assure, et le compilateur qui
    /// la vérifie. Un `_ =>` la supprimerait en silence.
    pub fn treasury_movement(&self, event: &TeamDomainEvent) -> Option<TreasuryMovement> {
        let solde = self.treasury;
        match event {
            // ── Mouvements ──────────────────────────────────────────────────
            TeamDomainEvent::TeamCreated { treasury, .. } => Some(TreasuryMovement::credit(
                solde,
                *treasury,
                MovementReason::InitialEndowment,
            )),
            TeamDomainEvent::PostMatchSequenceStarted {
                treasury_income, ..
            } => Some(TreasuryMovement::credit(
                solde,
                *treasury_income,
                MovementReason::MatchIncome,
            )),
            TeamDomainEvent::PostMatchSequenceReverted {
                treasury_refund, ..
            } => Some(TreasuryMovement::debit(
                solde,
                *treasury_refund,
                MovementReason::MatchIncomeReverted,
            )),
            TeamDomainEvent::CostlyMistakesApplied { gp_lost, .. } => Some(
                TreasuryMovement::debit(solde, *gp_lost, MovementReason::CostlyMistake),
            ),
            TeamDomainEvent::PlayerRecruited { cost_kpo, .. } => Some(TreasuryMovement::debit(
                solde,
                *cost_kpo,
                MovementReason::PlayerRecruitment,
            )),
            TeamDomainEvent::StaffBought { cost_kpo, .. } => Some(TreasuryMovement::debit(
                solde,
                *cost_kpo,
                MovementReason::StaffPurchase,
            )),
            TeamDomainEvent::InducementsPaid { amount_kpo, .. } => Some(TreasuryMovement::debit(
                solde,
                *amount_kpo,
                MovementReason::InducementPurchase,
            )),
            TeamDomainEvent::InducementsRefunded { amount_kpo, .. } => Some(
                TreasuryMovement::credit(solde, *amount_kpo, MovementReason::InducementRefunded),
            ),

            // ── Sans effet sur la trésorerie ────────────────────────────────
            // Énumérés un à un plutôt que balayés : c'est la liste qui rend le
            // verrou opérant.
            TeamDomainEvent::TeamEnrolled { .. }
            | TeamDomainEvent::TeamDismissed
            | TeamDomainEvent::TeamEnrollmentRejected { .. }
            | TeamDomainEvent::MatchReportingStarted { .. }
            | TeamDomainEvent::MatchReportingCancelled { .. }
            | TeamDomainEvent::PlayerImprovementPhaseValidated
            | TeamDomainEvent::StaffDismissed { .. }
            | TeamDomainEvent::RecruitmentPhaseValidated
            | TeamDomainEvent::PlayerDismissed { .. }
            | TeamDomainEvent::DismissalsPhaseValidated
            | TeamDomainEvent::CostlyMistakesPhaseStarted
            | TeamDomainEvent::PlayerRetiredTemporarily { .. }
            | TeamDomainEvent::RetirementPhaseValidated
            | TeamDomainEvent::OffSeasonStarted { .. }
            | TeamDomainEvent::PlayerReEngaged { .. }
            | TeamDomainEvent::PlayerNotReEngaged { .. }
            | TeamDomainEvent::OffSeasonCompleted
            | TeamDomainEvent::TeamValueRecomputed { .. }
            | TeamDomainEvent::GamePhaseOverridden { .. }
            | TeamDomainEvent::TeamRenamed { .. }
            | TeamDomainEvent::InitialsChanged { .. }
            | TeamDomainEvent::LogoChanged { .. } => None,
        }
    }
}

impl Team {
    /// Rejoue un événement sur l'agrégat — pure, sans effet de bord.
    pub fn apply(mut self, event: &TeamDomainEvent) -> Self {
        // La trésorerie est traitée une fois pour toutes, ici : `apply` et le
        // grand livre lisent le même `treasury_movement()`, donc ils ne peuvent
        // pas diverger. Les bras ci-dessous ne la touchent plus.
        if let Some(mouvement) = self.treasury_movement(event) {
            self.treasury = mouvement.balance_after;
        }
        match event {
            TeamDomainEvent::TeamCreated {
                team_id,
                space_id,
                competition_id,
                competition_name,
                season_id,
                season_name,
                name,
                logo_url,
                roster_id,
                roster_name,
                coach_id,
                coach_name,
                dedicated_fans,
                rerolls,
                apothecaries,
                assistants,
                cheerleaders,
                // `treasury` est posé par `treasury_movement()` en tête d'apply.
                ..
            } => {
                self.id = *team_id;
                self.space_id = *space_id;
                self.competition_id = Some(*competition_id);
                self.competition_name = Some(competition_name.clone());
                self.season_id = Some(*season_id);
                self.season_name = Some(season_name.clone());
                self.name = name.clone();
                self.initials = initials_from(name.as_ref());
                self.logo_url = logo_url.clone();
                self.roster_id = roster_id.clone();
                self.roster_name = roster_name.clone();
                self.coach_id = *coach_id;
                self.coach_name = coach_name.clone();
                self.dedicated_fans = *dedicated_fans;
                self.rerolls = *rerolls;
                self.apothecaries = *apothecaries;
                self.assistants = *assistants;
                self.cheerleaders = *cheerleaders;
                self.participation_status = ParticipationStatus::PendingEnrollment;
                self.game_phase = None;
            }
            TeamDomainEvent::TeamEnrolled {
                competition_id,
                competition_name,
                season_id,
                season_name,
            } => {
                self.competition_id = Some(*competition_id);
                self.competition_name = Some(competition_name.clone());
                self.season_id = Some(*season_id);
                self.season_name = Some(season_name.clone());
                self.participation_status = ParticipationStatus::Enrolled;
                self.game_phase = Some(GamePhase::ReadyToPlay);
            }
            TeamDomainEvent::TeamDismissed => {
                self.participation_status = ParticipationStatus::Dismissed;
                self.game_phase = None;
            }
            TeamDomainEvent::TeamEnrollmentRejected { .. } => {
                self.participation_status = ParticipationStatus::Rejected;
            }
            TeamDomainEvent::MatchReportingStarted { match_report_id } => {
                self.game_phase = Some(GamePhase::MatchReporting);
                self.current_match_report_id = Some(*match_report_id);
            }
            TeamDomainEvent::MatchReportingCancelled { .. } => {
                // Retour exact à l'état d'avant `MatchReportingStarted` :
                // l'équipe redevient disponible pour un autre rapport.
                self.game_phase = Some(GamePhase::ReadyToPlay);
                self.current_match_report_id = None;
            }
            TeamDomainEvent::PostMatchSequenceStarted {
                dedicated_fans,
                treasury_income,
                ..
            } => {
                // Capturé AVANT les affectations qui suivent : `dedicated_fans`
                // va être écrasé et `current_match_report_id` remis à None.
                // Passé cet instant, ni l'un ni l'autre n'est reconstructible.
                self.last_post_match =
                    self.current_match_report_id
                        .map(|match_report_id| LastPostMatch {
                            match_report_id,
                            dedicated_fans_before: self.dedicated_fans,
                            treasury_income: *treasury_income,
                            // Complété par `InducementsPaid` juste après, s'il
                            // y a eu des achats.
                            inducement_spending: Kpo(0),
                        });
                self.dedicated_fans = *dedicated_fans;
                self.game_phase = Some(GamePhase::PlayerImprovement);
                self.current_match_report_id = None;
            }
            // Suit `PostMatchSequenceStarted` dans le même lot, et enrichit
            // l'instantané qu'il vient de poser. L'identifiant est vérifié :
            // rejouer un événement d'un autre match ne doit pas contaminer la
            // compensation en cours.
            TeamDomainEvent::InducementsPaid {
                match_report_id,
                amount_kpo,
            } => {
                if let Some(last) = self
                    .last_post_match
                    .as_mut()
                    .filter(|l| l.match_report_id == *match_report_id)
                {
                    last.inducement_spending = *amount_kpo;
                }
            }
            // Rien à retenir : le remboursement clôt l'affaire, et la trésorerie
            // a déjà été créditée en tête d'`apply`.
            TeamDomainEvent::InducementsRefunded { .. } => {}
            TeamDomainEvent::PostMatchSequenceReverted {
                match_report_id,
                dedicated_fans,
                ..
            } => {
                self.dedicated_fans = *dedicated_fans;
                self.game_phase = Some(GamePhase::MatchReporting);
                // Restauré car `start_post_match_sequence` exige cette phase, et
                // la re-publication en dépend.
                self.current_match_report_id = Some(*match_report_id);
                // C'est ici que se joue l'idempotence : une seconde compensation
                // ne trouvera plus de dernier après-match et sera refusée.
                self.last_post_match = None;
            }
            TeamDomainEvent::PlayerImprovementPhaseValidated => {
                self.game_phase = Some(GamePhase::Recruitment);
            }
            TeamDomainEvent::RecruitmentPhaseValidated => {
                self.game_phase = Some(GamePhase::Dismissals);
            }
            TeamDomainEvent::CostlyMistakesPhaseStarted => {
                self.game_phase = Some(GamePhase::CostlyMistakes);
            }
            TeamDomainEvent::DismissalsPhaseValidated => {
                // Simplification temporaire : la retraite temporaire (carte 39,
                // to_be_refined) n'étant pas encore implémentée, on revient
                // directement en ReadyToPlay plutôt que de bloquer l'équipe
                // dans une phase sans action possible.
                self.game_phase = Some(GamePhase::ReadyToPlay);
            }
            TeamDomainEvent::RetirementPhaseValidated => {
                self.game_phase = Some(GamePhase::OffSeason);
            }
            TeamDomainEvent::CostlyMistakesApplied { .. } => {
                self.game_phase = Some(GamePhase::ReadyToPlay);
            }
            TeamDomainEvent::PlayerRecruited { .. } => {}
            TeamDomainEvent::StaffBought {
                staff_type,
                quantity,
                ..
            } => {
                let qty = quantity.into_inner();
                match staff_type {
                    StaffType::Reroll => self.rerolls.0 = self.rerolls.0.saturating_add(qty),
                    StaffType::Apothecary => {
                        self.apothecaries.0 = self.apothecaries.0.saturating_add(qty)
                    }
                    StaffType::Assistant => {
                        self.assistants.0 = self.assistants.0.saturating_add(qty)
                    }
                    StaffType::Cheerleader => {
                        self.cheerleaders.0 = self.cheerleaders.0.saturating_add(qty)
                    }
                    StaffType::FansFactor => {}
                }
            }
            TeamDomainEvent::StaffDismissed {
                staff_type,
                quantity,
                ..
            } => {
                let qty = quantity.into_inner();
                match staff_type {
                    StaffType::Reroll => self.rerolls.0 = self.rerolls.0.saturating_sub(qty),
                    StaffType::Apothecary => {
                        self.apothecaries.0 = self.apothecaries.0.saturating_sub(qty)
                    }
                    StaffType::Assistant => {
                        self.assistants.0 = self.assistants.0.saturating_sub(qty)
                    }
                    StaffType::Cheerleader => {
                        self.cheerleaders.0 = self.cheerleaders.0.saturating_sub(qty)
                    }
                    // Le facteur fans n'est pas un compteur de staff : il
                    // alimente `dedicated_fans`, et ne se renvoie pas.
                    StaffType::FansFactor => {}
                }
            }
            // `PlayerDismissed` est bien émis (`dismiss_player`), mais sans
            // effet sur l'agrégat : la valeur d'équipe est recalculée par
            // `TeamValueRecomputed`, la trésorerie n'est pas touchée — un renvoi
            // ne rembourse rien — et l'appartenance à l'effectif vit dans
            // `players`.
            //
            // `PlayerNotReEngaged` reste, lui, jamais émis : il appartient à
            // l'intersaison (cartes 39, 40 et 43), pas à cette série. Le
            // supprimer reviendrait à le réécrire là-bas.
            TeamDomainEvent::PlayerDismissed { .. }
            | TeamDomainEvent::PlayerNotReEngaged { .. } => {}
            TeamDomainEvent::TeamValueRecomputed { value } => {
                self.team_value = *value;
            }
            TeamDomainEvent::OffSeasonStarted { .. } => {
                self.game_phase = Some(GamePhase::OffSeason);
            }
            TeamDomainEvent::OffSeasonCompleted => {
                self.participation_status = ParticipationStatus::PendingEnrollment;
                self.competition_id = None;
                self.competition_name = None;
                self.season_id = None;
                self.season_name = None;
                self.game_phase = None;
            }
            TeamDomainEvent::GamePhaseOverridden { to_phase, .. } => {
                self.game_phase = Some(to_phase.clone());
            }
            TeamDomainEvent::TeamRenamed { name } => {
                self.name = name.clone();
                self.initials = initials_from(name.as_ref());
            }
            TeamDomainEvent::InitialsChanged { initials } => {
                self.initials = initials.clone();
            }
            TeamDomainEvent::LogoChanged { logo_url } => {
                self.logo_url = Some(logo_url.clone());
            }
            // Événements sans impact sur l'état de l'agrégat
            TeamDomainEvent::PlayerRetiredTemporarily { .. }
            | TeamDomainEvent::PlayerReEngaged { .. } => {}
        }
        self.version += 1;
        self
    }

    /// Hydrate l'agrégat en rejouant une séquence d'événements.
    pub fn hydrate(events: &[TeamDomainEvent]) -> Option<Self> {
        events.iter().fold(None, |acc, event| {
            Some(match acc {
                None => Team::default().apply(event),
                Some(t) => t.apply(event),
            })
        })
    }

    // ── Commandes ──────────────────────────────────────────────────────────

    pub fn enroll(
        &self,
        competition_id: CompetitionId,
        competition_name: String,
        season_id: SeasonId,
        season_name: String,
    ) -> Result<TeamDomainEvent, DomainError> {
        match self.participation_status {
            ParticipationStatus::PendingEnrollment => Ok(TeamDomainEvent::TeamEnrolled {
                competition_id,
                competition_name,
                season_id,
                season_name,
            }),
            _ => Err(DomainError::InvalidTransition {
                from: self.participation_status.clone(),
                to: ParticipationStatus::Enrolled,
            }),
        }
    }

    pub fn reject_enrollment(&self) -> Result<TeamDomainEvent, DomainError> {
        match self.participation_status {
            ParticipationStatus::PendingEnrollment => Ok(TeamDomainEvent::TeamEnrollmentRejected {
                competition_id: self.competition_id,
                season_id: self.season_id,
            }),
            _ => Err(DomainError::InvalidTransition {
                from: self.participation_status.clone(),
                to: ParticipationStatus::Rejected,
            }),
        }
    }

    pub fn dismiss(&self) -> Result<TeamDomainEvent, DomainError> {
        match self.participation_status {
            ParticipationStatus::Enrolled => Ok(TeamDomainEvent::TeamDismissed),
            ParticipationStatus::Dismissed => Err(DomainError::AlreadyDismissed),
            _ => Err(DomainError::InvalidTransition {
                from: self.participation_status.clone(),
                to: ParticipationStatus::Dismissed,
            }),
        }
    }

    pub fn start_match_reporting(
        &self,
        match_report_id: MatchReportId,
    ) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::ReadyToPlay)?;
        Ok(TeamDomainEvent::MatchReportingStarted { match_report_id })
    }

    /// Libère l'équipe du rapport `match_report_id`, annulé faute de pairing.
    ///
    /// La seconde garde n'est pas cosmétique : sans elle, l'annulation tardive
    /// d'un vieux rapport libérerait une équipe déjà repartie sur un autre
    /// match. Elle assure aussi l'idempotence — un événement rejoué ne trouve
    /// plus l'équipe en saisie sur ce rapport.
    pub fn cancel_match_reporting(
        &self,
        match_report_id: MatchReportId,
    ) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::MatchReporting)?;
        if self.current_match_report_id != Some(match_report_id) {
            return Err(DomainError::NotReportingThisMatch);
        }
        Ok(TeamDomainEvent::MatchReportingCancelled { match_report_id })
    }

    pub fn start_post_match_sequence(
        &self,
        result: MatchResult,
        fan_mod: i8,
        treasury_income: Kpo,
        spp_gains: Vec<SppGain>,
    ) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::MatchReporting)?;
        // fan_mod est déjà la valeur finale saisie par le coach sur le rapport
        // de match (bornée -2..2 côté BC match_report) — appliquée telle
        // quelle, aucun recalcul via le résultat du match.
        let raw = (self.dedicated_fans.into_inner() as i16 + fan_mod as i16).max(0) as u8;
        let dedicated_fans = DedicatedFans::try_new(raw.min(20)).expect("clamped to valid range");
        Ok(TeamDomainEvent::PostMatchSequenceStarted {
            result,
            dedicated_fans,
            treasury_income,
            spp_gains,
        })
    }

    /// Défait la séquence d'après-match d'un rapport dépublié pour correction.
    ///
    /// Refuse si l'équipe a quitté la phase d'amélioration — elle aurait alors
    /// pu recruter ou acheter du staff, et la trésorerie ne serait plus celle
    /// qu'on croit défaire. Refuse aussi si le dernier après-match ne concerne
    /// pas ce rapport, ce qui rend l'opération idempotente.
    /// Paie les coups de pouce du match. Appelé **après**
    /// `start_post_match_sequence`, dont il complète l'instantané.
    ///
    /// Aucune garde de trésorerie : le budget a été vérifié par `match_report`
    /// au moment de l'achat, contre cette même caisse. Si le montant la
    /// dépassait malgré tout, `treasury_movement` écrête à zéro — le listener
    /// le signale plutôt que de le laisser passer sous silence.
    pub fn pay_inducements(
        &self,
        match_report_id: MatchReportId,
        amount_kpo: Kpo,
    ) -> TeamDomainEvent {
        TeamDomainEvent::InducementsPaid {
            match_report_id,
            amount_kpo,
        }
    }

    /// Défait la séquence d'après-match, coups de pouce compris.
    ///
    /// Rend un **lot** : rembourser les coups de pouce sans rendre les gains, ou
    /// l'inverse, laisserait la trésorerie fausse. Les deux montants sont lus
    /// sur le même instantané, avant toute application, donc leur ordre dans le
    /// lot n'a pas d'importance.
    pub fn revert_post_match_sequence(
        &self,
        match_report_id: MatchReportId,
    ) -> Result<Vec<TeamDomainEvent>, DomainError> {
        self.expect_phase(GamePhase::PlayerImprovement)?;
        let last = self
            .last_post_match
            .as_ref()
            .filter(|l| l.match_report_id == match_report_id)
            .ok_or(DomainError::NoPostMatchToRevert)?;
        let coups_de_pouce = last.inducement_spending;

        let mut lot = vec![TeamDomainEvent::PostMatchSequenceReverted {
            match_report_id,
            dedicated_fans: last.dedicated_fans_before,
            treasury_refund: last.treasury_income,
        }];
        // Aucun événement si rien n'a été dépensé : une ligne de grand livre à
        // zéro ne raconterait rien.
        if coups_de_pouce.0 > 0 {
            lot.push(TeamDomainEvent::InducementsRefunded {
                match_report_id,
                amount_kpo: coups_de_pouce,
            });
        }
        Ok(lot)
    }

    /// Recrute un joueur. Gardes : phase et trésorerie, **rien d'autre**.
    ///
    /// Ni le plafond de 16, ni les quotas de poste, ni les limites croisées ne
    /// sont vérifiés ici : `Team` ne connaît pas la composition de son effectif.
    /// Ces gardes vivent dans le panier de recrutement (carte 262), qui le
    /// porte hydraté.
    ///
    /// Le contrôle de trésorerie fait doublon avec celui du panier, qui
    /// raisonne en total — mais il protège l'invariant propre à l'agrégat : sa
    /// trésorerie ne devient jamais négative. Gardé comme filet.
    pub fn recruit_player(
        &self,
        player_id: PlayerId,
        roster_line: RosterLineId,
        base_value_kpo: Kpo,
        cost_kpo: Kpo,
    ) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::Recruitment)?;
        if self.treasury.0 < cost_kpo.0 {
            return Err(DomainError::InsufficientTreasury);
        }
        Ok(TeamDomainEvent::PlayerRecruited {
            player_id,
            roster_line,
            base_value_kpo,
            cost_kpo,
        })
    }

    /// Renvoie un joueur. Garde : la phase, et elle seule.
    ///
    /// Le plancher des onze joueurs éligibles est vérifié par le panier de
    /// renvois (carte 267), qui connaît les disponibilités. Un renvoi ne
    /// rembourse rien, donc aucun effet sur la trésorerie.
    pub fn dismiss_player(
        &self,
        player_id: PlayerId,
        value_at_dismissal: Kpo,
    ) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::Dismissals)?;
        Ok(TeamDomainEvent::PlayerDismissed {
            player_id,
            value_at_dismissal,
        })
    }

    pub fn buy_staff(
        &self,
        staff_type: StaffType,
        quantity: StaffQuantity,
        cost_kpo: Kpo,
    ) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::Recruitment)?;
        // Le facteur fans est le seul non achetable ici. Le droit du roster à
        // tel ou tel staff (`allowed_staff`) est vérifié par le panier, qui
        // connaît le catalogue — `Team` ne le connaît pas.
        if matches!(staff_type, StaffType::FansFactor) {
            return Err(DomainError::StaffTypeNotBuyable);
        }
        if self.treasury.0 < cost_kpo.0 {
            return Err(DomainError::InsufficientTreasury);
        }
        Ok(TeamDomainEvent::StaffBought {
            staff_type,
            quantity,
            cost_kpo,
        })
    }

    pub fn dismiss_staff(
        &self,
        staff_type: StaffType,
        quantity: StaffQuantity,
    ) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::Dismissals)?;
        let owned = match staff_type {
            StaffType::Reroll => self.rerolls.0,
            StaffType::Apothecary => self.apothecaries.0,
            StaffType::Assistant => self.assistants.0,
            StaffType::Cheerleader => self.cheerleaders.0,
            StaffType::FansFactor => return Err(DomainError::StaffTypeNotDismissable),
        };
        if quantity.into_inner() > owned {
            return Err(DomainError::InsufficientStaff);
        }
        Ok(TeamDomainEvent::StaffDismissed {
            staff_type,
            quantity,
        })
    }

    pub fn validate_improvement_phase(&self) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::PlayerImprovement)
            .map(|_| TeamDomainEvent::PlayerImprovementPhaseValidated)
    }

    pub fn validate_recruitment_phase(&self) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::Recruitment)
            .map(|_| TeamDomainEvent::RecruitmentPhaseValidated)
    }

    /// Deux sorties : au-dessus du seuil, l'équipe doit son jet ; en dessous,
    /// elle repart jouer.
    ///
    /// La règle vit **ici**, dans la méthode de commande ; `apply()` reste bête
    /// et applique un fait sans en décider. Aucune migration n'en découle : les
    /// équipes dont l'historique ne porte que `DismissalsPhaseValidated` se
    /// rejouent à l'identique.
    ///
    /// **Une hypothèse à ne pas perdre** : la trésorerie lue est celle de
    /// l'agrégat chargé avant l'application des renvois. C'est juste parce qu'un
    /// renvoi ne rembourse rien. Si cela changeait, il faudrait passer la
    /// trésorerie d'après-lot au lieu de la lire dans `self`.
    pub fn validate_dismissals_phase(&self) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::Dismissals)?;
        Ok(if self.treasury.0 >= SEUIL_ERREURS_COUTEUSES {
            TeamDomainEvent::CostlyMistakesPhaseStarted
        } else {
            TeamDomainEvent::DismissalsPhaseValidated
        })
    }

    /// Applique le jet des erreurs coûteuses.
    ///
    /// Ne reçoit que **les dés bruts** : l'incident et la perte, l'agrégat les
    /// établit lui-même depuis sa propre trésorerie.
    ///
    /// La forme rejetée les passait en paramètres — le use case aurait alors pu
    /// produire un événement disant « incident mineur, 2 000 kPo perdus » sans
    /// que rien ne l'en empêche, et **l'agrégat aurait signé un fait qu'il n'a
    /// pas établi**. Le prix est un double appel à une fonction pure sur deux
    /// entiers.
    pub fn apply_costly_mistakes(
        &self,
        roll: u8,
        damage_dice: Vec<u8>,
    ) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::CostlyMistakes)?;
        let incident = incident_for(self.treasury, roll);
        let gp_lost = loss_for(self.treasury, incident, &damage_dice);
        Ok(TeamDomainEvent::CostlyMistakesApplied {
            roll,
            incident,
            gp_lost,
            damage_dice,
        })
    }

    pub fn validate_retirement_phase(&self) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::TemporaryRetirement)
            .map(|_| TeamDomainEvent::RetirementPhaseValidated)
    }

    pub fn override_phase(
        &self,
        admin_id: CoachId,
        to_phase: GamePhase,
        reason: Option<String>,
    ) -> Result<TeamDomainEvent, DomainError> {
        if self.participation_status != ParticipationStatus::Enrolled {
            return Err(DomainError::NotEnrolled);
        }
        Ok(TeamDomainEvent::GamePhaseOverridden {
            admin_id,
            from_phase: self.game_phase.clone(),
            to_phase,
            reason,
        })
    }

    fn expect_phase(&self, expected: GamePhase) -> Result<(), DomainError> {
        if self.game_phase == Some(expected) {
            Ok(())
        } else {
            Err(DomainError::WrongGamePhase(self.game_phase.clone()))
        }
    }
}

/// Calcule les initiales (2 premières lettres des 2 premiers mots) depuis un nom.
fn initials_from(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RosterId, SeasonId};
    use crate::app::shared_kernel::bloodbowl::staff_counts::{
        ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
    use crate::app::teams::domain::value_objects::{DedicatedFans, RosterName, TeamName};

    fn team_id() -> TeamId {
        TeamId::try_new("00000000000000000000000001").unwrap()
    }
    fn space_id() -> SpaceId {
        SpaceId::try_new("00000000000000000000000002").unwrap()
    }
    fn competition_id() -> CompetitionId {
        CompetitionId::try_new("00000000000000000000000003").unwrap()
    }
    fn season_id() -> SeasonId {
        SeasonId::try_new("00000000000000000000000004").unwrap()
    }
    fn roster_id() -> RosterId {
        RosterId::try_new("00000000000000000000000005").unwrap()
    }
    fn coach_id() -> CoachId {
        CoachId::try_new("00000000000000000000000006").unwrap()
    }
    /// L'identifiant du joueur recruté est frappé par l'appelant : les tests le
    /// fixent, ce qui les rend reproductibles.
    fn recrue() -> PlayerId {
        PlayerId::try_new("00000000000000000000000009").unwrap()
    }

    fn match_report_id() -> MatchReportId {
        MatchReportId::try_new("00000000000000000000000007").unwrap()
    }
    fn other_match_report_id() -> MatchReportId {
        MatchReportId::try_new("00000000000000000000000008").unwrap()
    }

    fn created_event() -> TeamDomainEvent {
        TeamDomainEvent::TeamCreated {
            team_id: team_id(),
            space_id: space_id(),
            competition_id: competition_id(),
            competition_name: "Ligue de Condate".to_string(),
            season_id: season_id(),
            season_name: "Saison 2025".to_string(),
            name: TeamName::try_new("Les Korrigans FC".to_string()).unwrap(),
            logo_url: None,
            roster_id: roster_id(),
            roster_name: RosterName::try_new("Elfes Sylvestres".to_string()).unwrap(),
            coach_id: coach_id(),
            coach_name: "Colonel Castor".to_string(),
            treasury: Kpo(1000),
            dedicated_fans: DedicatedFans::try_new(2).unwrap(),
            rerolls: RerollCount(3),
            apothecaries: ApothecaryCount(1),
            assistants: AssistantCount(2),
            cheerleaders: CheerleaderCount(3),
        }
    }

    fn enrolled_event() -> TeamDomainEvent {
        TeamDomainEvent::TeamEnrolled {
            competition_id: competition_id(),
            competition_name: "Ligue de Condate".to_string(),
            season_id: season_id(),
            season_name: "Saison 2025".to_string(),
        }
    }

    // ── Erreurs coûteuses (carte 408) ────────────────────────────────────────

    /// Une équipe en phase de renvois, avec la trésorerie voulue.
    ///
    /// La trésorerie est posée par un événement de crédit plutôt qu'écrite dans
    /// l'agrégat : c'est le seul chemin qui existe, et un test qui contournerait
    /// `apply` ne prouverait rien du rejeu.
    fn equipe_en_renvois(tresorerie: u32) -> Team {
        let mut evenements = vec![created_event()];
        evenements.push(TeamDomainEvent::TeamEnrolled {
            competition_id: competition_id(),
            competition_name: "Ligue de Condate".to_string(),
            season_id: season_id(),
            season_name: "Saison 2025".to_string(),
        });
        let mut equipe = Team::hydrate(&evenements).unwrap();
        // `TeamCreated` a posé 1000 kPo ; on redescend au montant voulu par un
        // débit, puis on amène l'équipe en phase de renvois.
        equipe.treasury = Kpo(tresorerie);
        equipe.game_phase = Some(GamePhase::Dismissals);
        equipe
    }

    #[test]
    fn sous_le_seuil_la_validation_des_renvois_ramene_au_jeu() {
        let event = equipe_en_renvois(99).validate_dismissals_phase().unwrap();
        assert!(matches!(event, TeamDomainEvent::DismissalsPhaseValidated));
    }

    /// Le seuil est **inclusif** : à 100 kPo tout rond, l'équipe doit son jet.
    #[test]
    fn au_seuil_exact_la_validation_ouvre_les_erreurs_couteuses() {
        let event = equipe_en_renvois(100).validate_dismissals_phase().unwrap();
        assert!(matches!(event, TeamDomainEvent::CostlyMistakesPhaseStarted));
    }

    #[test]
    fn l_entree_en_phase_change_la_phase_et_rien_d_autre() {
        let equipe = equipe_en_renvois(500);
        let tresorerie_avant = equipe.treasury;
        let apres = equipe.apply(&TeamDomainEvent::CostlyMistakesPhaseStarted);
        assert_eq!(apres.game_phase, Some(GamePhase::CostlyMistakes));
        assert_eq!(
            apres.treasury, tresorerie_avant,
            "entrer en phase ne coûte rien"
        );
    }

    /// L'agrégat ne reçoit que les dés : il établit lui-même l'incident et la
    /// perte depuis sa propre trésorerie. C'est ce qui l'empêche de signer un
    /// fait qu'il n'a pas établi.
    #[test]
    fn le_jet_est_traduit_par_l_agregat_depuis_sa_propre_tresorerie() {
        let equipe = equipe_en_renvois(560).apply(&TeamDomainEvent::CostlyMistakesPhaseStarted);
        let event = equipe.apply_costly_mistakes(1, vec![3, 4]).unwrap();
        match event {
            TeamDomainEvent::CostlyMistakesApplied {
                roll,
                incident,
                gp_lost,
                damage_dice,
            } => {
                assert_eq!(roll, 1);
                assert_eq!(incident, IncidentType::Catastrophe, "560 kPo, jet de 1");
                assert_eq!(gp_lost, Kpo(490), "il doit rester 70 kPo");
                assert_eq!(damage_dice, vec![3, 4], "les dés bruts sont conservés");
            }
            autre => panic!("attendu un jet appliqué, obtenu {autre:?}"),
        }
    }

    #[test]
    fn le_jet_hors_phase_est_refuse_et_ne_produit_rien() {
        let equipe = equipe_en_renvois(500);
        assert!(
            equipe.apply_costly_mistakes(1, vec![]).is_err(),
            "l'équipe est encore en phase de renvois"
        );
    }

    #[test]
    fn le_jet_applique_ramene_l_equipe_au_jeu_et_debite_la_perte() {
        let equipe = equipe_en_renvois(150).apply(&TeamDomainEvent::CostlyMistakesPhaseStarted);
        let event = equipe.apply_costly_mistakes(1, vec![2]).unwrap();
        let apres = equipe.apply(&event);
        assert_eq!(apres.game_phase, Some(GamePhase::ReadyToPlay));
        assert_eq!(apres.treasury, Kpo(130), "un mineur de 2 retire 20 kPo");
    }

    #[test]
    fn hydrate_creates_team_from_created_event() {
        let events = vec![created_event()];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.name.as_ref(), "Les Korrigans FC");
        assert_eq!(team.initials, "LK");
        assert_eq!(
            team.participation_status,
            ParticipationStatus::PendingEnrollment
        );
        assert_eq!(team.version, 1);
    }

    #[test]
    fn enroll_transitions_to_enrolled() {
        let events = vec![created_event()];
        let team = Team::hydrate(&events).unwrap();
        let event = team
            .enroll(
                competition_id(),
                "Ligue de Condate".to_string(),
                season_id(),
                "Saison 2025".to_string(),
            )
            .unwrap();
        let team = team.apply(&event);
        assert_eq!(team.participation_status, ParticipationStatus::Enrolled);
        assert_eq!(team.game_phase, Some(GamePhase::ReadyToPlay));
    }

    #[test]
    fn cannot_enroll_enrolled_team() {
        let events = vec![created_event(), enrolled_event()];
        let team = Team::hydrate(&events).unwrap();
        assert!(team
            .enroll(
                competition_id(),
                "Ligue".to_string(),
                season_id(),
                "Saison".to_string(),
            )
            .is_err());
    }

    #[test]
    fn dismiss_enrolled_team() {
        let events = vec![created_event(), enrolled_event()];
        let team = Team::hydrate(&events).unwrap();
        let event = team.dismiss().unwrap();
        let team = team.apply(&event);
        assert_eq!(team.participation_status, ParticipationStatus::Dismissed);
        assert_eq!(team.game_phase, None);
    }

    #[test]
    fn cannot_dismiss_already_dismissed() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::TeamDismissed,
        ];
        let team = Team::hydrate(&events).unwrap();
        assert!(matches!(team.dismiss(), Err(DomainError::AlreadyDismissed)));
    }

    /// Recruter débite la trésorerie et **ne touche plus à la TV** : celle-ci
    /// n'est plus accumulée, elle est recalculée puis écrasée par
    /// `TeamValueRecomputed`.
    #[test]
    fn player_recruited_debite_la_tresorerie_sans_toucher_a_la_tv() {
        let pos_id = RosterLineId("DEMO_GRANIT__PIETAILLE".to_string());
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::PlayerRecruited {
                player_id: recrue(),
                roster_line: pos_id.clone(),
                base_value_kpo: Kpo(95),
                cost_kpo: Kpo(95),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.treasury, Kpo(905)); // 1000 initial - 95 coût
        assert_eq!(team.team_value, Kpo(0), "la TV n'est plus incrémentale");
    }

    /// Rejeu d'un flux complet : achats de staff, recrutement et recalculs se
    /// succèdent, et la TV vaut exactement le **dernier** `TeamValueRecomputed`
    /// — jamais la somme de ce qui a précédé.
    #[test]
    fn le_rejeu_donne_la_tv_du_dernier_recalcul() {
        let pos_id = RosterLineId("DEMO_GRANIT__PIETAILLE".to_string());
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::TeamValueRecomputed { value: Kpo(550) },
            TeamDomainEvent::StaffBought {
                staff_type: StaffType::Apothecary,
                quantity: StaffQuantity::try_new(1).unwrap(),
                cost_kpo: Kpo(50),
            },
            TeamDomainEvent::PlayerRecruited {
                player_id: recrue(),
                roster_line: pos_id.clone(),
                base_value_kpo: Kpo(95),
                cost_kpo: Kpo(95),
            },
            TeamDomainEvent::TeamValueRecomputed { value: Kpo(695) },
        ];

        let team = Team::hydrate(&events).unwrap();

        assert_eq!(team.team_value, Kpo(695));
        // L'équipe naît avec un apothicaire, elle en achète un second
        assert_eq!(team.apothecaries.0, 2, "les compteurs de staff suivent");
        assert_eq!(team.treasury, Kpo(855)); // 1000 - 50 - 95
    }

    /// Une valeur en baisse doit passer telle quelle : l'événement porte un
    /// absolu, pas un delta, donc rien ne peut la « retenir ».
    #[test]
    fn un_recalcul_peut_faire_baisser_la_tv() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::TeamValueRecomputed { value: Kpo(700) },
            TeamDomainEvent::TeamValueRecomputed { value: Kpo(420) },
        ];
        assert_eq!(Team::hydrate(&events).unwrap().team_value, Kpo(420));
    }

    #[test]
    fn post_match_sequence_calculates_fans_correctly() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::MatchReportingStarted {
                match_report_id: match_report_id(),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        let event = team
            .start_post_match_sequence(MatchResult::Win, 2, Kpo(150), vec![])
            .unwrap();
        if let TeamDomainEvent::PostMatchSequenceStarted { dedicated_fans, .. } = &event {
            // 2 (initial: 1 base + 1 amélioration) + 2 (fan_mod du rapport) = 4
            assert_eq!(dedicated_fans.into_inner(), 4);
        } else {
            panic!("mauvais variant d'événement");
        }
    }

    #[test]
    fn post_match_sequence_clamps_fans_at_20() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::MatchReportingStarted {
                match_report_id: match_report_id(),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        // 2 fans + fan_mod 100 → clampé à 20 (la borne -2..2 est imposée côté
        // match_report/FanFactorMod, pas ici — ce test vérifie le clamp mécanique)
        let event = team
            .start_post_match_sequence(MatchResult::Win, 100, Kpo(0), vec![])
            .unwrap();
        if let TeamDomainEvent::PostMatchSequenceStarted { dedicated_fans, .. } = &event {
            assert_eq!(dedicated_fans.into_inner(), 20);
        } else {
            panic!("mauvais variant d'événement");
        }
    }

    #[test]
    fn post_match_sequence_negative_fan_mod_never_goes_below_zero() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::MatchReportingStarted {
                match_report_id: match_report_id(),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        // 2 fans - 2 (fan_mod minimal du rapport) = 0
        let event = team
            .start_post_match_sequence(MatchResult::Loss, -2, Kpo(0), vec![])
            .unwrap();
        if let TeamDomainEvent::PostMatchSequenceStarted { dedicated_fans, .. } = &event {
            assert_eq!(dedicated_fans.into_inner(), 0);
        } else {
            panic!("mauvais variant d'événement");
        }
    }

    #[test]
    fn phase_sequence_advances_correctly() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::PostMatchSequenceStarted {
                result: MatchResult::Win,
                dedicated_fans: DedicatedFans::try_new(5).unwrap(),
                treasury_income: Kpo(150),
                spp_gains: vec![],
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.game_phase, Some(GamePhase::PlayerImprovement));

        let event = team.validate_improvement_phase().unwrap();
        let team = team.apply(&event);
        assert_eq!(team.game_phase, Some(GamePhase::Recruitment));

        let event = team.validate_recruitment_phase().unwrap();
        let team = team.apply(&event);
        assert_eq!(team.game_phase, Some(GamePhase::Dismissals));

        // L'équipe de ce test a 1000 kPo en caisse : depuis la carte 408, la
        // validation des renvois l'envoie aux erreurs coûteuses avant de la
        // rendre au jeu. Une équipe pauvre y va toujours directement — c'est le
        // test `sous_le_seuil_la_validation_des_renvois_ramene_au_jeu`.
        let event = team.validate_dismissals_phase().unwrap();
        let team = team.apply(&event);
        assert_eq!(team.game_phase, Some(GamePhase::CostlyMistakes));

        let event = team.apply_costly_mistakes(6, vec![]).unwrap();
        let team = team.apply(&event);
        assert_eq!(team.game_phase, Some(GamePhase::ReadyToPlay));
    }

    #[test]
    fn initials_from_two_words() {
        assert_eq!(initials_from("Les Korrigans FC"), "LK");
        assert_eq!(initials_from("Nantes Undead"), "NU");
        assert_eq!(initials_from("Skaven"), "S");
    }

    fn recruitment_phase_team() -> Team {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::PostMatchSequenceStarted {
                result: MatchResult::Win,
                dedicated_fans: DedicatedFans::try_new(5).unwrap(),
                treasury_income: Kpo(150),
                spp_gains: vec![],
            },
            TeamDomainEvent::PlayerImprovementPhaseValidated,
            // treasury = 1000 + 150 = 1150, phase = Recruitment
        ];
        Team::hydrate(&events).unwrap()
    }

    fn dismissals_phase_team() -> Team {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::PostMatchSequenceStarted {
                result: MatchResult::Win,
                dedicated_fans: DedicatedFans::try_new(5).unwrap(),
                treasury_income: Kpo(150),
                spp_gains: vec![],
            },
            TeamDomainEvent::PlayerImprovementPhaseValidated,
            TeamDomainEvent::RecruitmentPhaseValidated,
            // phase = Dismissals
        ];
        Team::hydrate(&events).unwrap()
    }

    #[test]
    fn buy_staff_hors_phase_retourne_erreur() {
        let events = vec![created_event(), enrolled_event()];
        let team = Team::hydrate(&events).unwrap();
        // Phase ReadyToPlay — pas Recruitment
        assert!(matches!(
            team.buy_staff(
                StaffType::Reroll,
                StaffQuantity::try_new(1).unwrap(),
                Kpo(50)
            ),
            Err(DomainError::WrongGamePhase(_))
        ));
    }

    #[test]
    fn buy_staff_tresorerie_insuffisante_retourne_erreur() {
        let team = recruitment_phase_team();
        // treasury = 1150, coût = 2000
        assert!(matches!(
            team.buy_staff(
                StaffType::Reroll,
                StaffQuantity::try_new(1).unwrap(),
                Kpo(2000)
            ),
            Err(DomainError::InsufficientTreasury)
        ));
    }

    #[test]
    fn buy_staff_met_a_jour_compteur_et_tresorerie() {
        let team = recruitment_phase_team();
        assert_eq!(team.rerolls.0, 3);
        assert_eq!(team.treasury.0, 1150);

        let event = team
            .buy_staff(
                StaffType::Reroll,
                StaffQuantity::try_new(2).unwrap(),
                Kpo(100),
            )
            .unwrap();
        let team = team.apply(&event);

        assert_eq!(team.rerolls.0, 5);
        assert_eq!(team.treasury.0, 1050);
    }

    #[test]
    fn buy_assistant_met_a_jour_compteur() {
        let team = recruitment_phase_team();
        let event = team
            .buy_staff(
                StaffType::Assistant,
                StaffQuantity::try_new(1).unwrap(),
                Kpo(10),
            )
            .unwrap();
        let team = team.apply(&event);
        assert_eq!(team.assistants.0, 3); // 2 initial + 1
    }

    #[test]
    fn dismiss_staff_hors_phase_retourne_erreur() {
        let team = recruitment_phase_team();
        assert!(matches!(
            team.dismiss_staff(StaffType::Assistant, StaffQuantity::try_new(1).unwrap()),
            Err(DomainError::WrongGamePhase(_))
        ));
    }

    #[test]
    fn dismiss_staff_quantite_insuffisante_retourne_erreur() {
        let team = dismissals_phase_team();
        // 1 apothecary initial
        assert!(matches!(
            team.dismiss_staff(StaffType::Apothecary, StaffQuantity::try_new(2).unwrap()),
            Err(DomainError::InsufficientStaff)
        ));
    }

    #[test]
    fn dismiss_staff_decremente_le_compteur_sans_toucher_a_la_tresorerie() {
        let team = dismissals_phase_team();
        let treasury_before = team.treasury.0;

        let event = team
            .dismiss_staff(StaffType::Assistant, StaffQuantity::try_new(1).unwrap())
            .unwrap();
        let team = team.apply(&event);

        assert_eq!(team.assistants.0, 1); // 2 - 1
        assert_eq!(
            team.treasury.0, treasury_before,
            "un renvoi ne rembourse rien"
        );
    }

    // ── reject_enrollment ────────────────────────────────────────────────

    #[test]
    fn reject_enrollment_transitions_to_rejected() {
        let events = vec![created_event()];
        let team = Team::hydrate(&events).unwrap();
        let event = team.reject_enrollment().unwrap();
        let team = team.apply(&event);
        assert_eq!(team.participation_status, ParticipationStatus::Rejected);
    }

    #[test]
    fn cannot_reject_enrolled_team() {
        let events = vec![created_event(), enrolled_event()];
        let team = Team::hydrate(&events).unwrap();
        assert!(team.reject_enrollment().is_err());
    }

    #[test]
    fn cannot_reject_already_rejected_team() {
        let events = vec![
            created_event(),
            TeamDomainEvent::TeamEnrollmentRejected {
                competition_id: Some(competition_id()),
                season_id: Some(season_id()),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        assert!(team.reject_enrollment().is_err());
    }

    #[test]
    fn cannot_enroll_rejected_team() {
        let events = vec![
            created_event(),
            TeamDomainEvent::TeamEnrollmentRejected {
                competition_id: Some(competition_id()),
                season_id: Some(season_id()),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        assert!(team
            .enroll(
                competition_id(),
                "Ligue".to_string(),
                season_id(),
                "Saison".to_string(),
            )
            .is_err());
    }

    #[test]
    fn start_match_reporting_from_ready_to_play() {
        let events = vec![created_event(), enrolled_event()];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.game_phase, Some(GamePhase::ReadyToPlay));

        let event = team.start_match_reporting(match_report_id()).unwrap();
        let team = team.apply(&event);
        assert_eq!(team.game_phase, Some(GamePhase::MatchReporting));
    }

    #[test]
    fn start_match_reporting_sets_current_match_report_id() {
        let events = vec![created_event(), enrolled_event()];
        let team = Team::hydrate(&events).unwrap();
        let event = team.start_match_reporting(match_report_id()).unwrap();
        let team = team.apply(&event);
        assert_eq!(team.current_match_report_id, Some(match_report_id()));
    }

    #[test]
    fn cancel_match_reporting_frees_the_team() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::MatchReportingStarted {
                match_report_id: match_report_id(),
            },
        ];
        let team = Team::hydrate(&events).unwrap();

        let event = team.cancel_match_reporting(match_report_id()).unwrap();
        let team = team.apply(&event);

        assert_eq!(team.game_phase, Some(GamePhase::ReadyToPlay));
        assert_eq!(team.current_match_report_id, None);
    }

    #[test]
    fn cancel_match_reporting_allows_starting_another_report() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::MatchReportingStarted {
                match_report_id: match_report_id(),
            },
            TeamDomainEvent::MatchReportingCancelled {
                match_report_id: match_report_id(),
            },
        ];
        let team = Team::hydrate(&events).unwrap();

        // Le cœur de la carte : sans libération, l'équipe resterait bloquée.
        assert!(team.start_match_reporting(other_match_report_id()).is_ok());
    }

    #[test]
    fn cancel_match_reporting_wrong_phase_fails() {
        let events = vec![created_event(), enrolled_event()];
        let team = Team::hydrate(&events).unwrap();

        assert!(matches!(
            team.cancel_match_reporting(match_report_id()),
            Err(DomainError::WrongGamePhase(Some(GamePhase::ReadyToPlay)))
        ));
    }

    #[test]
    fn cancel_match_reporting_other_report_fails() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::MatchReportingStarted {
                match_report_id: match_report_id(),
            },
        ];
        let team = Team::hydrate(&events).unwrap();

        // L'équipe est repartie sur un autre match : l'annulation tardive du
        // premier ne doit pas la libérer du second.
        assert!(matches!(
            team.cancel_match_reporting(other_match_report_id()),
            Err(DomainError::NotReportingThisMatch)
        ));
    }

    #[test]
    fn cancel_match_reporting_twice_does_nothing() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::MatchReportingStarted {
                match_report_id: match_report_id(),
            },
            TeamDomainEvent::MatchReportingCancelled {
                match_report_id: match_report_id(),
            },
        ];
        let team = Team::hydrate(&events).unwrap();

        assert!(team.cancel_match_reporting(match_report_id()).is_err());
    }

    #[test]
    fn post_match_sequence_clears_current_match_report_id() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::MatchReportingStarted {
                match_report_id: match_report_id(),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.current_match_report_id, Some(match_report_id()));

        let event = team
            .start_post_match_sequence(MatchResult::Win, 4, Kpo(150), vec![])
            .unwrap();
        let team = team.apply(&event);
        assert_eq!(team.current_match_report_id, None);
    }

    #[test]
    fn start_match_reporting_wrong_phase_fails() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::PostMatchSequenceStarted {
                result: MatchResult::Win,
                dedicated_fans: DedicatedFans::try_new(5).unwrap(),
                treasury_income: Kpo(150),
                spp_gains: vec![],
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.game_phase, Some(GamePhase::PlayerImprovement));
        assert!(matches!(
            team.start_match_reporting(match_report_id()),
            Err(DomainError::WrongGamePhase(_))
        ));
    }

    #[test]
    fn cannot_dismiss_pending_team() {
        let events = vec![created_event()];
        let team = Team::hydrate(&events).unwrap();
        assert!(team.dismiss().is_err());
    }

    // ── hydratation séquences complètes ──────────────────────────────────

    #[test]
    fn hydrate_created_then_enrolled() {
        let events = vec![created_event(), enrolled_event()];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.participation_status, ParticipationStatus::Enrolled);
        assert_eq!(team.competition_name.as_deref(), Some("Ligue de Condate"));
        assert_eq!(team.version, 2);
    }

    #[test]
    fn hydrate_created_then_rejected() {
        let events = vec![
            created_event(),
            TeamDomainEvent::TeamEnrollmentRejected {
                competition_id: Some(competition_id()),
                season_id: Some(season_id()),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.participation_status, ParticipationStatus::Rejected);
        assert_eq!(team.version, 2);
    }

    #[test]
    fn hydrate_created_enrolled_then_dismissed() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::TeamDismissed,
        ];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.participation_status, ParticipationStatus::Dismissed);
        assert_eq!(team.game_phase, None);
        assert_eq!(team.version, 3);
    }
    // ── revert_post_match_sequence — compensation d'une dépublication ────────

    /// Équipe en phase Amélioration après un après-match, prête à être
    /// compensée. `fan_mod` et `gain` sont ceux du rapport.
    fn team_after_post_match(fan_mod: i8, gain: Kpo) -> Team {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::MatchReportingStarted {
                match_report_id: match_report_id(),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        let started = team
            .start_post_match_sequence(MatchResult::Win, fan_mod, gain, vec![])
            .unwrap();
        Team::hydrate(&[
            created_event(),
            enrolled_event(),
            TeamDomainEvent::MatchReportingStarted {
                match_report_id: match_report_id(),
            },
            started,
        ])
        .unwrap()
    }

    // ── Coups de pouce ────────────────────────────────────────────────────
    //
    // Le montant est déjà net de la petite monnaie quand il arrive ici :
    // `teams` ne connaît ni l'écart de valeur d'équipe ni les achats de
    // l'adversaire. Il débite ce que `match_report` a calculé, rien de plus.

    fn team_avec_coups_de_pouce(gain: Kpo, coups_de_pouce: Kpo) -> Team {
        let base = team_after_post_match(0, gain);
        base.clone()
            .apply(&base.pay_inducements(match_report_id(), coups_de_pouce))
    }

    #[test]
    fn payer_les_coups_de_pouce_debite_la_tresorerie() {
        let avant = team_after_post_match(0, Kpo(100)).treasury.0;
        let apres = team_avec_coups_de_pouce(Kpo(100), Kpo(30));
        assert_eq!(apres.treasury.0, avant - 30);
    }

    /// Le grand livre porte une ligne par événement : le débit des coups de
    /// pouce doit avoir son motif propre, pas se fondre dans le revenu du
    /// match. « MatchIncome 70 » masquerait « +100 de gains, −30 de coups de
    /// pouce ».
    #[test]
    fn le_motif_distingue_les_coups_de_pouce_du_revenu_de_match() {
        let team = team_after_post_match(0, Kpo(100));
        let paiement = team.pay_inducements(match_report_id(), Kpo(30));
        let mouvement = team
            .treasury_movement(&paiement)
            .expect("un paiement débite");
        assert_eq!(mouvement.reason, MovementReason::InducementPurchase);

        // Le contraste : sans lui, un `treasury_movement` qui rendrait le même
        // motif partout passerait pour une confirmation.
        let revenu = TeamDomainEvent::PostMatchSequenceStarted {
            result: MatchResult::Win,
            dedicated_fans: DedicatedFans::try_new(2).unwrap(),
            treasury_income: Kpo(100),
            spp_gains: vec![],
        };
        assert_eq!(
            team.treasury_movement(&revenu).unwrap().reason,
            MovementReason::MatchIncome
        );
    }

    /// Sans ce remboursement, corriger un rapport rendrait les gains du match
    /// mais garderait l'argent des coups de pouce — un trou de trésorerie à
    /// chaque dépublication.
    #[test]
    fn depublier_rembourse_les_coups_de_pouce() {
        let team = team_avec_coups_de_pouce(Kpo(100), Kpo(30));
        let depart = team_after_post_match(0, Kpo(100)).treasury.0 - 30;
        assert_eq!(team.treasury.0, depart);

        let apres = revert(&team);
        // Les gains repartent, les coups de pouce reviennent : on retrouve la
        // caisse d'avant le match.
        assert_eq!(apres.treasury.0, depart + 30 - 100);
    }

    /// Aucun achat, aucun événement : une ligne de grand livre à zéro ne
    /// raconterait rien.
    #[test]
    fn sans_achat_le_lot_de_depublication_ne_porte_qu_un_evenement() {
        let team = team_after_post_match(0, Kpo(100));
        let lot = team.revert_post_match_sequence(match_report_id()).unwrap();
        assert_eq!(lot.len(), 1);

        let team = team_avec_coups_de_pouce(Kpo(100), Kpo(30));
        let lot = team.revert_post_match_sequence(match_report_id()).unwrap();
        assert_eq!(lot.len(), 2, "les gains et les coups de pouce");
    }

    /// L'instantané est posé par `PostMatchSequenceStarted` et complété par
    /// `InducementsPaid` : dans l'autre ordre, la dépublication ne saurait pas
    /// quoi rembourser.
    #[test]
    fn un_paiement_d_un_autre_rapport_ne_contamine_pas_l_instantane() {
        let team = team_after_post_match(0, Kpo(100));
        let autre = MatchReportId::try_new("00000000000000000000000099").unwrap();
        let egare = team.pay_inducements(autre, Kpo(30));
        let team = team.apply(&egare);

        assert_eq!(
            team.last_post_match.as_ref().unwrap().inducement_spending,
            Kpo(0),
            "l'instantané ne retient que son propre rapport"
        );
    }

    fn revert(team: &Team) -> Team {
        let lot = team.revert_post_match_sequence(match_report_id()).unwrap();
        lot.iter().fold(team.clone(), |t, e| t.apply(e))
    }

    /// Le test décisif de la feature : l'équipe part de 2 fans, le rapport
    /// donne +100, l'écrêtage plafonne à 20. Restaurer par soustraction
    /// donnerait -80 ; seul l'instantané rend les 2 fans d'origine.
    #[test]
    fn revert_restaure_les_fans_ecretes_a_vingt() {
        let team = team_after_post_match(100, Kpo(0));
        assert_eq!(
            team.dedicated_fans.into_inner(),
            20,
            "précondition : écrêtage atteint"
        );

        let reverted = revert(&team);

        assert_eq!(reverted.dedicated_fans.into_inner(), 2);
    }

    /// Symétrique au plancher : -100 écrête à 0, et retrancher -100 donnerait
    /// 100 au lieu des 2 fans d'origine.
    #[test]
    fn revert_restaure_les_fans_apres_plancher_a_zero() {
        let team = team_after_post_match(-100, Kpo(0));
        assert_eq!(
            team.dedicated_fans.into_inner(),
            0,
            "précondition : plancher atteint"
        );

        let reverted = revert(&team);

        assert_eq!(reverted.dedicated_fans.into_inner(), 2);
    }

    #[test]
    fn revert_soustrait_le_gain_de_tresorerie() {
        let team = team_after_post_match(0, Kpo(150));
        assert_eq!(team.treasury.0, 1150);

        let reverted = revert(&team);

        assert_eq!(reverted.treasury.0, 1000);
    }

    #[test]
    fn revert_repasse_en_match_reporting_avec_le_rapport_courant() {
        let reverted = revert(&team_after_post_match(1, Kpo(100)));

        assert_eq!(reverted.game_phase, Some(GamePhase::MatchReporting));
        assert_eq!(reverted.current_match_report_id, Some(match_report_id()));
    }

    #[test]
    fn revert_refuse_si_la_phase_a_deja_avance() {
        let team = team_after_post_match(1, Kpo(100))
            .apply(&TeamDomainEvent::PlayerImprovementPhaseValidated);

        assert!(matches!(
            team.revert_post_match_sequence(match_report_id()),
            Err(DomainError::WrongGamePhase(_))
        ));
    }

    #[test]
    fn revert_refuse_un_autre_match_report_id() {
        let team = team_after_post_match(1, Kpo(100));
        let autre = MatchReportId::try_new("00000000000000000000000099").unwrap();

        assert!(matches!(
            team.revert_post_match_sequence(autre),
            Err(DomainError::NoPostMatchToRevert)
        ));
    }

    /// Règle 11 : rejouer la compensation ne doit rien produire de plus.
    #[test]
    fn un_second_revert_est_refuse() {
        let reverted = revert(&team_after_post_match(1, Kpo(100)));

        assert!(matches!(
            reverted.revert_post_match_sequence(match_report_id()),
            Err(DomainError::WrongGamePhase(_))
        ));
    }

    /// Règle 8 : le nombre de corrections n'est pas limité, donc le cycle doit
    /// converger — republier après avoir dépublié doit rendre exactement l'état
    /// de la première publication.
    #[test]
    fn publier_depublier_republier_converge_vers_le_meme_etat() {
        let apres_publication = team_after_post_match(2, Kpo(150));
        let reverted = revert(&apres_publication);

        let republie = reverted.clone().apply(
            &reverted
                .start_post_match_sequence(MatchResult::Win, 2, Kpo(150), vec![])
                .unwrap(),
        );

        assert_eq!(republie.dedicated_fans, apres_publication.dedicated_fans);
        assert_eq!(republie.treasury, apres_publication.treasury);
        assert_eq!(republie.game_phase, apres_publication.game_phase);
    }

    // ── Recruter et licencier (carte 261) ────────────────────────────────

    fn position() -> RosterLineId {
        RosterLineId("DEMO_GRANIT__PIETAILLE".to_string())
    }

    /// Test 18 de `recrutement/06-domaine.md`.
    #[test]
    fn recruit_player_hors_phase_recruitment_retourne_erreur() {
        let team = dismissals_phase_team();
        assert!(matches!(
            team.recruit_player(recrue(), position(), Kpo(50), Kpo(50)),
            Err(DomainError::WrongGamePhase(_))
        ));
    }

    /// Test 19 : le recrutement débite la trésorerie du coût.
    #[test]
    fn recruit_player_debite_la_tresorerie_du_cout() {
        let team = recruitment_phase_team();
        let avant = team.treasury.0; // 1150

        let event = team
            .recruit_player(recrue(), position(), Kpo(50), Kpo(90))
            .unwrap();
        let team = team.apply(&event);

        assert_eq!(team.treasury.0, avant - 90);
    }

    #[test]
    fn recruit_player_tresorerie_insuffisante_retourne_erreur() {
        let team = recruitment_phase_team();
        assert!(matches!(
            team.recruit_player(recrue(), position(), Kpo(50), Kpo(9_999)),
            Err(DomainError::InsufficientTreasury)
        ));
    }

    /// Test 12 de `renvois/06-domaine.md`.
    #[test]
    fn dismiss_player_hors_phase_dismissals_retourne_erreur() {
        let team = recruitment_phase_team();
        assert!(matches!(
            team.dismiss_player(PlayerId::new(), Kpo(70)),
            Err(DomainError::WrongGamePhase(_))
        ));
    }

    /// Tests 13 et 14 : un renvoi ne touche pas la trésorerie, et son
    /// événement ne produit aucun mouvement.
    #[test]
    fn dismiss_player_ne_touche_pas_la_tresorerie() {
        let team = dismissals_phase_team();
        let avant = team.treasury.0;

        let event = team.dismiss_player(PlayerId::new(), Kpo(70)).unwrap();
        assert!(
            team.treasury_movement(&event).is_none(),
            "un renvoi de joueur ne bouge pas la trésorerie"
        );

        let team = team.apply(&event);
        assert_eq!(team.treasury.0, avant);
    }

    // ── Staff : les deux refus qui n'avaient pas lieu d'être ─────────────

    /// Test 20 : l'apothicaire était renvoyable mais pas achetable.
    #[test]
    fn buy_staff_accepte_l_apothicaire() {
        let team = recruitment_phase_team();
        let event = team
            .buy_staff(
                StaffType::Apothecary,
                StaffQuantity::try_new(1).unwrap(),
                Kpo(50),
            )
            .unwrap();
        let team = team.apply(&event);
        assert_eq!(team.apothecaries.0, 2); // 1 initial + 1
    }

    /// Test 21 : le facteur fans reste le seul non achetable.
    #[test]
    fn buy_staff_refuse_le_facteur_fans() {
        let team = recruitment_phase_team();
        assert!(matches!(
            team.buy_staff(
                StaffType::FansFactor,
                StaffQuantity::try_new(1).unwrap(),
                Kpo(5)
            ),
            Err(DomainError::StaffTypeNotBuyable)
        ));
    }

    /// Test 22 : la relance était achetable mais pas renvoyable — et son
    /// compteur doit bien décroître, sans quoi le renvoi serait sans effet.
    #[test]
    fn dismiss_staff_accepte_la_relance_et_decremente_le_compteur() {
        let team = dismissals_phase_team();
        let avant = team.rerolls.0;

        let event = team
            .dismiss_staff(StaffType::Reroll, StaffQuantity::try_new(1).unwrap())
            .unwrap();
        let team = team.apply(&event);

        assert_eq!(team.rerolls.0, avant - 1);
    }

    #[test]
    fn dismiss_staff_refuse_le_facteur_fans() {
        let team = dismissals_phase_team();
        assert!(matches!(
            team.dismiss_staff(StaffType::FansFactor, StaffQuantity::try_new(1).unwrap()),
            Err(DomainError::StaffTypeNotDismissable)
        ));
    }

    // ── Mouvements de trésorerie (tests 24, 25 et 15) ────────────────────

    #[test]
    fn treasury_movement_staff_bought_est_un_debit_du_cout() {
        let team = recruitment_phase_team();
        let event = team
            .buy_staff(
                StaffType::Reroll,
                StaffQuantity::try_new(1).unwrap(),
                Kpo(60),
            )
            .unwrap();

        let m = team.treasury_movement(&event).expect("un achat débite");
        assert_eq!(m.amount, Kpo(60));
        assert_eq!(m.balance_after, Kpo(team.treasury.0 - 60));
    }

    #[test]
    fn treasury_movement_staff_dismissed_est_none() {
        let team = dismissals_phase_team();
        let event = team
            .dismiss_staff(StaffType::Assistant, StaffQuantity::try_new(1).unwrap())
            .unwrap();
        assert!(team.treasury_movement(&event).is_none());
    }
}
