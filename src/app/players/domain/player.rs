use crate::app::players::domain::error::DomainError;
use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::match_impact::{
    CasualtyCount, FoulCount, InjuryType, InterceptionCount, MatchContext, MatchReportId,
    MatchesPlayedCount, MvpCount, PassCount, PersistentInjuryCount, PlayerInjuryRecord,
    PlayerParticipationStatus, SppEarned, StatAdjustment, StatKind, StatMalus, TouchdownCount,
};
use crate::app::players::domain::value_objects::{
    CustomisationId, DisplayOrder, JerseyVo, KpoDelta, PersonalName, PositionNameVo, RosterLineId,
    SkillId, SkillName, SppAmount, SppCost, StatCrans,
};
use crate::app::shared_kernel::identity::ids::SpaceId;
use serde::{Deserialize, Serialize};

// ── Value objects ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spp(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueKpo(pub u32);

// ── Compétences acquises ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquiredSkill {
    pub skill_id: SkillId,
    pub skill_name: SkillName,
    pub mode: AcquisitionMode,
    pub spp_cost: SppCost,
    pub value_delta: ValueKpo,
    /// Le match qui l'a produite — `None` pour un achat ou une customisation.
    ///
    /// C'est ce qui rend la dépublication **exacte**. Compter « combien ce match
    /// en a ajouté » puis tronquer la fin de la liste ne serait vrai que si rien
    /// d'autre n'était venu après : une compétence accordée par un commissaire
    /// entre la publication et la dépublication serait retirée à la place de la
    /// Haine. L'origine portée par l'élément ne peut pas diverger de lui, un
    /// compteur si.
    pub from_match: Option<MatchReportId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcquisitionMode {
    Chosen,
    Random,
    /// Gagnée en encaissant un coup. Ce n'est pas `Automatic` : le coach répond
    /// à une question puis choisit parmi trente mots-clefs — c'est le geste le
    /// moins automatique de l'écran. Les trois autres modes nomment la façon
    /// d'obtenir ; celui-ci est « à la suite d'une blessure ».
    Injury,
    /// Donnée par un commissaire, hors des règles du jeu. C'est ce mode qui
    /// permet au journal des évolutions d'afficher sa pastille de customisation
    /// sans interroger l'event store.
    Customised,
}

// ── Augmentations de caractéristiques ───────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StatIncrease {
    pub stat: StatKind,
    pub spp_cost: SppCost,
    pub value_delta: ValueKpo,
}

/// Ajustement de caractéristique donné par un commissaire.
///
/// Séparé de `StatIncrease` et de `StatAdjustment` : ni un achat en SPP, ni une
/// séquelle de match. `offset` est **brut** et signé — il porte déjà le sens de
/// la caractéristique, traduit par `StatKind::improvement_step()` au moment de
/// la commande.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StatCustomisation {
    pub stat: StatKind,
    pub offset: i8, // arch:ok offset brut, déjà borné par le panier
}

// ── Agrégat Player ─────────────────────────────────────────────────────────────

/// Appartenance à l'effectif — un axe **distinct** de la participation.
///
/// `PlayerParticipationStatus` vit dans `match_impact.rs` et décrit ce qu'un
/// match a fait au joueur : disponible, absent, mort. L'appartenance, elle,
/// répond à une décision de coach — « ce joueur est-il encore de l'équipe ? »
/// — et c'est elle qui décide si le joueur figure dans l'effectif.
///
/// Les mêler reviendrait à faire d'un renvoyé un blessé de plus, qui
/// continuerait d'occuper sa place dans les quotas et le plafond de seize.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RosterMembership {
    Active,
    Dismissed,
}

impl RosterMembership {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Dismissed => "Dismissed",
        }
    }

    /// Tout ce qui n'est pas explicitement un renvoi est une appartenance :
    /// c'est le défaut de la colonne, et celui d'un agrégat rejoué.
    pub fn from_str(valeur: &str) -> Self {
        match valeur {
            "Dismissed" => Self::Dismissed,
            _ => Self::Active,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    pub id: PlayerId,
    pub team_id: TeamId,
    pub space_id: SpaceId,
    pub position_name: PositionNameVo,
    pub roster_line_id: RosterLineId,

    /// Nom donné par le coach, distinct du nom de poste. `None` tant qu'il n'a
    /// pas été saisi — la lecture retombe alors sur `position_name`.
    pub personal_name: Option<PersonalName>,
    pub jersey: Option<JerseyVo>,

    /// Rang libre dans l'effectif, posé par glisser-déposer. `None` tant que le
    /// joueur n'a jamais été réordonné : le tri retombe sur le maillot.
    pub display_order: Option<DisplayOrder>,
    pub base_skills: Vec<SkillId>,
    pub acquired_skills: Vec<AcquiredSkill>,
    pub stat_increases: Vec<StatIncrease>,

    /// Ajustements donnés par un commissaire, hors règles du jeu. Troisième
    /// source de caractéristiques après la base du poste et les deux
    /// précédentes — tenue à part pour que l'origine reste lisible.
    pub stat_customisations: Vec<StatCustomisation>,
    pub spp: Spp,
    pub value: ValueKpo,

    /// Hors du bloc ci-dessous, et c'est le sujet de la carte 260 : un renvoi
    /// est une décision de coach, pas une conséquence de match.
    pub membership: RosterMembership,

    // ── Impact des rapports de match ───────────────────────────────────────────
    pub participation_status: PlayerParticipationStatus,
    pub career_touchdowns: TouchdownCount,
    pub career_passes: PassCount,
    pub career_interceptions: InterceptionCount,
    pub career_casualties: CasualtyCount,
    pub career_mvps: MvpCount,
    pub career_fouls: FoulCount,
    pub career_persistent_injuries: PersistentInjuryCount,
    pub injuries: Vec<PlayerInjuryRecord>,
    pub stat_adjustments: Vec<StatAdjustment>,
    pub matches_played: MatchesPlayedCount,

    /// Ce que le dernier match a apporté à ce joueur, pour pouvoir le défaire
    /// si le rapport est dépublié pour correction.
    ///
    /// État **dérivé**, reconstruit à chaque rejeu : aucune migration. Seul le
    /// dernier match est corrigible (garde-fou « à chaud »), donc un seul
    /// accumulateur suffit.
    pub last_match: Option<LastMatchContribution>,

    /// Version courante de l'agrégat (nombre d'events déjà appliqués) — permet à
    /// l'appelant de connaître la prochaine version à utiliser pour `append()`,
    /// même pattern que `teams::Team::version`.
    pub version: i32, // arch:ok compteur technique d'event-sourcing, pas un concept domaine
}

/// Contributions d'un match à l'état du joueur — tout ce qu'une compensation
/// doit retrancher.
#[derive(Debug, Clone)]
pub struct LastMatchContribution {
    pub match_report_id: MatchReportId,
    pub spp_earned: Spp,
    pub touchdowns: u16,
    pub passes: u16,
    pub interceptions: u16,
    pub casualties: u16,
    pub mvps: u16,
    pub fouls: u16,
    pub matches_played: u16,
    pub injuries_added: usize,
    /// Compté explicitement plutôt que dérivé de `injuries` : `stat_adjustments`
    /// ne porte pas de `match_report_id`, et raisonner « les N derniers » ne
    /// serait vrai que tant qu'on ne défait que le dernier match.
    pub stat_adjustments_added: usize,
    pub persistent_injuries_added: u16,
    /// Statut **avant** tout événement de ce match. Sa définition n'est stable
    /// que depuis la carte 225 : une blessure subie en match n'est plus annulée
    /// par la conclusion de ce même match.
    pub participation_status_before: PlayerParticipationStatus,
}

impl LastMatchContribution {
    fn starting_from(
        match_report_id: MatchReportId,
        status_before: PlayerParticipationStatus,
    ) -> Self {
        Self {
            match_report_id,
            spp_earned: Spp(0),
            touchdowns: 0,
            passes: 0,
            interceptions: 0,
            casualties: 0,
            mvps: 0,
            fouls: 0,
            matches_played: 0,
            injuries_added: 0,
            stat_adjustments_added: 0,
            persistent_injuries_added: 0,
            participation_status_before: status_before,
        }
    }
}

impl Player {
    /// Reconstruit l'état de l'agrégat en rejouant une séquence d'events.
    /// Retourne `None` si la séquence est vide.
    pub fn from_events(events: &[PlayerDomainEvent]) -> Option<Self> {
        let mut state: Option<Self> = None;
        for event in events {
            state = Self::apply(state, event);
        }
        state
    }

    fn apply(current: Option<Self>, event: &PlayerDomainEvent) -> Option<Self> {
        match event {
            // Fait d'équipe, jamais persisté : le rejeu d'un joueur ne le
            // rencontre pas, et il ne modifierait rien de son état.
            PlayerDomainEvent::InitialRosterCompleted { .. } => current,
            PlayerDomainEvent::PlayerCreated {
                player_id,
                team_id,
                space_id,
                position_name,
                roster_line_id,
                jersey,
                base_skills,
                starting_spp,
                starting_value,
            } => {
                if current.is_some() {
                    return current;
                }
                Some(Self {
                    id: player_id.clone(),
                    team_id: team_id.clone(),
                    membership: RosterMembership::Active,
                    space_id: space_id.clone(),
                    position_name: position_name.clone(),
                    roster_line_id: roster_line_id.clone(),
                    personal_name: None,
                    jersey: *jersey,
                    display_order: None,
                    base_skills: base_skills.clone(),
                    acquired_skills: vec![],
                    stat_increases: vec![],
                    stat_customisations: vec![],
                    spp: *starting_spp,
                    value: *starting_value,
                    participation_status: PlayerParticipationStatus::Available,
                    career_touchdowns: TouchdownCount::default(),
                    career_passes: PassCount::default(),
                    career_interceptions: InterceptionCount::default(),
                    career_casualties: CasualtyCount::default(),
                    career_mvps: MvpCount::default(),
                    career_fouls: FoulCount::default(),
                    career_persistent_injuries: PersistentInjuryCount::default(),
                    injuries: vec![],
                    stat_adjustments: vec![],
                    last_match: None,
                    matches_played: MatchesPlayedCount::default(),
                    version: 1,
                })
            }
            PlayerDomainEvent::InitialSkillEarned {
                skill_id,
                skill_name,
                mode,
                spp_cost,
                value_delta,
                ..
            } => {
                let mut player = current?;
                player.acquired_skills.push(AcquiredSkill {
                    skill_id: skill_id.clone(),
                    skill_name: skill_name.clone(),
                    mode: *mode,
                    spp_cost: *spp_cost,
                    value_delta: *value_delta,
                    // Un achat en SPP ne vient d'aucun match.
                    from_match: None,
                });
                player.value = ValueKpo(player.value.0 + value_delta.0);
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::PlayerSkillPurchased {
                skill_id,
                skill_name,
                mode,
                spp_cost,
                value_delta,
                ..
            } => {
                let mut player = current?;
                player.acquired_skills.push(AcquiredSkill {
                    skill_id: skill_id.clone(),
                    skill_name: skill_name.clone(),
                    mode: *mode,
                    spp_cost: *spp_cost,
                    value_delta: *value_delta,
                    // Un achat en SPP ne vient d'aucun match.
                    from_match: None,
                });
                player.value = ValueKpo(player.value.0 + value_delta.0);
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::PlayerStatIncreased {
                stat,
                spp_cost,
                value_delta,
                ..
            } => {
                let mut player = current?;
                player.stat_increases.push(StatIncrease {
                    stat: *stat,
                    spp_cost: *spp_cost,
                    value_delta: *value_delta,
                });
                player.value = ValueKpo(player.value.0 + value_delta.0);
                player.version += 1;
                Some(player)
            }

            PlayerDomainEvent::TouchdownScored {
                context,
                spp_earned,
                ..
            } => {
                let mut player = current?;
                player.begin_match(&context.match_report_id);
                player.spp = Spp(player.spp.0 + spp_earned.into_inner());
                player.career_touchdowns.0 += 1;
                let c = player.contribution();
                c.spp_earned = Spp(c.spp_earned.0 + spp_earned.into_inner());
                c.touchdowns += 1;
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::PassCompleted {
                context,
                spp_earned,
                ..
            } => {
                let mut player = current?;
                player.begin_match(&context.match_report_id);
                player.spp = Spp(player.spp.0 + spp_earned.into_inner());
                player.career_passes.0 += 1;
                let c = player.contribution();
                c.spp_earned = Spp(c.spp_earned.0 + spp_earned.into_inner());
                c.passes += 1;
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::InterceptionMade {
                context,
                spp_earned,
                ..
            } => {
                let mut player = current?;
                player.begin_match(&context.match_report_id);
                player.spp = Spp(player.spp.0 + spp_earned.into_inner());
                player.career_interceptions.0 += 1;
                let c = player.contribution();
                c.spp_earned = Spp(c.spp_earned.0 + spp_earned.into_inner());
                c.interceptions += 1;
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::CasualtyInflicted {
                context,
                spp_earned,
                ..
            } => {
                let mut player = current?;
                player.begin_match(&context.match_report_id);
                player.spp = Spp(player.spp.0 + spp_earned.into_inner());
                player.career_casualties.0 += 1;
                let c = player.contribution();
                c.spp_earned = Spp(c.spp_earned.0 + spp_earned.into_inner());
                c.casualties += 1;
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::MatchMvpNamed {
                context,
                spp_earned,
                ..
            } => {
                let mut player = current?;
                player.begin_match(&context.match_report_id);
                player.spp = Spp(player.spp.0 + spp_earned.into_inner());
                player.career_mvps.0 += 1;
                let c = player.contribution();
                c.spp_earned = Spp(c.spp_earned.0 + spp_earned.into_inner());
                c.mvps += 1;
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::FoulCommitted { context, .. } => {
                let mut player = current?;
                player.begin_match(&context.match_report_id);
                player.career_fouls.0 += 1;
                player.contribution().fouls += 1;
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::InjurySustained {
                context,
                injury_type,
                ..
            } => {
                let mut player = current?;
                player.begin_match(&context.match_report_id);
                player.contribution().injuries_added += 1;
                player.injuries.push(PlayerInjuryRecord {
                    injury_type: injury_type.clone(),
                    context: context.clone(),
                });
                match injury_type {
                    InjuryType::Commotion => {}
                    InjuryType::Mort => {
                        player.participation_status = PlayerParticipationStatus::Dead;
                    }
                    InjuryType::BlessureSerieuse => {
                        player.participation_status = PlayerParticipationStatus::MissingNextGame;
                        player.career_persistent_injuries.0 += 1;
                        player.contribution().persistent_injuries_added += 1;
                    }
                    InjuryType::Amoche => {
                        player.participation_status = PlayerParticipationStatus::MissingNextGame;
                    }
                    InjuryType::Sequel { stat } => {
                        player.participation_status = PlayerParticipationStatus::MissingNextGame;
                        player.stat_adjustments.push(StatAdjustment {
                            stat: *stat,
                            malus: StatMalus::try_new(1).unwrap(),
                        });
                        player.contribution().stat_adjustments_added += 1;
                    }
                }
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::PlayerAvailabilityRestored {
                match_report_id, ..
            } => {
                let mut player = current?;
                player.begin_match(match_report_id);
                if player.participation_status == PlayerParticipationStatus::MissingNextGame {
                    player.participation_status = PlayerParticipationStatus::Available;
                }
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::MatchConcluded { context, .. } => {
                let mut player = current?;
                player.begin_match(&context.match_report_id);
                player.matches_played.0 += 1;
                player.contribution().matches_played += 1;
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::MatchImpactReverted {
                match_report_id, ..
            } => {
                let mut player = current?;
                player.revert_last_match(match_report_id);
                player.version += 1;
                Some(player)
            }
            // Le joueur n'est pas effacé : il garde ses SPP, ses compétences et
            // son historique. Seule son appartenance change, et c'est elle que
            // les lectures d'effectif regardent.
            PlayerDomainEvent::PlayerDismissed { .. } => {
                let mut player = current?;
                player.membership = RosterMembership::Dismissed;
                player.version += 1;
                Some(player)
            }

            // Les trois éditions du coach. `None` n'est pas un « pas de
            // changement » mais un effacement demandé : on écrase sans condition.
            PlayerDomainEvent::PlayerRenamed { personal_name, .. } => {
                let mut player = current?;
                player.personal_name = personal_name.clone();
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::PlayerJerseyChanged { jersey, .. } => {
                let mut player = current?;
                player.jersey = *jersey;
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::PlayerReordered { display_order, .. } => {
                let mut player = current?;
                player.display_order = Some(*display_order);
                player.version += 1;
                Some(player)
            }

            // ── Customisation ─────────────────────────────────────────────────
            // La compétence rejoint les acquises, mais **sans valeur** : seul le
            // prix déplace la valeur d'équipe. Une compétence donnée par un
            // commissaire ne renchérit pas le joueur, contrairement à la même
            // achetée en SPP.
            // ── Haine ─────────────────────────────────────────────────────────
            // Un trait gagné en encaissant un coup : ni coût, ni valeur. C'est
            // la même distinction que la customisation — l'événement ne porte
            // pas de champ de valeur, l'état projeté porte des zéros.
            //
            // `from_match` est ce qui rend la dépublication exacte : on retirera
            // ce que ce match a produit, sans dépendre de l'ordre de la liste.
            PlayerDomainEvent::PlayerHatredGained {
                context,
                skill_id,
                skill_name,
                ..
            } => {
                let mut player = current?;
                player.acquired_skills.push(AcquiredSkill {
                    skill_id: skill_id.clone(),
                    skill_name: skill_name.clone(),
                    mode: AcquisitionMode::Injury,
                    spp_cost: SppCost::try_new(0).unwrap(),
                    value_delta: ValueKpo(0),
                    from_match: Some(context.match_report_id.clone()),
                });
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::PlayerSkillCustomised {
                skill_id,
                skill_name,
                ..
            } => {
                let mut player = current?;
                player.acquired_skills.push(AcquiredSkill {
                    skill_id: skill_id.clone(),
                    skill_name: skill_name.clone(),
                    mode: AcquisitionMode::Customised,
                    // Ni coût ni valeur : une compétence donnée par un
                    // commissaire ne se paie pas et ne renchérit pas le joueur.
                    spp_cost: SppCost::try_new(0).unwrap(),
                    value_delta: ValueKpo(0),
                    // Une customisation ne vient d'aucun match : rien à défaire
                    // à la dépublication d'un rapport.
                    from_match: None,
                });
                player.version += 1;
                Some(player)
            }
            // L'offset est déjà brut et déjà borné : le panier l'a validé avant
            // que l'événement n'existe. `apply` enregistre, il ne juge pas.
            PlayerDomainEvent::PlayerStatCustomised { stat, offset, .. } => {
                let mut player = current?;
                player.stat_customisations.push(StatCustomisation {
                    stat: *stat,
                    offset: *offset,
                });
                player.version += 1;
                Some(player)
            }
            // Même arithmétique que la customisation : un delta signé, borné
            // à zéro. Une valeur ne descend pas sous zéro, même si un
            // commissaire l'avait déjà baissée sous son barème.
            PlayerDomainEvent::PlayerValueRecalibrated { delta, .. } => {
                let mut player = current?;
                let resultat = player.value.0 as i64 + delta.into_inner() as i64;
                player.value = ValueKpo(resultat.max(0) as u32);
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::PlayerValueCustomised { delta, .. } => {
                let mut player = current?;
                let resultat = player.value.0 as i64 + delta.into_inner() as i64;
                player.value = ValueKpo(resultat.max(0) as u32);
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::PlayerSppCustomised { amount, .. } => {
                let mut player = current?;
                player.spp = Spp(player.spp.0 + amount.into_inner() as u32);
                player.version += 1;
                Some(player)
            }
        }
    }

    /// Ouvre l'accumulateur si cet événement appartient à un autre match que
    /// celui en cours. Appelé **avant** toute mutation, sans quoi
    /// `participation_status_before` capturerait l'état d'après.
    fn begin_match(&mut self, match_report_id: &MatchReportId) {
        let already_current =
            self.last_match.as_ref().map(|m| &m.match_report_id) == Some(match_report_id);
        if !already_current {
            self.last_match = Some(LastMatchContribution::starting_from(
                match_report_id.clone(),
                self.participation_status,
            ));
        }
    }

    fn contribution(&mut self) -> &mut LastMatchContribution {
        self.last_match
            .as_mut()
            .expect("begin_match doit être appelé avant toute accumulation")
    }

    /// Retranche l'impact du dernier match. Sans effet si l'instantané concerne
    /// un autre match — c'est ce qui rend la compensation idempotente.
    fn revert_last_match(&mut self, match_report_id: &MatchReportId) {
        let Some(c) = self.last_match.take() else {
            return;
        };
        if &c.match_report_id != match_report_id {
            self.last_match = Some(c);
            return;
        }

        self.subtract_counters(&c);

        // Les blessures de ce match sont les dernières ajoutées, donc en fin de
        // liste — on ne défait que le dernier match.
        truncate_by(&mut self.injuries, c.injuries_added);
        truncate_by(&mut self.stat_adjustments, c.stat_adjustments_added);

        // Les compétences se retirent par **origine**, pas par position. Compter
        // « combien ce match en a ajouté » puis tronquer la fin ne serait vrai
        // que si rien d'autre n'était venu après : une compétence accordée par
        // un commissaire entre la publication et la dépublication serait retirée
        // à sa place. `has_spent_spp_since_match` couvre l'achat en SPP, rien ne
        // couvre la customisation.
        self.acquired_skills
            .retain(|s| s.from_match.as_ref() != Some(match_report_id));

        self.participation_status = c.participation_status_before;
    }

    fn subtract_counters(&mut self, c: &LastMatchContribution) {
        self.spp = Spp(self.spp.0.saturating_sub(c.spp_earned.0));
        self.career_touchdowns.0 = self.career_touchdowns.0.saturating_sub(c.touchdowns);
        self.career_passes.0 = self.career_passes.0.saturating_sub(c.passes);
        self.career_interceptions.0 = self.career_interceptions.0.saturating_sub(c.interceptions);
        self.career_casualties.0 = self.career_casualties.0.saturating_sub(c.casualties);
        self.career_mvps.0 = self.career_mvps.0.saturating_sub(c.mvps);
        self.career_fouls.0 = self.career_fouls.0.saturating_sub(c.fouls);
        self.matches_played.0 = self.matches_played.0.saturating_sub(c.matches_played);
        self.career_persistent_injuries.0 = self
            .career_persistent_injuries
            .0
            .saturating_sub(c.persistent_injuries_added);
    }

    // ── Méthodes de commande — infaillibles, aucune garde métier (BR14) ─────────
    // Ne construisent que l'event : toute la logique vit dans apply() ci-dessus.

    pub fn record_touchdown(
        &self,
        context: MatchContext,
        spp_earned: SppEarned,
    ) -> PlayerDomainEvent {
        PlayerDomainEvent::TouchdownScored {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            context,
            spp_earned,
        }
    }
    pub fn record_pass(&self, context: MatchContext, spp_earned: SppEarned) -> PlayerDomainEvent {
        PlayerDomainEvent::PassCompleted {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            context,
            spp_earned,
        }
    }
    pub fn record_interception(
        &self,
        context: MatchContext,
        spp_earned: SppEarned,
    ) -> PlayerDomainEvent {
        PlayerDomainEvent::InterceptionMade {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            context,
            spp_earned,
        }
    }
    pub fn record_casualty(
        &self,
        context: MatchContext,
        spp_earned: SppEarned,
    ) -> PlayerDomainEvent {
        PlayerDomainEvent::CasualtyInflicted {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            context,
            spp_earned,
        }
    }
    pub fn record_mvp(&self, context: MatchContext, spp_earned: SppEarned) -> PlayerDomainEvent {
        PlayerDomainEvent::MatchMvpNamed {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            context,
            spp_earned,
        }
    }
    pub fn record_foul(&self, context: MatchContext) -> PlayerDomainEvent {
        PlayerDomainEvent::FoulCommitted {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            context,
        }
    }
    pub fn record_injury(
        &self,
        context: MatchContext,
        injury_type: InjuryType,
    ) -> PlayerDomainEvent {
        PlayerDomainEvent::InjurySustained {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            context,
            injury_type,
        }
    }
    /// Le joueur se met à haïr une espèce.
    ///
    /// Pas de `Result` : tout est vérifié en amont — le type de blessure par le
    /// domaine de `match_report`, l'existence du mot-clef par le use case de
    /// saisie, celle de la compétence par le listener. Une méthode qui ne peut
    /// pas échouer ferait écrire des `unwrap` à ses appelants.
    ///
    /// Aucune règle de doublon : haïr deux fois la même espèce est possible, et
    /// c'est au règlement de le dire, pas à ce greffier.
    pub fn record_hatred(
        &self,
        context: MatchContext,
        skill_id: SkillId,
        skill_name: SkillName,
    ) -> PlayerDomainEvent {
        PlayerDomainEvent::PlayerHatredGained {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            context,
            skill_id,
            skill_name,
        }
    }

    pub fn restore_availability(&self, match_report_id: MatchReportId) -> PlayerDomainEvent {
        PlayerDomainEvent::PlayerAvailabilityRestored {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            match_report_id,
        }
    }
    pub fn record_match_concluded(
        &self,
        context: MatchContext,
        team_score: u8,
        opponent_score: u8,
    ) -> PlayerDomainEvent {
        PlayerDomainEvent::MatchConcluded {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            context,
            team_score,
            opponent_score,
        }
    }

    /// Annule l'impact de ce match sur ce joueur.
    ///
    /// `None` si le dernier match enregistré n'est pas celui-ci : le joueur n'a
    /// rien à défaire. C'est à la fois l'idempotence et ce qui permet au
    /// listener d'itérer sur tout l'effectif sans savoir qui a joué.
    pub fn revert_match_impact(
        &self,
        match_report_id: &MatchReportId,
    ) -> Option<PlayerDomainEvent> {
        let last = self.last_match.as_ref()?;
        if &last.match_report_id != match_report_id {
            return None;
        }
        Some(PlayerDomainEvent::MatchImpactReverted {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            match_report_id: match_report_id.clone(),
        })
    }

    // ── Dépense de SPP (phase PlayerImprovement) — méthodes faillibles ──────────
    // Exception à BR14 : ces 2 méthodes portent de vraies gardes métier (SPP
    // insuffisant, compétence déjà possédée), contrairement aux méthodes
    // ci-dessus qui ne font qu'enregistrer des faits déjà validés par match_report.

    /// SPP encore disponibles — dérivé, jamais stocké (cohérent avec l'event sourcing).
    pub fn spp_remaining(&self) -> u32 {
        let spent: u32 = self
            .acquired_skills
            .iter()
            .map(|s| s.spp_cost.into_inner() as u32)
            .sum::<u32>()
            + self
                .stat_increases
                .iter()
                .map(|s| s.spp_cost.into_inner() as u32)
                .sum::<u32>();
        self.spp.0.saturating_sub(spent)
    }

    /// Niveau de la prochaine amélioration dans la matrice de coût — compteur
    /// unique partagé entre compétences et caractéristiques, plafonné à 6.
    pub fn next_improvement_level(&self) -> u8 {
        ((self.acquired_skills.len() + self.stat_increases.len()) as u8 + 1).min(6)
    }

    pub fn purchase_skill(
        &self,
        skill_id: SkillId,
        skill_name: SkillName,
        category_css: String,
        mode: AcquisitionMode,
        spp_cost: SppCost,
        value_delta: ValueKpo,
    ) -> Result<PlayerDomainEvent, DomainError> {
        let already_acquired = self.base_skills.contains(&skill_id)
            || self.acquired_skills.iter().any(|s| s.skill_id == skill_id);
        if already_acquired {
            return Err(DomainError::SkillAlreadyAcquired);
        }
        if self.spp_remaining() < spp_cost.into_inner() as u32 {
            return Err(DomainError::InsufficientSpp);
        }
        Ok(PlayerDomainEvent::PlayerSkillPurchased {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            skill_id,
            skill_name,
            category_css,
            mode,
            spp_cost,
            value_delta,
        })
    }

    pub fn increase_stat(
        &self,
        stat: StatKind,
        spp_cost: SppCost,
        value_delta: ValueKpo,
    ) -> Result<PlayerDomainEvent, DomainError> {
        if self.spp_remaining() < spp_cost.into_inner() as u32 {
            return Err(DomainError::InsufficientSpp);
        }
        Ok(PlayerDomainEvent::PlayerStatIncreased {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            stat,
            spp_cost,
            value_delta,
        })
    }

    // ── Édition de l'effectif par le coach ─────────────────────────────────────
    // Un joueur renvoyé n'est plus modifiable : il a quitté l'effectif, et son
    // maillot doit pouvoir être réattribué sans qu'il le dispute. C'est la seule
    // règle des trois — l'unicité du numéro et de l'ordre porte sur l'effectif
    // entier, qu'un agrégat isolé ne connaît pas ; elle revient au use case.

    pub fn rename(
        &self,
        personal_name: Option<PersonalName>,
    ) -> Result<PlayerDomainEvent, DomainError> {
        self.guard_active()?;
        Ok(PlayerDomainEvent::PlayerRenamed {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            personal_name,
        })
    }

    pub fn change_jersey(
        &self,
        jersey: Option<JerseyVo>,
    ) -> Result<PlayerDomainEvent, DomainError> {
        self.guard_active()?;
        Ok(PlayerDomainEvent::PlayerJerseyChanged {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            jersey,
        })
    }

    pub fn reorder(&self, display_order: DisplayOrder) -> Result<PlayerDomainEvent, DomainError> {
        self.guard_active()?;
        Ok(PlayerDomainEvent::PlayerReordered {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            display_order,
        })
    }

    fn guard_active(&self) -> Result<(), DomainError> {
        match self.membership {
            RosterMembership::Active => Ok(()),
            RosterMembership::Dismissed => Err(DomainError::PlayerNotActive),
        }
    }

    // ── Customisation par un commissaire ───────────────────────────────────────
    //
    // **Aucune garde.** Ni de phase, ni d'appartenance : les customisations
    // s'appliquent toujours, un joueur renvoyé reste customisable. C'est ce qui
    // les distingue de `rename`, qui exige `membership == Active`.
    //
    // Les invariants de validité — bornes, doublon de compétence, prix plancher,
    // plafond de SPP — ont déjà été joués par l'agrégat panier. Le panier est le
    // gardien, `Player` est le greffier : revérifier ici dédoublerait la règle
    // et la ferait diverger.

    pub fn customise_skill(
        &self,
        customisation_id: CustomisationId,
        skill_id: SkillId,
        skill_name: SkillName,
        author: String,
    ) -> Result<PlayerDomainEvent, DomainError> {
        Ok(PlayerDomainEvent::PlayerSkillCustomised {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            customisation_id,
            skill_id,
            skill_name,
            author,
        })
    }

    /// `crans` porte le sens en **qualité du joueur** ; la traduction en offset
    /// brut est faite ici, seul endroit qui détient la table des directions.
    /// C'est l'offset qui part dans l'événement : un rejeu ne doit dépendre
    /// d'aucune convention externe.
    pub fn customise_stat(
        &self,
        customisation_id: CustomisationId,
        stat: StatKind,
        crans: StatCrans,
        author: String,
    ) -> Result<PlayerDomainEvent, DomainError> {
        let offset = crans.into_inner() * stat.improvement_step();
        Ok(PlayerDomainEvent::PlayerStatCustomised {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            customisation_id,
            stat,
            offset,
            author,
        })
    }

    pub fn customise_value(
        &self,
        customisation_id: CustomisationId,
        delta: KpoDelta,
        author: String,
    ) -> Result<PlayerDomainEvent, DomainError> {
        Ok(PlayerDomainEvent::PlayerValueCustomised {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            customisation_id,
            delta,
            author,
        })
    }

    pub fn customise_spp(
        &self,
        customisation_id: CustomisationId,
        amount: SppAmount,
        author: String,
    ) -> Result<PlayerDomainEvent, DomainError> {
        Ok(PlayerDomainEvent::PlayerSppCustomised {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            customisation_id,
            amount,
            author,
        })
    }
}

#[cfg(test)]
mod match_impact_tests {
    use super::*;
    use crate::app::players::domain::match_impact::{RoundId, StatKind};

    fn sample_player() -> Player {
        let created = PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
            jersey: None,
            base_skills: vec![],
            starting_spp: Spp(0),
            starting_value: ValueKpo(100),
        };
        Player::from_events(&[created]).unwrap()
    }

    fn sample_context() -> MatchContext {
        MatchContext {
            match_report_id: MatchReportId("mr1".into()),
            round_id: RoundId("r1".into()),
            round_label: "Journée 5".into(),
            opponent_team_id: TeamId("opponent".into()),
            opponent_team_name: "Bone Crushers".into(),
        }
    }

    #[test]
    fn touchdown_credits_spp_and_increments_counter() {
        let player = sample_player();
        let event = player.record_touchdown(sample_context(), SppEarned::try_new(3).unwrap());
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.spp.0, 3);
        assert_eq!(player.career_touchdowns.0, 1);
    }

    #[test]
    fn foul_increments_counter_without_spp() {
        let player = sample_player();
        let event = player.record_foul(sample_context());
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.spp.0, 0);
        assert_eq!(player.career_fouls.0, 1);
    }

    #[test]
    fn commotion_is_logged_without_status_or_counter_change() {
        let player = sample_player();
        let event = player.record_injury(sample_context(), InjuryType::Commotion);
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.injuries.len(), 1);
        assert_eq!(
            player.participation_status,
            PlayerParticipationStatus::Available
        );
        assert_eq!(player.career_persistent_injuries.0, 0);
        assert!(player.stat_adjustments.is_empty());
    }

    #[test]
    fn death_sets_dead_status() {
        let player = sample_player();
        let event = player.record_injury(sample_context(), InjuryType::Mort);
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.participation_status, PlayerParticipationStatus::Dead);
    }

    #[test]
    fn serious_injury_sets_missing_next_game_and_increments_persistent_counter() {
        let player = sample_player();
        let event = player.record_injury(sample_context(), InjuryType::BlessureSerieuse);
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(
            player.participation_status,
            PlayerParticipationStatus::MissingNextGame
        );
        assert_eq!(player.career_persistent_injuries.0, 1);
        assert!(player.stat_adjustments.is_empty());
    }

    #[test]
    fn amoche_sets_missing_next_game_without_counter_or_adjustment() {
        let player = sample_player();
        let event = player.record_injury(sample_context(), InjuryType::Amoche);
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(
            player.participation_status,
            PlayerParticipationStatus::MissingNextGame
        );
        assert_eq!(player.career_persistent_injuries.0, 0);
        assert!(player.stat_adjustments.is_empty());
    }

    #[test]
    fn sequel_sets_missing_next_game_and_adds_stat_adjustment_without_persistent_counter() {
        let player = sample_player();
        let event =
            player.record_injury(sample_context(), InjuryType::Sequel { stat: StatKind::Ag });
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(
            player.participation_status,
            PlayerParticipationStatus::MissingNextGame
        );
        assert_eq!(player.career_persistent_injuries.0, 0);
        assert_eq!(player.stat_adjustments.len(), 1);
        assert_eq!(player.stat_adjustments[0].stat, StatKind::Ag);
    }

    #[test]
    fn availability_restored_only_changes_missing_next_game_players() {
        let player = sample_player();
        assert_eq!(
            player.participation_status,
            PlayerParticipationStatus::Available
        );
        let event = player.restore_availability(MatchReportId("mr2".into()));
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(
            player.participation_status,
            PlayerParticipationStatus::Available
        );
    }

    #[test]
    fn availability_restored_lifts_missing_next_game_to_available() {
        let player = sample_player();
        let injury_event = player.record_injury(sample_context(), InjuryType::Amoche);
        let player = Player::apply(Some(player), &injury_event).unwrap();
        assert_eq!(
            player.participation_status,
            PlayerParticipationStatus::MissingNextGame
        );

        let event = player.restore_availability(MatchReportId("mr2".into()));
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(
            player.participation_status,
            PlayerParticipationStatus::Available
        );
    }

    #[test]
    fn availability_restored_does_not_affect_dead_players() {
        let player = sample_player();
        let injury_event = player.record_injury(sample_context(), InjuryType::Mort);
        let player = Player::apply(Some(player), &injury_event).unwrap();
        assert_eq!(player.participation_status, PlayerParticipationStatus::Dead);

        let event = player.restore_availability(MatchReportId("mr2".into()));
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.participation_status, PlayerParticipationStatus::Dead);
    }

    #[test]
    fn match_concluded_increments_matches_played() {
        let player = sample_player();
        assert_eq!(player.matches_played.0, 0);
        let event = player.record_match_concluded(sample_context(), 2, 1);
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.matches_played.0, 1);
    }

    #[test]
    fn match_concluded_does_not_affect_other_counters() {
        let player = sample_player();
        let event = player.record_match_concluded(sample_context(), 2, 1);
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.career_touchdowns.0, 0);
        assert_eq!(player.spp.0, 0);
        assert_eq!(
            player.participation_status,
            PlayerParticipationStatus::Available
        );
    }
}

#[cfg(test)]
mod improvement_tests {
    use super::*;
    use crate::app::players::domain::match_impact::StatKind;

    fn sample_player_with_spp(spp: u32) -> Player {
        let created = PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
            jersey: None,
            base_skills: vec![SkillId::try_new("existing-base").unwrap()],
            starting_spp: Spp(spp),
            starting_value: ValueKpo(100),
        };
        Player::from_events(&[created]).unwrap()
    }

    fn skill(id: &str) -> SkillId {
        SkillId::try_new(id).unwrap()
    }
    fn name(n: &str) -> SkillName {
        SkillName::try_new(n).unwrap()
    }

    #[test]
    fn purchase_skill_nominal_appends_skill_and_credits_value() {
        let player = sample_player_with_spp(50);
        let event = player
            .purchase_skill(
                skill("block"),
                name("Bloc"),
                "type-general".into(),
                AcquisitionMode::Chosen,
                SppCost::try_new(6).unwrap(),
                ValueKpo(20),
            )
            .unwrap();
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.acquired_skills.len(), 1);
        assert_eq!(player.value.0, 120);
        assert_eq!(player.spp_remaining(), 44);
    }

    #[test]
    fn purchase_already_base_skill_is_rejected() {
        let player = sample_player_with_spp(50);
        let result = player.purchase_skill(
            skill("existing-base"),
            name("Existant"),
            "type-general".into(),
            AcquisitionMode::Chosen,
            SppCost::try_new(6).unwrap(),
            ValueKpo(20),
        );
        assert!(matches!(result, Err(DomainError::SkillAlreadyAcquired)));
    }

    #[test]
    fn purchase_already_acquired_skill_is_rejected() {
        let player = sample_player_with_spp(50);
        let event = player
            .purchase_skill(
                skill("block"),
                name("Bloc"),
                "type-general".into(),
                AcquisitionMode::Chosen,
                SppCost::try_new(6).unwrap(),
                ValueKpo(20),
            )
            .unwrap();
        let player = Player::apply(Some(player), &event).unwrap();

        let result = player.purchase_skill(
            skill("block"),
            name("Bloc"),
            "type-general".into(),
            AcquisitionMode::Chosen,
            SppCost::try_new(6).unwrap(),
            ValueKpo(20),
        );
        assert!(matches!(result, Err(DomainError::SkillAlreadyAcquired)));
    }

    #[test]
    fn purchase_skill_insufficient_spp_is_rejected() {
        let player = sample_player_with_spp(5);
        let result = player.purchase_skill(
            skill("block"),
            name("Bloc"),
            "type-general".into(),
            AcquisitionMode::Chosen,
            SppCost::try_new(6).unwrap(),
            ValueKpo(20),
        );
        assert!(matches!(result, Err(DomainError::InsufficientSpp)));
    }

    #[test]
    fn increase_stat_nominal_credits_value_and_spends_spp() {
        let player = sample_player_with_spp(20);
        let event = player
            .increase_stat(StatKind::Ma, SppCost::try_new(14).unwrap(), ValueKpo(20))
            .unwrap();
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.stat_increases.len(), 1);
        assert_eq!(player.value.0, 120);
        assert_eq!(player.spp_remaining(), 6);
    }

    #[test]
    fn increase_stat_insufficient_spp_is_rejected() {
        let player = sample_player_with_spp(5);
        let result =
            player.increase_stat(StatKind::St, SppCost::try_new(14).unwrap(), ValueKpo(60));
        assert!(matches!(result, Err(DomainError::InsufficientSpp)));
    }

    #[test]
    fn next_improvement_level_counts_skills_and_stats_together_capped_at_6() {
        let mut player = sample_player_with_spp(1000);
        for i in 0u8..8 {
            assert_eq!(player.next_improvement_level(), (i + 1).min(6));
            let event = if i % 2 == 0 {
                player
                    .purchase_skill(
                        skill(&format!("s{i}")),
                        name(&format!("Skill{i}")),
                        "type-general".into(),
                        AcquisitionMode::Chosen,
                        SppCost::try_new(1).unwrap(),
                        ValueKpo(0),
                    )
                    .unwrap()
            } else {
                player
                    .increase_stat(StatKind::Pa, SppCost::try_new(1).unwrap(), ValueKpo(0))
                    .unwrap()
            };
            player = Player::apply(Some(player), &event).unwrap();
        }
        assert_eq!(player.next_improvement_level(), 6);
    }

    #[test]
    fn spp_remaining_accounts_for_mixed_skill_and_stat_spending() {
        let player = sample_player_with_spp(100);
        let skill_event = player
            .purchase_skill(
                skill("block"),
                name("Bloc"),
                "type-general".into(),
                AcquisitionMode::Chosen,
                SppCost::try_new(6).unwrap(),
                ValueKpo(0),
            )
            .unwrap();
        let player = Player::apply(Some(player), &skill_event).unwrap();
        let stat_event = player
            .increase_stat(StatKind::Ag, SppCost::try_new(14).unwrap(), ValueKpo(0))
            .unwrap();
        let player = Player::apply(Some(player), &stat_event).unwrap();
        assert_eq!(player.spp_remaining(), 80);
    }
}

/// Retire les `count` derniers éléments — les contributions du dernier match
/// sont toujours en fin de liste.
fn truncate_by<T>(items: &mut Vec<T>, count: usize) {
    let keep = items.len().saturating_sub(count);
    items.truncate(keep);
}

#[cfg(test)]
mod revert_match_impact_tests {
    use super::*;
    use crate::app::players::domain::match_impact::RoundId;

    fn context(match_report_id: &str) -> MatchContext {
        MatchContext {
            match_report_id: MatchReportId(match_report_id.into()),
            round_id: RoundId("r1".into()),
            round_label: "Journée 5".into(),
            opponent_team_id: TeamId("adversaire".into()),
            opponent_team_name: "Bone Crushers".into(),
        }
    }

    fn created() -> PlayerDomainEvent {
        PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
            jersey: None,
            base_skills: vec![],
            starting_spp: Spp(0),
            starting_value: ValueKpo(100),
        }
    }

    fn spp(n: u32) -> SppEarned {
        SppEarned::try_new(n).unwrap()
    }

    // ── Haine (carte 403) ────────────────────────────────────────────────────

    fn haine(mr: &str, uid: &str) -> PlayerDomainEvent {
        PlayerDomainEvent::PlayerHatredGained {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            context: context(mr),
            skill_id: SkillId::try_new(uid.to_string()).unwrap(),
            skill_name: SkillName::try_new(format!("Haine : {uid}")).unwrap(),
        }
    }

    fn joueur(events: Vec<PlayerDomainEvent>) -> Player {
        let mut tous = vec![created()];
        tous.extend(events);
        Player::from_events(&tous).expect("le joueur doit se reconstituer")
    }

    #[test]
    fn une_haine_est_gratuite_et_ne_renchérit_pas_le_joueur() {
        let p = joueur(vec![haine("mr-1", "HAINE_DWARF")]);
        assert_eq!(p.acquired_skills.len(), 1);
        let s = &p.acquired_skills[0];
        assert_eq!(s.spp_cost.into_inner(), 0);
        assert_eq!(s.value_delta.0, 0);
        assert!(matches!(s.mode, AcquisitionMode::Injury));
        assert_eq!(s.from_match, Some(MatchReportId("mr-1".into())));
    }

    /// Le joueur encaisse, il ne gagne rien : sa réserve ne bouge pas.
    #[test]
    fn une_haine_ne_touche_pas_la_reserve_de_spp() {
        let avant = joueur(vec![]).spp_remaining();
        let apres = joueur(vec![haine("mr-1", "HAINE_DWARF")]).spp_remaining();
        assert_eq!(avant, apres);
    }

    /// Aucune règle de doublon : c'est au règlement de le dire, pas au greffier.
    #[test]
    fn hair_deux_fois_la_meme_espece_est_accepte() {
        let p = joueur(vec![
            haine("mr-1", "HAINE_DWARF"),
            haine("mr-2", "HAINE_DWARF"),
        ]);
        assert_eq!(p.acquired_skills.len(), 2);
    }

    #[test]
    fn trois_haines_differentes_se_cumulent() {
        let p = joueur(vec![
            haine("mr-1", "HAINE_DWARF"),
            haine("mr-1", "HAINE_ELF"),
            haine("mr-1", "HAINE_SKAVEN"),
        ]);
        assert_eq!(p.acquired_skills.len(), 3);
    }

    /// Le cœur de la carte : la dépublication retire la Haine de ce match, et
    /// **elle seule**.
    #[test]
    fn la_depublication_defait_la_haine_du_match() {
        let mut events = match_events("mr-1");
        events.push(haine("mr-1", "HAINE_DWARF"));
        let p = joueur(events);
        assert_eq!(p.acquired_skills.len(), 1);

        let revert = p
            .revert_match_impact(&MatchReportId("mr-1".into()))
            .unwrap();
        let mut tous = vec![created()];
        tous.extend(match_events("mr-1"));
        tous.push(haine("mr-1", "HAINE_DWARF"));
        tous.push(revert);
        let apres = Player::from_events(&tous).unwrap();
        assert!(
            apres.acquired_skills.is_empty(),
            "la Haine doit partir avec l'impact du match"
        );
    }

    /// Le scénario qui condamnait le compteur : une compétence arrivée **après**
    /// la Haine, par un autre chemin, ne doit pas être retirée à sa place.
    #[test]
    fn la_depublication_epargne_une_competence_d_une_autre_origine() {
        let mut tous = vec![created()];
        tous.extend(match_events("mr-1"));
        tous.push(haine("mr-1", "HAINE_DWARF"));
        tous.push(PlayerDomainEvent::PlayerSkillCustomised {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            customisation_id: CustomisationId::try_new("cust-1".to_string()).unwrap(),
            skill_id: SkillId::try_new("BLOC".to_string()).unwrap(),
            skill_name: SkillName::try_new("Bloc".to_string()).unwrap(),
            author: "Commissaire".into(),
        });
        let p = Player::from_events(&tous).unwrap();
        assert_eq!(p.acquired_skills.len(), 2);

        tous.push(
            p.revert_match_impact(&MatchReportId("mr-1".into()))
                .unwrap(),
        );
        let apres = Player::from_events(&tous).unwrap();
        let restants: Vec<&str> = apres
            .acquired_skills
            .iter()
            .map(|s| s.skill_id.as_ref())
            .collect();
        assert_eq!(
            restants,
            vec!["BLOC"],
            "la compétence du commissaire devait survivre, la Haine partir"
        );
    }

    /// Un match complet : essai, passe, sortie, faute, MVP, puis conclusion.
    fn match_events(mr: &str) -> Vec<PlayerDomainEvent> {
        let p = || PlayerId("p1".into());
        let t = || TeamId("t1".into());
        vec![
            PlayerDomainEvent::TouchdownScored {
                player_id: p(),
                team_id: t(),
                context: context(mr),
                spp_earned: spp(3),
            },
            PlayerDomainEvent::PassCompleted {
                player_id: p(),
                team_id: t(),
                context: context(mr),
                spp_earned: spp(1),
            },
            PlayerDomainEvent::CasualtyInflicted {
                player_id: p(),
                team_id: t(),
                context: context(mr),
                spp_earned: spp(2),
            },
            PlayerDomainEvent::FoulCommitted {
                player_id: p(),
                team_id: t(),
                context: context(mr),
            },
            PlayerDomainEvent::MatchMvpNamed {
                player_id: p(),
                team_id: t(),
                context: context(mr),
                spp_earned: spp(4),
            },
            PlayerDomainEvent::MatchConcluded {
                player_id: p(),
                team_id: t(),
                context: context(mr),
                team_score: 2,
                opponent_score: 1,
            },
        ]
    }

    fn revert(mr: &str) -> PlayerDomainEvent {
        PlayerDomainEvent::MatchImpactReverted {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            match_report_id: MatchReportId(mr.into()),
        }
    }

    fn hydrate(events: Vec<PlayerDomainEvent>) -> Player {
        Player::from_events(&events).unwrap()
    }

    #[test]
    fn revert_retire_les_spp_et_les_compteurs_du_match() {
        let mut events = vec![created()];
        events.extend(match_events("mr1"));
        let avant = hydrate(events.clone());
        assert_eq!(avant.spp.0, 10);

        events.push(revert("mr1"));
        let apres = hydrate(events);

        assert_eq!(apres.spp.0, 0);
        assert_eq!(apres.career_touchdowns.0, 0);
        assert_eq!(apres.career_passes.0, 0);
        assert_eq!(apres.career_casualties.0, 0);
        assert_eq!(apres.career_fouls.0, 0);
        assert_eq!(apres.career_mvps.0, 0);
        assert_eq!(apres.matches_played.0, 0);
    }

    #[test]
    fn revert_ne_touche_pas_les_matchs_anterieurs() {
        let mut events = vec![created()];
        events.extend(match_events("mr1"));
        events.extend(match_events("mr2"));
        events.push(revert("mr2"));

        let apres = hydrate(events);

        // il reste exactement la contribution de mr1
        assert_eq!(apres.spp.0, 10);
        assert_eq!(apres.career_touchdowns.0, 1);
        assert_eq!(apres.matches_played.0, 1);
    }

    #[test]
    fn revert_retire_la_blessure_et_le_malus_de_sequelle() {
        let mut events = vec![created()];
        events.push(PlayerDomainEvent::InjurySustained {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            context: context("mr1"),
            injury_type: InjuryType::Sequel { stat: StatKind::Ag },
        });
        let avant = hydrate(events.clone());
        assert_eq!(avant.injuries.len(), 1);
        assert_eq!(avant.stat_adjustments.len(), 1);

        events.push(revert("mr1"));
        let apres = hydrate(events);

        assert!(apres.injuries.is_empty());
        assert!(
            apres.stat_adjustments.is_empty(),
            "le malus de séquelle doit disparaître"
        );
    }

    #[test]
    fn revert_retire_le_compteur_de_blessures_persistantes() {
        let mut events = vec![created()];
        events.push(PlayerDomainEvent::InjurySustained {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            context: context("mr1"),
            injury_type: InjuryType::BlessureSerieuse,
        });
        events.push(revert("mr1"));

        assert_eq!(hydrate(events).career_persistent_injuries.0, 0);
    }

    /// Règle 15 : le statut retrouve sa valeur d'avant le match, pas `Available`
    /// par défaut.
    #[test]
    fn revert_restaure_le_statut_de_participation() {
        let mut events = vec![created()];
        events.push(PlayerDomainEvent::InjurySustained {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            context: context("mr1"),
            injury_type: InjuryType::Amoche,
        });
        assert_eq!(
            hydrate(events.clone()).participation_status,
            PlayerParticipationStatus::MissingNextGame
        );

        events.push(revert("mr1"));

        assert_eq!(
            hydrate(events).participation_status,
            PlayerParticipationStatus::Available
        );
    }

    /// Un joueur déjà absent avant le match doit le redevenir : la conclusion du
    /// match l'avait rendu disponible, la compensation défait aussi cela.
    #[test]
    fn revert_remet_un_joueur_deja_absent_dans_son_absence() {
        let mut events = vec![created()];
        // absence héritée du match mr0
        events.push(PlayerDomainEvent::InjurySustained {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            context: context("mr0"),
            injury_type: InjuryType::Amoche,
        });
        // mr1 : le joueur ne se blesse pas, la conclusion le restaure
        events.push(PlayerDomainEvent::MatchConcluded {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            context: context("mr1"),
            team_score: 1,
            opponent_score: 0,
        });
        events.push(PlayerDomainEvent::PlayerAvailabilityRestored {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            match_report_id: MatchReportId("mr1".into()),
        });
        assert_eq!(
            hydrate(events.clone()).participation_status,
            PlayerParticipationStatus::Available
        );

        events.push(revert("mr1"));

        assert_eq!(
            hydrate(events).participation_status,
            PlayerParticipationStatus::MissingNextGame,
            "la restauration de disponibilité doit être défaite elle aussi"
        );
    }

    #[test]
    fn revert_d_un_autre_match_ne_produit_rien() {
        let mut events = vec![created()];
        events.extend(match_events("mr1"));
        let player = hydrate(events);

        assert!(player
            .revert_match_impact(&MatchReportId("mr-autre".into()))
            .is_none());
    }

    /// Règle 11 : rejouer la compensation ne retranche rien de plus.
    #[test]
    fn un_second_revert_ne_produit_rien() {
        let mut events = vec![created()];
        events.extend(match_events("mr1"));
        events.push(revert("mr1"));
        let player = hydrate(events);

        assert!(player
            .revert_match_impact(&MatchReportId("mr1".into()))
            .is_none());
        assert_eq!(player.spp.0, 0);
    }

    /// Règle 8 : le cycle doit converger, sans quoi corriger deux fois dériverait.
    #[test]
    fn publier_depublier_republier_converge_vers_le_meme_etat() {
        let mut nominal = vec![created()];
        nominal.extend(match_events("mr1"));
        let attendu = hydrate(nominal.clone());

        let mut corrige = vec![created()];
        corrige.extend(match_events("mr1"));
        corrige.push(revert("mr1"));
        corrige.extend(match_events("mr1"));
        let obtenu = hydrate(corrige);

        assert_eq!(obtenu.spp.0, attendu.spp.0);
        assert_eq!(obtenu.career_touchdowns.0, attendu.career_touchdowns.0);
        assert_eq!(obtenu.matches_played.0, attendu.matches_played.0);
        assert_eq!(obtenu.participation_status, attendu.participation_status);
    }
}

#[cfg(test)]
mod roster_edition_tests {
    use super::*;

    fn active_player() -> Player {
        let created = PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
            jersey: Some(JerseyVo::try_new(7).unwrap()),
            base_skills: vec![],
            starting_spp: Spp(0),
            starting_value: ValueKpo(100),
        };
        Player::from_events(&[created]).unwrap()
    }

    fn dismissed_player() -> Player {
        let player = active_player();
        let dismissed = PlayerDomainEvent::PlayerDismissed {
            player_id: player.id.clone(),
            team_id: player.team_id.clone(),
        };
        Player::apply(Some(player), &dismissed).unwrap()
    }

    fn name(value: &str) -> PersonalName {
        PersonalName::try_new(value.to_string()).unwrap()
    }

    #[test]
    fn rename_produces_player_renamed_event_with_new_name() {
        let player = active_player();
        let event = player.rename(Some(name("Grok Fracasse"))).unwrap();

        match &event {
            PlayerDomainEvent::PlayerRenamed { personal_name, .. } => {
                assert_eq!(personal_name.as_ref().unwrap().as_ref(), "Grok Fracasse");
            }
            autre => panic!("événement inattendu : {autre:?}"),
        }

        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.personal_name.unwrap().as_ref(), "Grok Fracasse");
    }

    #[test]
    fn rename_allows_clearing_to_none() {
        let player = active_player();
        let player = Player::apply(
            Some(player.clone()),
            &player.rename(Some(name("Grok"))).unwrap(),
        )
        .unwrap();
        assert!(player.personal_name.is_some());

        // `None` efface : ce n'est pas un « champ non fourni », c'est un retrait
        // demandé. La lecture retombe alors sur le nom de poste.
        let player = Player::apply(Some(player.clone()), &player.rename(None).unwrap()).unwrap();
        assert!(player.personal_name.is_none());
    }

    #[test]
    fn change_jersey_produces_player_jersey_changed_event() {
        let player = active_player();
        let event = player
            .change_jersey(Some(JerseyVo::try_new(12).unwrap()))
            .unwrap();
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.jersey.unwrap().into_inner(), 12);
    }

    #[test]
    fn change_jersey_allows_clearing_to_none() {
        let player = active_player();
        assert!(player.jersey.is_some());

        let event = player.change_jersey(None).unwrap();
        let player = Player::apply(Some(player), &event).unwrap();
        assert!(player.jersey.is_none());
    }

    #[test]
    fn reorder_produces_player_reordered_event() {
        let player = active_player();
        assert!(player.display_order.is_none());

        let event = player.reorder(DisplayOrder::new(3)).unwrap();
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.display_order.unwrap().into_inner(), 3);
    }

    #[test]
    fn rename_change_jersey_reorder_reject_dismissed_player() {
        let player = dismissed_player();

        assert_eq!(
            player.rename(Some(name("Grok"))).unwrap_err(),
            DomainError::PlayerNotActive
        );
        assert_eq!(
            player
                .change_jersey(Some(JerseyVo::try_new(12).unwrap()))
                .unwrap_err(),
            DomainError::PlayerNotActive
        );
        assert_eq!(
            player.reorder(DisplayOrder::new(1)).unwrap_err(),
            DomainError::PlayerNotActive
        );
    }

    #[test]
    fn personal_name_rejects_over_50_chars() {
        assert!(PersonalName::try_new("a".repeat(50)).is_ok());
        assert!(PersonalName::try_new("a".repeat(51)).is_err());
    }

    #[test]
    fn personal_name_allows_apostrophe() {
        // Beaucoup de patronymes en portent une — c'est la seule différence
        // admise avec `PositionNameVo`, dont les valeurs viennent du corpus.
        assert!(PersonalName::try_new("Sean O'Malley".to_string()).is_ok());
    }

    #[test]
    fn personal_name_rejects_empty_string() {
        assert!(PersonalName::try_new(String::new()).is_err());
        // `trim` s'applique avant la validation : des espaces seuls sont vides.
        assert!(PersonalName::try_new("   ".to_string()).is_err());
    }

    #[test]
    fn jersey_vo_rejects_zero_and_above_99() {
        assert!(JerseyVo::try_new(0).is_err());
        assert!(JerseyVo::try_new(1).is_ok());
        assert!(JerseyVo::try_new(99).is_ok());
        assert!(JerseyVo::try_new(100).is_err());
    }
}

#[cfg(test)]
mod customisation_tests {
    use super::*;
    use crate::app::players::domain::value_objects::BasketLineId;

    fn joueur() -> Player {
        let created = PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
            jersey: Some(JerseyVo::try_new(7).unwrap()),
            base_skills: vec![],
            starting_spp: Spp(4),
            starting_value: ValueKpo(100),
        };
        Player::from_events(&[created]).unwrap()
    }

    fn renvoye() -> Player {
        let j = joueur();
        let event = PlayerDomainEvent::PlayerDismissed {
            player_id: j.id.clone(),
            team_id: j.team_id.clone(),
        };
        Player::apply(Some(j), &event).unwrap()
    }

    fn id() -> CustomisationId {
        CustomisationId::try_new("c1".to_string()).unwrap()
    }

    fn nom(v: &str) -> SkillName {
        SkillName::try_new(v.to_string()).unwrap()
    }

    fn competence(v: &str) -> SkillId {
        SkillId::try_new(v.to_string()).unwrap()
    }

    /// Le cœur : la commande reçoit des **crans** en qualité du joueur,
    /// l'événement porte l'**offset brut**. Améliorer l'agilité donne -1.
    #[test]
    fn customise_stat_traduit_les_crans_en_offset_brut() {
        let j = joueur();
        let crans = StatCrans::try_new(1).unwrap();

        let event = j
            .customise_stat(id(), StatKind::Ag, crans, "Bagouze".into())
            .unwrap();
        match &event {
            PlayerDomainEvent::PlayerStatCustomised { offset, .. } => assert_eq!(*offset, -1),
            autre => panic!("événement inattendu : {autre:?}"),
        }

        // Et l'armure, qui partage le suffixe « + » mais pas la direction.
        let event = j
            .customise_stat(id(), StatKind::Av, crans, "Bagouze".into())
            .unwrap();
        match &event {
            PlayerDomainEvent::PlayerStatCustomised { offset, .. } => assert_eq!(*offset, 1),
            autre => panic!("événement inattendu : {autre:?}"),
        }
    }

    #[test]
    fn customise_stat_accumule_dans_stat_customisations() {
        let j = joueur();
        let event = j
            .customise_stat(
                id(),
                StatKind::Ma,
                StatCrans::try_new(2).unwrap(),
                "Bagouze".into(),
            )
            .unwrap();
        let j = Player::apply(Some(j), &event).unwrap();

        assert_eq!(j.stat_customisations.len(), 1);
        assert_eq!(j.stat_customisations[0].offset, 2);
    }

    /// La règle la plus contre-intuitive de la fonctionnalité : une compétence
    /// donnée par un commissaire ne renchérit pas le joueur, contrairement à la
    /// même achetée en SPP.
    #[test]
    fn une_competence_customisee_n_ajoute_aucune_valeur() {
        let j = joueur();
        let avant = j.value.0;

        let event = j
            .customise_skill(id(), competence("BLOCK"), nom("Bloc"), "Bagouze".into())
            .unwrap();
        let j = Player::apply(Some(j), &event).unwrap();

        assert_eq!(j.value.0, avant, "la valeur ne doit pas bouger");
        assert_eq!(j.acquired_skills.len(), 1);
        assert_eq!(j.acquired_skills[0].mode, AcquisitionMode::Customised);
        assert_eq!(j.acquired_skills[0].value_delta.0, 0);
    }

    #[test]
    fn une_caracteristique_customisee_n_ajoute_aucune_valeur() {
        let j = joueur();
        let avant = j.value.0;
        let event = j
            .customise_stat(
                id(),
                StatKind::St,
                StatCrans::try_new(1).unwrap(),
                "Bagouze".into(),
            )
            .unwrap();
        let j = Player::apply(Some(j), &event).unwrap();
        assert_eq!(j.value.0, avant);
    }

    /// Le prix, lui, déplace bien la valeur — c'est le seul levier du mode.
    #[test]
    fn le_prix_customise_deplace_la_valeur() {
        let j = joueur();
        let event = j
            .customise_value(id(), KpoDelta::try_new(-30).unwrap(), "Bagouze".into())
            .unwrap();
        let j = Player::apply(Some(j), &event).unwrap();
        assert_eq!(j.value.0, 70);
    }

    /// Le plancher est une règle du panier, mais `apply` ne doit pas produire
    /// d'underflow si un événement historique le franchissait.
    #[test]
    fn le_prix_ne_passe_pas_sous_zero_a_l_application() {
        let j = joueur();
        let event = j
            .customise_value(id(), KpoDelta::try_new(-500).unwrap(), "Bagouze".into())
            .unwrap();
        let j = Player::apply(Some(j), &event).unwrap();
        assert_eq!(j.value.0, 0);
    }

    #[test]
    fn les_spp_customises_s_ajoutent_au_total() {
        let j = joueur();
        let event = j
            .customise_spp(id(), SppAmount::try_new(10).unwrap(), "Bagouze".into())
            .unwrap();
        let j = Player::apply(Some(j), &event).unwrap();
        assert_eq!(j.spp.0, 14);
    }

    /// Contrairement à `rename`, aucune garde d'appartenance : la phase 1 pose
    /// que les customisations s'appliquent toujours.
    #[test]
    fn un_joueur_renvoye_reste_customisable() {
        let j = renvoye();

        assert!(j
            .customise_skill(id(), competence("BLOCK"), nom("Bloc"), "B".into())
            .is_ok());
        assert!(j
            .customise_stat(
                id(),
                StatKind::Ma,
                StatCrans::try_new(1).unwrap(),
                "B".into()
            )
            .is_ok());
        assert!(j
            .customise_value(id(), KpoDelta::try_new(10).unwrap(), "B".into())
            .is_ok());
        assert!(j
            .customise_spp(id(), SppAmount::try_new(5).unwrap(), "B".into())
            .is_ok());

        // Le contraste : renommer reste interdit.
        assert_eq!(j.rename(None).unwrap_err(), DomainError::PlayerNotActive);
    }

    #[test]
    fn les_value_objects_bornent_ce_qu_ils_doivent() {
        assert!(StatCrans::try_new(0).is_err());
        assert!(KpoDelta::try_new(0).is_err());
        assert!(SppAmount::try_new(0).is_err());
        assert!(SppAmount::try_new(100).is_ok());
        assert!(SppAmount::try_new(101).is_err());
        assert!(CustomisationId::try_new(String::new()).is_err());
        assert!(BasketLineId::try_new(String::new()).is_err());
    }
}
