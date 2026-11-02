use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_draft::MatchReportDraft;
use crate::app::match_report::domain::match_report_ready_to_publish::MatchReportReadyToPublish;
use crate::app::match_report::domain::mvp_draw::{self, DesignatedPlayer, DrawnMvp};
use crate::app::match_report::domain::underdog;
use crate::app::match_report::domain::value_objects::NoStallingBonuses;
use crate::app::match_report::domain::value_objects::TurnNumber;
use crate::app::match_report::domain::value_objects::{
    ActionId, ActionPlayer, AllowedInducementSpec, D3Roll, DedicatedFans, EntryMode, FanFactorMod,
    InducementMaxQty, InducementPurchase, InducementQty, MatchAction, MatchActionType, MatchGain,
    MatchReportOrigin, PurchaseKind, TeamSide, TeamValue, TempPlayer, TempPlayerKind,
};
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::bloodbowl::inducement_definition::InducementId;
use rand::Rng;

/// Ce que l'underdog peut ajouter à sa petite monnaie, pris à sa trésorerie.
/// Plafond de règlement, pas de configuration.
pub const MAX_TREASURY_TOP_UP: u32 = 50;
/// Star players qu'une équipe peut engager pour un match. Publique, comme les
/// deux suivantes, pour que la feuille imprimée rappelle la règle sans en
/// recopier le chiffre (carte 607).
pub const MAX_STAR_PLAYERS: usize = 2;
/// Mercenaires par match, toutes positions confondues.
pub const MAX_MERCENARIES: u8 = 3;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};

#[derive(Debug, Clone)]
pub struct MatchReportPreMatch {
    pub id: MatchReportId,
    pub space_id: SpaceId,
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
    pub created_by: CoachId,
    /// Le mode de saisie, choisi au démarrage (carte 631).
    pub entry_mode: EntryMode,
    /// Celui qui a démarré le compte rendu. À ne pas confondre avec
    /// `created_by`, qui vaut l'identifiant de compétition pour un brouillon
    /// né du calendrier.
    pub started_by: CoachId,
    pub origin: MatchReportOrigin,
    pub pairing_id: Option<String>,
    pub home_fan_roll: Option<D3Roll>,
    pub away_fan_roll: Option<D3Roll>,
    pub home_dedicated_fans: DedicatedFans,
    pub away_dedicated_fans: DedicatedFans,
    pub home_team_value: Option<TeamValue>,
    pub away_team_value: Option<TeamValue>,
    pub home_inducements: Option<Vec<InducementPurchase>>,
    pub away_inducements: Option<Vec<InducementPurchase>>,
    /// Qui a validé les coups de pouce de chaque camp (carte 628, décision 5) :
    /// posé au rejeu de `InducementsRecorded`, effacé par
    /// `InducementsCancelled`. Le récapitulatif en tire son « par … ».
    pub home_inducements_recorded_by: Option<CoachId>,
    pub away_inducements_recorded_by: Option<CoachId>,
    pub star_engagements: Vec<(TeamId, InducementId)>,
    pub home_temp_players: Vec<TempPlayer>,
    pub away_temp_players: Vec<TempPlayer>,
    pub home_actions: Vec<MatchAction>,
    pub away_actions: Vec<MatchAction>,
    /// Les MVP tirés au sort et leurs désignés (carte 638) : leur action ne
    /// se supprime pas. Vide pour un rapport sans tirage.
    pub drawn_mvps: Vec<DrawnMvp>,
    pub version: u64,
}

/// Où en sont les coups de pouce (cartes 627 et 628) : une équipe a la main,
/// ou les deux ont validé et la page devient le récapitulatif (R6).
#[derive(Debug, PartialEq, Eq)]
pub enum InducementsTurn<'a> {
    Team(&'a TeamId),
    BothRecorded,
}

impl MatchReportPreMatch {
    /// Annule un rapport dont la sélection est déjà confirmée — les deux
    /// équipes sont verrouillées en saisie, le listener `teams` les libère à
    /// partir des ids portés par l'événement.
    pub fn cancel(self, reason: String) -> MatchReportDomainEvent {
        // Les journaliers des **deux** équipes : le rapport n'aura pas lieu,
        // aucun d'eux n'a de raison de rester dans un effectif.
        let journeymen = self
            .home_temp_players
            .iter()
            .chain(self.away_temp_players.iter())
            .filter(|p| matches!(p.kind, TempPlayerKind::Journeyman { .. }))
            .map(|p| p.id.clone())
            .collect();
        MatchReportDomainEvent::MatchReportCancelled {
            reason,
            journeymen,
            home_team_id: Some(self.home_team_id),
            away_team_id: Some(self.away_team_id),
            pairing_id: self.pairing_id,
        }
    }

    pub fn record_fan_factor(
        &self,
        home_fan_roll: D3Roll,
        away_fan_roll: D3Roll,
        home_dedicated_fans: DedicatedFans,
        away_dedicated_fans: DedicatedFans,
        recorded_by: CoachId,
    ) -> (Self, MatchReportDomainEvent) {
        let event = MatchReportDomainEvent::FanFactorRecorded {
            home_fan_roll,
            away_fan_roll,
            home_dedicated_fans,
            away_dedicated_fans,
            recorded_by,
        };
        let mut updated = self.clone();
        updated.home_fan_roll = Some(home_fan_roll);
        updated.away_fan_roll = Some(away_fan_roll);
        updated.home_dedicated_fans = home_dedicated_fans;
        updated.away_dedicated_fans = away_dedicated_fans;
        updated.version += 1;
        (updated, event)
    }

    pub fn record_team_values(
        &self,
        home_tv: TeamValue,
        away_tv: TeamValue,
        recorded_by: CoachId,
    ) -> (Self, MatchReportDomainEvent) {
        let event = MatchReportDomainEvent::TeamValuesRecorded {
            home_team_value: home_tv,
            away_team_value: away_tv,
            recorded_by,
        };
        let mut updated = self.clone();
        updated.home_team_value = Some(home_tv);
        updated.away_team_value = Some(away_tv);
        updated.version += 1;
        (updated, event)
    }

    pub fn topdog_team_id(&self) -> &TeamId {
        let home_tv = self.home_team_value.expect("TV not recorded");
        let away_tv = self.away_team_value.expect("TV not recorded");
        // La règle vit dans `underdog` (carte 602) : la feuille imprimée
        // l'applique avant qu'aucun rapport n'existe.
        match underdog::top_dog(home_tv, away_tv) {
            underdog::Side::Away => &self.away_team_id,
            underdog::Side::Home => &self.home_team_id,
        }
    }

    pub fn underdog_team_id(&self) -> &TeamId {
        if self.topdog_team_id() == &self.home_team_id {
            &self.away_team_id
        } else {
            &self.home_team_id
        }
    }

    pub fn topdog_spending(&self) -> u32 {
        let topdog = self.topdog_team_id().clone();
        let purchases = if topdog == self.home_team_id {
            self.home_inducements.as_deref()
        } else {
            self.away_inducements.as_deref()
        };
        purchases
            .unwrap_or(&[])
            .iter()
            .map(|p| p.total_cost())
            .sum()
    }

    /// La petite monnaie de l'underdog : l'écart de valeur d'équipe, plus ce que
    /// le top dog a lui-même dépensé.
    ///
    /// Elle est **dynamique** — un achat de plus côté top dog l'augmente, et
    /// c'est voulu : elle compense l'écart au moment où il se creuse.
    pub fn petty_cash(&self) -> u32 {
        let tv_diff =
            underdog::tv_gap(self.home_team_value.unwrap(), self.away_team_value.unwrap());
        tv_diff + self.topdog_spending()
    }

    /// Ce qu'une équipe a le droit de dépenser en coups de pouce.
    ///
    /// Le top dog n'achète qu'avec sa trésorerie. L'underdog dépense d'abord sa
    /// petite monnaie, puis complète avec **50 kPo au plus** pris à sa caisse —
    /// et seulement s'il les a.
    pub fn inducement_budget_for(&self, team_id: &TeamId, treasury: u32) -> u32 {
        if team_id == self.topdog_team_id() {
            treasury
        } else {
            self.petty_cash() + treasury.min(MAX_TREASURY_TOP_UP)
        }
    }

    /// Ce que ces achats retirent réellement à la trésorerie — qui n'est pas le
    /// montant acheté.
    ///
    /// Le top dog paie tout. L'underdog puise d'abord dans sa petite monnaie,
    /// qui ne vient de nulle part et ne coûte rien : seul le dépassement sort de
    /// sa caisse.
    ///
    /// Le plafond de 50 kPo n'est pas réappliqué ici : `inducement_budget_for`
    /// l'a déjà imposé en amont, donc la différence ne peut pas le dépasser.
    /// L'écrire une seconde fois donnerait deux endroits à corriger le jour où
    /// il change.
    pub fn treasury_spending_for(&self, team_id: &TeamId) -> u32 {
        // Sans valeurs d'équipe, il n'y a ni top dog ni petite monnaie — et il
        // ne peut pas y avoir d'achats non plus : `record_inducements_use_case`
        // les exige et refuse par `TeamValuesNotRecorded` sans elles. Rendre
        // zéro garde la méthode **totale**, y compris pour un rapport publié
        // sans jamais passer par l'étape des coups de pouce, qui est le cas le
        // plus courant.
        if self.home_team_value.is_none() || self.away_team_value.is_none() {
            return 0;
        }
        let depense: u32 = self
            .purchases_for(team_id)
            .iter()
            .map(|p| p.total_cost())
            .sum();
        if team_id == self.topdog_team_id() {
            depense
        } else {
            depense.saturating_sub(self.petty_cash())
        }
    }

    pub fn record_inducements(
        &self,
        team_id: &TeamId,
        purchases: &[(InducementId, u8)],
        budget: u32,
        allowed_specs: &[AllowedInducementSpec],
        opponent_star_uids: &[InducementId],
        recorded_by: CoachId,
    ) -> Result<(Self, Vec<MatchReportDomainEvent>), DomainError> {
        self.check_inducements_open_for(team_id)?;
        validate_purchases(purchases, allowed_specs, opponent_star_uids, budget)?;
        let purchase_list = build_purchase_list(purchases, allowed_specs)?;
        let events = build_inducement_events(team_id, &purchase_list, allowed_specs, recorded_by);
        let mut updated = self.clone();
        set_inducements_for(&mut updated, team_id, purchase_list, recorded_by);
        updated.version += events.len() as u64;
        Ok((updated, events))
    }

    /// R1 : l'équipe a validé ses coups de pouce depuis la dernière
    /// annulation — achats ou « Passer », qui enregistre une liste vide.
    pub fn has_recorded_inducements(&self, team_id: &TeamId) -> bool {
        self.inducements_for(team_id).is_some()
    }

    /// R2 puis R9 : l'équipe peut-elle encore enregistrer ses coups de pouce ?
    ///
    /// Une équipe qui a validé ne rachète plus ; l'outsider attend que le favori
    /// ait validé. R9 ne regarde que les nouveaux envois : un outsider qui avait
    /// acheté avant le favori garde ses achats.
    ///
    /// R2 passe avant la question des valeurs d'équipe : un `ReadyToPublish`
    /// converti par `into_pre_match()` n'en porte pas, et doit être refusé
    /// pour ce qu'il est.
    pub fn check_inducements_open_for(&self, team_id: &TeamId) -> Result<(), DomainError> {
        if self.has_recorded_inducements(team_id) {
            return Err(DomainError::InducementsAlreadyRecorded);
        }
        if self.home_team_value.is_none() || self.away_team_value.is_none() {
            return Err(DomainError::TeamValuesNotRecorded);
        }
        let topdog = self.topdog_team_id();
        if team_id != topdog && !self.has_recorded_inducements(topdog) {
            return Err(DomainError::TopdogHasNotChosen);
        }
        Ok(())
    }

    /// L'équipe dont c'est le tour de choisir (R7, R9) : le favori tant qu'il
    /// n'a pas validé, puis l'outsider. Quand les deux ont validé, plus
    /// personne : c'est le récapitulatif (R6, carte 628).
    pub fn inducements_turn(&self) -> InducementsTurn<'_> {
        let topdog = self.topdog_team_id();
        let underdog = self.underdog_team_id();
        match (
            self.has_recorded_inducements(topdog),
            self.has_recorded_inducements(underdog),
        ) {
            (true, true) => InducementsTurn::BothRecorded,
            (true, false) => InducementsTurn::Team(underdog),
            (false, _) => InducementsTurn::Team(topdog),
        }
    }

    /// R3 et R4, en question : le récapitulatif ne propose l'annulation que si
    /// le domaine l'accepterait (carte 628). L'état `PreMatch` est garanti par
    /// le type ; un `ReadyToPublish` ne la propose jamais.
    pub fn can_cancel_inducements(&self) -> bool {
        self.check_inducements_cancellable().is_ok()
    }

    /// Ce que les achats de cette équipe coûtent, au prix du catalogue — le
    /// total que le récapitulatif rapporte au budget (carte 628).
    pub fn inducements_total_for(&self, team_id: &TeamId) -> u32 {
        self.purchases_for(team_id)
            .iter()
            .map(|p| p.total_cost())
            .sum()
    }

    /// Qui a validé les coups de pouce de cette équipe (décision 5).
    pub fn inducements_recorded_by(&self, team_id: &TeamId) -> Option<CoachId> {
        if team_id == &self.home_team_id {
            self.home_inducements_recorded_by
        } else {
            self.away_inducements_recorded_by
        }
    }

    /// La nature d'un achat de cette équipe (décision 6) : un mercenaire se
    /// lit à son identifiant, une vedette à son engagement.
    pub fn purchase_kind(&self, team_id: &TeamId, purchase: &InducementPurchase) -> PurchaseKind {
        if let Some(kind) = purchase.mercenary_kind() {
            return kind;
        }
        let engaged = self
            .star_engagements
            .iter()
            .any(|(tid, uid)| tid == team_id && uid == &purchase.uid);
        match engaged {
            true => PurchaseKind::StarPlayer,
            false => PurchaseKind::Inducement,
        }
    }

    fn inducements_for(&self, team_id: &TeamId) -> Option<&Vec<InducementPurchase>> {
        if team_id == &self.home_team_id {
            self.home_inducements.as_ref()
        } else {
            self.away_inducements.as_ref()
        }
    }

    /// Efface les coups de pouce des **deux** équipes : on recommence par le
    /// favori (carte 626, R3 à R5).
    ///
    /// Rend `InducementsCancelled`, puis un `TempPlayersReset` par camp qui a
    /// des joueurs temporaires — c'est lui qui nomme les journaliers retirés,
    /// et le chemin qui les sort de `players` est celui de la carte 455.
    ///
    /// La petite monnaie ne se recalcule nulle part : sans achats du favori,
    /// `petty_cash` retombe d'elle-même sur l'écart de valeur.
    pub fn cancel_inducements(
        &self,
        cancelled_by: CoachId,
    ) -> Result<(Self, Vec<MatchReportDomainEvent>), DomainError> {
        self.check_inducements_cancellable()?;
        let mut updated = self.clone().apply_inducements_cancelled();
        let mut events = vec![MatchReportDomainEvent::InducementsCancelled { cancelled_by }];
        for team_id in [self.home_team_id.clone(), self.away_team_id.clone()] {
            if updated.temp_players_of(&team_id).is_empty() {
                continue;
            }
            let (next, reset) = updated.reset_temp_players(&team_id);
            updated = next;
            events.push(reset);
        }
        Ok((updated, events))
    }

    /// R3 et R4 : aucune action **vivante** — une action supprimée ne désigne
    /// plus personne (décision du 2026-11-01) — et au moins une équipe qui a
    /// validé. L'état `PreMatch` est garanti par le type.
    fn check_inducements_cancellable(&self) -> Result<(), DomainError> {
        if !self.home_actions.is_empty() || !self.away_actions.is_empty() {
            return Err(DomainError::MatchAlreadyStarted);
        }
        let nobody_validated = self.inducements_for(&self.home_team_id).is_none()
            && self.inducements_for(&self.away_team_id).is_none();
        if nobody_validated {
            return Err(DomainError::NoInducementsToCancel);
        }
        Ok(())
    }

    /// L'effet d'un `InducementsCancelled`, partagé par la commande et le
    /// rejeu : achats et vedettes des deux équipes effacés.
    pub(crate) fn apply_inducements_cancelled(mut self) -> Self {
        self.home_inducements = None;
        self.away_inducements = None;
        self.home_inducements_recorded_by = None;
        self.away_inducements_recorded_by = None;
        self.star_engagements.clear();
        self.version += 1;
        self
    }

    fn temp_players_of(&self, team_id: &TeamId) -> &[TempPlayer] {
        match team_id == &self.home_team_id {
            true => &self.home_temp_players,
            false => &self.away_temp_players,
        }
    }

    pub fn init_temp_players(
        &self,
        team_id: &TeamId,
        players: Vec<TempPlayer>,
    ) -> (Self, MatchReportDomainEvent) {
        let event = MatchReportDomainEvent::TempPlayersInitialized {
            team_id: team_id.clone(),
            players: players.clone(),
        };
        let mut updated = self.clone();
        if team_id == &updated.home_team_id {
            updated.home_temp_players = players;
        } else {
            updated.away_temp_players = players;
        }
        updated.version += 1;
        (updated, event)
    }

    /// **L'événement nomme les journaliers retirés.** Ce sont les seuls
    /// remplaçants à exister dans `players` : une vedette et un mercenaire
    /// vivent le temps du rapport et n'y ont pas de joueur à effacer.
    pub fn reset_temp_players(&self, team_id: &TeamId) -> (Self, MatchReportDomainEvent) {
        let side = match team_id == &self.home_team_id {
            true => &self.home_temp_players,
            false => &self.away_temp_players,
        };
        let event = MatchReportDomainEvent::TempPlayersReset {
            team_id: team_id.clone(),
            withdrawn: side
                .iter()
                .filter(|p| matches!(p.kind, TempPlayerKind::Journeyman { .. }))
                .map(|p| p.id.clone())
                .collect(),
        };
        let mut updated = self.clone();
        if team_id == &updated.home_team_id {
            updated.home_temp_players = vec![];
        } else {
            updated.away_temp_players = vec![];
        }
        updated.version += 1;
        (updated, event)
    }

    pub fn record_action(
        &self,
        team_side: TeamSide,
        turn: TurnNumber,
        player: ActionPlayer,
        action: MatchActionType,
        player_display_name: String,
        player_position: String,
        action_id: ActionId,
        recorded_by: CoachId,
    ) -> Result<(Self, MatchReportDomainEvent), DomainError> {
        check_match_action(&action)?;
        let entry = MatchAction {
            id: action_id,
            turn,
            player,
            action,
            player_display_name,
            player_position,
        };
        Ok(self.push_action(team_side, entry, recorded_by))
    }

    /// Le MVP d'un camp (carte 637, R7) : une action `Mvp` au **tour 16**, et
    /// **une seule par camp** — un MVP déjà saisi, à quelque tour que ce soit,
    /// se supprime avant d'en désigner un autre.
    ///
    /// La règle vaut à la saisie, jamais au rejeu : `rehydrate` construit les
    /// actions sans passer par ici, et un ancien rapport à deux MVP, ou à un MVP
    /// au tour 7, se relit à l'identique.
    pub fn record_mvp(
        &self,
        side: TeamSide,
        player: ActionPlayer,
        player_display_name: String,
        player_position: String,
        action_id: ActionId,
        recorded_by: CoachId,
    ) -> Result<(Self, MatchReportDomainEvent), DomainError> {
        if self.has_mvp(side) {
            return Err(DomainError::MvpAlreadyRecorded);
        }
        let entry = MatchAction {
            id: action_id,
            turn: TurnNumber::LAST,
            player,
            action: MatchActionType::Mvp,
            player_display_name,
            player_position,
        };
        Ok(self.push_action(side, entry, recorded_by))
    }

    /// Les MVP d'un camp. Plusieurs pour un rapport ancien, saisi avant que
    /// la règle n'existe — ils restent valides, et l'étape les montre.
    pub fn mvps_of(&self, side: TeamSide) -> Vec<&MatchAction> {
        self.actions_for(side)
            .iter()
            .filter(|a| matches!(a.action, MatchActionType::Mvp))
            .collect()
    }

    pub fn has_mvp(&self, side: TeamSide) -> bool {
        !self.mvps_of(side).is_empty()
    }

    /// L'action ajoutée à son camp, et l'événement qui le dit.
    fn push_action(
        &self,
        side: TeamSide,
        entry: MatchAction,
        recorded_by: CoachId,
    ) -> (Self, MatchReportDomainEvent) {
        let event = MatchReportDomainEvent::ActionRecorded {
            action_id: entry.id.clone(),
            team_side: side,
            turn: entry.turn,
            player: entry.player.clone(),
            action: entry.action.clone(),
            player_display_name: entry.player_display_name.clone(),
            player_position: entry.player_position.clone(),
            recorded_by,
        };
        let mut updated = self.clone();
        match side {
            TeamSide::Home => updated.home_actions.push(entry),
            TeamSide::Away => updated.away_actions.push(entry),
        }
        updated.version += 1;
        (updated, event)
    }

    /// Le MVP tiré au sort parmi six désignés (carte 638, R8).
    ///
    /// Exactement six désignés, distincts ; le tiré passe par `record_mvp`, et
    /// en reçoit les refus — un camp qui a déjà son MVP ne tire pas. Rend
    /// `ActionRecorded` (le `Mvp` au tour 16) puis `MvpDrawn`, à appender
    /// ensemble.
    pub fn draw_mvp(
        &self,
        side: TeamSide,
        designated: Vec<DesignatedPlayer>,
        rng: &mut impl Rng,
        action_id: ActionId,
        recorded_by: CoachId,
    ) -> Result<(Self, Vec<MatchReportDomainEvent>), DomainError> {
        mvp_draw::check_designated(&designated)?;
        let winner = mvp_draw::pick(&designated, rng).clone();
        let (mut updated, recorded) = self.record_mvp(
            side,
            winner.player,
            winner.display_name,
            winner.position,
            action_id.clone(),
            recorded_by,
        )?;
        let drawn = DrawnMvp {
            action_id,
            team_side: side,
            designated,
        };
        let event = drawn.to_event();
        updated.apply_mvp_drawn(drawn);
        Ok((updated, vec![recorded, event]))
    }

    /// Retient un MVP tiré — à la saisie comme au rejeu de `MvpDrawn`.
    pub(crate) fn apply_mvp_drawn(&mut self, drawn: DrawnMvp) {
        self.drawn_mvps.push(drawn);
        self.version += 1;
    }

    /// Le tirage dont cette action est le résultat, s'il y en a un.
    pub fn drawn_mvp(&self, action_id: &ActionId) -> Option<&DrawnMvp> {
        self.drawn_mvps.iter().find(|d| &d.action_id == action_id)
    }

    /// Le premier camp, domicile puis visiteur, qui n'a pas son MVP (carte
    /// 638) : l'après-match ne se clôt pas tant qu'il y en a un.
    pub fn first_side_without_mvp(&self) -> Option<TeamSide> {
        first_side_without_mvp(&self.home_actions, &self.away_actions)
    }

    /// Un MVP tiré au sort ne se supprime pas, même en correction (carte 638).
    pub fn delete_action(
        &self,
        action_id: &ActionId,
        deleted_by: CoachId,
    ) -> Result<(Self, MatchReportDomainEvent), DomainError> {
        if self.drawn_mvp(action_id).is_some() {
            return Err(DomainError::DrawnMvpIsFinal);
        }
        let team_side = self.side_of_action(action_id)?;
        let event = MatchReportDomainEvent::ActionDeleted {
            action_id: action_id.clone(),
            team_side,
            deleted_by,
        };
        let mut updated = self.clone();
        match team_side {
            TeamSide::Home => updated.home_actions.retain(|a| &a.id != action_id),
            TeamSide::Away => updated.away_actions.retain(|a| &a.id != action_id),
        }
        updated.version += 1;
        Ok((updated, event))
    }

    fn side_of_action(&self, action_id: &ActionId) -> Result<TeamSide, DomainError> {
        if self.home_actions.iter().any(|a| &a.id == action_id) {
            Ok(TeamSide::Home)
        } else if self.away_actions.iter().any(|a| &a.id == action_id) {
            Ok(TeamSide::Away)
        } else {
            Err(DomainError::ActionNotFound(action_id.0.clone()))
        }
    }

    pub fn temp_players_for(&self, side: TeamSide) -> &[TempPlayer] {
        match side {
            TeamSide::Home => &self.home_temp_players,
            TeamSide::Away => &self.away_temp_players,
        }
    }

    pub fn star_player_uids_for(&self, team_id: &TeamId) -> Vec<InducementId> {
        self.star_engagements
            .iter()
            .filter(|(tid, _)| tid == team_id)
            .map(|(_, uid)| uid.clone())
            .collect()
    }

    pub fn purchases_for(&self, team_id: &TeamId) -> &[InducementPurchase] {
        self.inducements_for(team_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn actions_for(&self, side: TeamSide) -> &[MatchAction] {
        match side {
            TeamSide::Home => &self.home_actions,
            TeamSide::Away => &self.away_actions,
        }
    }

    pub fn compute_score(&self) -> (u8, u8) {
        let home = self
            .home_actions
            .iter()
            .filter(|a| matches!(a.action, MatchActionType::Touchdown))
            .count() as u8;
        let away = self
            .away_actions
            .iter()
            .filter(|a| matches!(a.action, MatchActionType::Touchdown))
            .count() as u8;
        (home, away)
    }

    pub fn compute_cas(&self) -> (u8, u8) {
        let home = self
            .home_actions
            .iter()
            .filter(|a| matches!(a.action, MatchActionType::Sortie))
            .count() as u8;
        let away = self
            .away_actions
            .iter()
            .filter(|a| matches!(a.action, MatchActionType::Sortie))
            .count() as u8;
        (home, away)
    }

    pub fn suggest_gains(&self) -> (u32, u32) {
        let fans_home = self.home_dedicated_fans.into_inner()
            + self.home_fan_roll.map(|r| r.value() as u32).unwrap_or(0);
        let fans_away = self.away_dedicated_fans.into_inner()
            + self.away_fan_roll.map(|r| r.value() as u32).unwrap_or(0);
        let (tds_home, tds_away) = self.compute_score();
        let base = (fans_home + fans_away) / 2 * 10;
        (base + tds_home as u32 * 10, base + tds_away as u32 * 10)
    }

    pub fn record_post_match(
        &self,
        home_gain: MatchGain,
        away_gain: MatchGain,
        bonuses: NoStallingBonuses,
        home_fan_mod: FanFactorMod,
        away_fan_mod: FanFactorMod,
        summary_title: Option<String>,
        summary_body: Option<String>,
        recorded_by: CoachId,
    ) -> Result<(MatchReportReadyToPublish, MatchReportDomainEvent), DomainError> {
        // Chaque camp a son MVP avant les gains (carte 638) — à la saisie,
        // jamais au rejeu : `rehydrate` ne passe pas par ici.
        if let Some(side) = self.first_side_without_mvp() {
            return Err(DomainError::MvpMissing(side));
        }
        let event = MatchReportDomainEvent::PostMatchRecorded {
            home_gain,
            away_gain,
            home_no_stalling_bonus: bonuses.home,
            away_no_stalling_bonus: bonuses.away,
            home_fan_mod,
            away_fan_mod,
            summary_title: summary_title.clone(),
            summary_body: summary_body.clone(),
            recorded_by,
        };
        let ready = MatchReportReadyToPublish::from_pre_match(
            self,
            home_gain,
            away_gain,
            bonuses,
            home_fan_mod,
            away_fan_mod,
            summary_title,
            summary_body,
        );
        Ok((ready, event))
    }

    /// L'avant-match d'un brouillon confirmé par `confirmed_by`.
    ///
    /// **Premier temps du rejeu** (carte 631) : sans `EntryModeChosen` à la
    /// suite — tout flux écrit avant la carte —, le compte rendu est en « Je
    /// saisis tout », démarré par celui qui a confirmé.
    pub fn from_draft(draft: MatchReportDraft, confirmed_by: CoachId) -> Self {
        Self {
            id: draft.id,
            space_id: draft.space_id,
            competition_id: draft.competition_id,
            season_id: draft.season_id,
            round_id: draft.round_id,
            home_team_id: draft.home_team_id,
            away_team_id: draft.away_team_id,
            created_by: draft.created_by,
            entry_mode: EntryMode::SoloEntry,
            started_by: confirmed_by,
            origin: draft.origin,
            pairing_id: draft.pairing_id,
            home_fan_roll: None,
            away_fan_roll: None,
            home_dedicated_fans: DedicatedFans::default(),
            away_dedicated_fans: DedicatedFans::default(),
            home_team_value: None,
            away_team_value: None,
            home_inducements: None,
            away_inducements: None,
            home_inducements_recorded_by: None,
            away_inducements_recorded_by: None,
            star_engagements: vec![],
            home_temp_players: vec![],
            away_temp_players: vec![],
            home_actions: vec![],
            away_actions: vec![],
            drawn_mvps: vec![],
            version: draft.version + 1,
        }
    }

    /// **Second temps du rejeu** (carte 631) : `EntryModeChosen` remplace ce
    /// que `from_draft` a posé par défaut.
    pub(crate) fn choose_entry_mode(self, mode: EntryMode, started_by: CoachId) -> Self {
        Self {
            entry_mode: mode,
            started_by,
            version: self.version + 1,
            ..self
        }
    }
}

/// Le premier camp, domicile puis visiteur, dont les actions ne comptent aucun
/// MVP (carte 638). Partagée avec le rapport prêt à publier, que la correction
/// ramène aux mêmes étapes.
pub(crate) fn first_side_without_mvp(
    home_actions: &[MatchAction],
    away_actions: &[MatchAction],
) -> Option<TeamSide> {
    let has_mvp = |actions: &[MatchAction]| {
        actions
            .iter()
            .any(|a| matches!(a.action, MatchActionType::Mvp))
    };
    if !has_mvp(home_actions) {
        Some(TeamSide::Home)
    } else if !has_mvp(away_actions) {
        Some(TeamSide::Away)
    } else {
        None
    }
}

/// Ce qu'une action de match ne peut pas porter : une Haine sur une blessure
/// qui n'en donne pas, ou un MVP — qui n'entre que par `record_mvp` (carte
/// 637, décision du 2026-11-01 : la règle n'a pas de porte de service).
fn check_match_action(action: &MatchActionType) -> Result<(), DomainError> {
    match action {
        MatchActionType::Mvp => Err(DomainError::MvpOutsideMvpStep),
        MatchActionType::Blesse {
            injury,
            hatred: Some(_),
            ..
        } if !injury.peut_donner_haine() => Err(DomainError::HatredNotAllowedForInjury),
        _ => Ok(()),
    }
}

fn validate_purchases(
    purchases: &[(InducementId, u8)],
    allowed_specs: &[AllowedInducementSpec],
    opponent_star_uids: &[InducementId],
    budget: u32,
) -> Result<(), DomainError> {
    validate_max_qty(purchases, allowed_specs)?;
    validate_star_player_limit(purchases, allowed_specs)?;
    validate_star_player_conflict(purchases, allowed_specs, opponent_star_uids)?;
    validate_budget(purchases, allowed_specs, budget)?;
    validate_mercenary_limit(purchases)
}

fn validate_mercenary_limit(purchases: &[(InducementId, u8)]) -> Result<(), DomainError> {
    let total: u8 = purchases
        .iter()
        .filter(|(uid, _)| uid.0.starts_with("MERCO:"))
        .map(|(_, qty)| *qty)
        .sum();
    if total > MAX_MERCENARIES {
        Err(DomainError::TooManyMercenaries {
            requested: total,
            max: MAX_MERCENARIES,
        })
    } else {
        Ok(())
    }
}

fn validate_max_qty(
    purchases: &[(InducementId, u8)],
    allowed_specs: &[AllowedInducementSpec],
) -> Result<(), DomainError> {
    for (uid, qty) in purchases {
        // Un achat sans spec passait ici en silence — troisième avalement de la
        // même chaîne. Il est refusé au plus tôt, avant même la question du
        // plafond : c'est la phase de validation, pas celle de construction.
        let spec = allowed_specs
            .iter()
            .find(|s| &s.uid == uid)
            .ok_or_else(|| DomainError::UnknownInducement { uid: uid.0.clone() })?;
        if *qty > spec.max_qty.into_inner() {
            return Err(DomainError::MaxQtyExceeded {
                uid: uid.0.clone(),
                qty: *qty,
                max_qty: spec.max_qty.into_inner(),
            });
        }
    }
    Ok(())
}

fn validate_star_player_limit(
    purchases: &[(InducementId, u8)],
    allowed_specs: &[AllowedInducementSpec],
) -> Result<(), DomainError> {
    let star_count = purchases
        .iter()
        .filter(|(uid, _)| {
            allowed_specs
                .iter()
                .any(|s| &s.uid == uid && s.is_star_player.0)
        })
        .count();
    if star_count > MAX_STAR_PLAYERS {
        Err(DomainError::StarPlayerLimitExceeded)
    } else {
        Ok(())
    }
}

fn validate_star_player_conflict(
    purchases: &[(InducementId, u8)],
    allowed_specs: &[AllowedInducementSpec],
    opponent_star_uids: &[InducementId],
) -> Result<(), DomainError> {
    for (uid, _) in purchases {
        let is_star = allowed_specs
            .iter()
            .any(|s| &s.uid == uid && s.is_star_player.0);
        if is_star && opponent_star_uids.contains(uid) {
            return Err(DomainError::StarPlayerConflict { uid: uid.0.clone() });
        }
    }
    Ok(())
}

fn validate_budget(
    purchases: &[(InducementId, u8)],
    allowed_specs: &[AllowedInducementSpec],
    budget: u32,
) -> Result<(), DomainError> {
    let spent: u32 = purchases
        .iter()
        .filter_map(|(uid, qty)| {
            allowed_specs
                .iter()
                .find(|s| &s.uid == uid)
                .map(|s| s.unit_cost.into_inner() * (*qty as u32))
        })
        .sum();
    if spent > budget {
        Err(DomainError::BudgetExceeded { spent, budget })
    } else {
        Ok(())
    }
}

/// **Refuse** un achat sans spécification, au lieu de le filtrer.
///
/// Un `filter_map` le jetait sans un mot : le coup de pouce était facturé au
/// coach, puis n'existait plus nulle part — ni dans l'événement, ni dans les
/// joueurs temporaires. Aucune erreur, aucune ligne de journal (carte 406).
///
/// Un achat dont aucune spec ne porte l'uid est une incohérence d'appelant, pas
/// une donnée à ignorer. Le domaine ne journalise pas : il refuse, et c'est le
/// contrôleur qui en fait un 422 et une ligne.
fn build_purchase_list(
    purchases: &[(InducementId, u8)],
    allowed_specs: &[AllowedInducementSpec],
) -> Result<Vec<InducementPurchase>, DomainError> {
    purchases
        .iter()
        .map(|(uid, qty)| {
            let spec = allowed_specs
                .iter()
                .find(|s| &s.uid == uid)
                .ok_or_else(|| DomainError::UnknownInducement { uid: uid.0.clone() })?;
            Ok(InducementPurchase {
                uid: uid.clone(),
                qty: InducementQty::try_new(*qty).expect("qty validated at IO boundary"),
                unit_cost: spec.unit_cost,
            })
        })
        .collect()
}

fn build_inducement_events(
    team_id: &TeamId,
    purchase_list: &[InducementPurchase],
    allowed_specs: &[AllowedInducementSpec],
    recorded_by: CoachId,
) -> Vec<MatchReportDomainEvent> {
    let mut events = vec![MatchReportDomainEvent::InducementsRecorded {
        team_id: team_id.clone(),
        purchases: purchase_list.to_vec(),
        recorded_by: recorded_by.clone(),
    }];
    for p in purchase_list {
        if allowed_specs
            .iter()
            .any(|s| s.uid == p.uid && s.is_star_player.0)
        {
            events.push(MatchReportDomainEvent::StarPlayerEngaged {
                team_id: team_id.clone(),
                star_player_uid: p.uid.clone(),
                recorded_by: recorded_by.clone(),
            });
        }
    }
    events
}

fn set_inducements_for(
    pm: &mut MatchReportPreMatch,
    team_id: &TeamId,
    purchases: Vec<InducementPurchase>,
    recorded_by: CoachId,
) {
    if team_id == &pm.home_team_id {
        pm.home_inducements = Some(purchases);
        pm.home_inducements_recorded_by = Some(recorded_by);
    } else {
        pm.away_inducements = Some(purchases);
        pm.away_inducements_recorded_by = Some(recorded_by);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::{
        InducementCost, InducementQty, InjuryType, IsStarPlayer, MatchReportOrigin,
    };
    use crate::app::shared_kernel::bloodbowl::ids::{
        CompetitionId, MatchReportId, RoundId, SeasonId,
    };
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};

    fn make_pm(home_tv: u32, away_tv: u32) -> MatchReportPreMatch {
        let home_id = TeamId::new();
        let away_id = TeamId::new();
        MatchReportPreMatch {
            id: MatchReportId::new(),
            space_id: SpaceId::new(),
            competition_id: CompetitionId::new(),
            season_id: SeasonId::new(),
            round_id: RoundId::new(),
            home_team_id: home_id,
            away_team_id: away_id,
            created_by: CoachId::new(),
            entry_mode: EntryMode::SoloEntry,
            started_by: CoachId::new(),
            origin: MatchReportOrigin::Manual,
            pairing_id: None,
            home_fan_roll: None,
            away_fan_roll: None,
            home_dedicated_fans: DedicatedFans::default(),
            away_dedicated_fans: DedicatedFans::default(),
            home_team_value: Some(TeamValue::try_new(home_tv).unwrap()),
            away_team_value: Some(TeamValue::try_new(away_tv).unwrap()),
            home_inducements: None,
            away_inducements: None,
            home_inducements_recorded_by: None,
            away_inducements_recorded_by: None,
            star_engagements: vec![],
            home_temp_players: vec![],
            away_temp_players: vec![],
            home_actions: vec![],
            away_actions: vec![],
            drawn_mvps: vec![],
            version: 1,
        }
    }

    fn spec(uid: &str, max_qty: u8, unit_cost: u32, is_star: bool) -> AllowedInducementSpec {
        AllowedInducementSpec {
            uid: InducementId(uid.to_string()),
            max_qty: InducementMaxQty::try_new(max_qty).unwrap(),
            unit_cost: InducementCost::try_new(unit_cost).unwrap(),
            is_star_player: IsStarPlayer(is_star),
        }
    }

    #[test]
    fn topdog_is_home_when_tv_equal() {
        let pm = make_pm(1000, 1000);
        assert_eq!(pm.topdog_team_id(), &pm.home_team_id);
    }

    #[test]
    fn topdog_is_away_when_away_higher_tv() {
        let pm = make_pm(1000, 1100);
        assert_eq!(pm.topdog_team_id(), &pm.away_team_id);
    }

    #[test]
    fn topdog_budget_equals_treasury() {
        let pm = make_pm(1000, 1000);
        assert_eq!(pm.inducement_budget_for(&pm.home_team_id, 80), 80);
    }

    #[test]
    fn underdog_budget_includes_tv_diff_and_topdog_spending() {
        // diff = 100 kPo, treasury = 0
        let pm = make_pm(1100, 1000);
        assert_eq!(pm.inducement_budget_for(&pm.away_team_id, 0), 100);
    }

    // ── Ce qui sort réellement de la caisse ───────────────────────────────
    //
    // Le montant acheté n'est pas le montant payé : la petite monnaie de
    // l'underdog ne vient d'aucune trésorerie. Ces tests sont ceux du bug —
    // aucun débit n'avait lieu avant qu'ils existent.

    fn achat(cout: u32, qty: u8) -> InducementPurchase {
        InducementPurchase {
            uid: InducementId("BLOODWEISER".into()),
            qty: InducementQty::try_new(qty).unwrap(),
            unit_cost: InducementCost::try_new(cout).unwrap(),
        }
    }

    #[test]
    fn le_topdog_paie_l_integralite_de_ses_achats() {
        let mut pm = make_pm(1100, 1000);
        pm.home_inducements = Some(vec![achat(50, 2)]);
        assert_eq!(pm.treasury_spending_for(&pm.home_team_id), 100);
    }

    /// La petite monnaie couvre tout : rien ne sort de la trésorerie. C'est le
    /// cas le plus courant, et celui qu'un débit naïf du montant acheté aurait
    /// facturé à tort.
    #[test]
    fn l_underdog_ne_paie_rien_sous_sa_petite_monnaie() {
        let mut pm = make_pm(1100, 1000); // écart de 100
        pm.away_inducements = Some(vec![achat(30, 2)]); // 60 acheté
        assert_eq!(pm.petty_cash(), 100);
        assert_eq!(pm.treasury_spending_for(&pm.away_team_id), 0);
    }

    #[test]
    fn l_underdog_ne_paie_que_le_depassement() {
        let mut pm = make_pm(1100, 1000); // écart de 100
        pm.away_inducements = Some(vec![achat(50, 3)]); // 150 acheté
        assert_eq!(pm.treasury_spending_for(&pm.away_team_id), 50);
    }

    /// La petite monnaie grandit avec les achats du top dog : ce que l'underdog
    /// paie de sa poche diminue d'autant. Sans ce lien, il paierait un écart
    /// que le top dog vient lui-même de creuser.
    #[test]
    fn les_achats_du_topdog_reduisent_ce_que_paie_l_underdog() {
        let mut pm = make_pm(1100, 1000);
        pm.away_inducements = Some(vec![achat(50, 3)]); // 150 acheté
        assert_eq!(pm.treasury_spending_for(&pm.away_team_id), 50);

        pm.home_inducements = Some(vec![achat(40, 1)]); // le top dog dépense 40
        assert_eq!(pm.petty_cash(), 140);
        assert_eq!(pm.treasury_spending_for(&pm.away_team_id), 10);
    }

    /// Conséquence du garde-fou, vérifiée plutôt que supposée : le budget
    /// plafonne l'appoint à 50 kPo, donc le dépassement ne peut jamais aller
    /// au-delà. C'est pour ça que `treasury_spending_for` ne réapplique pas le
    /// plafond — il serait alors écrit à deux endroits.
    #[test]
    fn le_depassement_ne_peut_pas_exceder_le_plafond_de_50() {
        let pm = make_pm(1100, 1000);
        let budget = pm.inducement_budget_for(&pm.away_team_id, 500);
        assert_eq!(budget, 150, "petite monnaie 100 + 50 au plus");
        assert_eq!(budget - pm.petty_cash(), 50);
    }

    /// Un rapport publié sans jamais passer par l'étape des coups de pouce n'a
    /// pas de valeurs d'équipe. La méthode doit rester totale : c'est ce qui
    /// l'a fait paniquer sur six tests avant d'être gardée.
    #[test]
    fn sans_valeurs_d_equipe_la_depense_est_nulle() {
        let mut pm = make_pm(1100, 1000);
        pm.home_team_value = None;
        pm.away_team_value = None;
        assert_eq!(pm.treasury_spending_for(&pm.home_team_id), 0);
    }

    #[test]
    fn underdog_budget_caps_treasury_at_50k() {
        // diff = 100 kPo, treasury = 80 kPo → capped at 50 kPo → total = 150 kPo
        let pm = make_pm(1100, 1000);
        assert_eq!(pm.inducement_budget_for(&pm.away_team_id, 80), 150);
    }

    #[test]
    fn underdog_budget_uses_full_treasury_when_below_50k() {
        // diff = 100 kPo, treasury = 30 kPo < 50 kPo cap → total = 130 kPo
        let pm = make_pm(1100, 1000);
        assert_eq!(pm.inducement_budget_for(&pm.away_team_id, 30), 130);
    }

    #[test]
    fn topdog_spending_zero_before_purchase() {
        let pm = make_pm(1100, 1000);
        assert_eq!(pm.topdog_spending(), 0);
    }

    #[test]
    fn topdog_spending_zero_when_passed() {
        let mut pm = make_pm(1100, 1000);
        pm.home_inducements = Some(vec![]);
        assert_eq!(pm.topdog_spending(), 0);
    }

    #[test]
    fn record_inducements_fails_on_budget_exceeded() {
        let pm = make_pm(1000, 1000);
        let specs = vec![spec("BRIBE", 2, 100, false)];
        let result = pm.record_inducements(
            &pm.home_team_id.clone(),
            &[(InducementId("BRIBE".into()), 1)],
            50,
            &specs,
            &[],
            CoachId::new(),
        );
        assert!(matches!(result, Err(DomainError::BudgetExceeded { .. })));
    }

    #[test]
    fn record_inducements_fails_on_max_qty_exceeded() {
        let pm = make_pm(1000, 1000);
        let specs = vec![spec("BRIBE", 1, 10, false)];
        let result = pm.record_inducements(
            &pm.home_team_id.clone(),
            &[(InducementId("BRIBE".into()), 2)],
            100,
            &specs,
            &[],
            CoachId::new(),
        );
        assert!(matches!(result, Err(DomainError::MaxQtyExceeded { .. })));
    }

    #[test]
    fn record_inducements_fails_when_star_player_limit_exceeded() {
        let pm = make_pm(1000, 1000);
        let specs = vec![
            spec("SP1", 1, 10_000, true),
            spec("SP2", 1, 10_000, true),
            spec("SP3", 1, 10_000, true),
        ];
        let result = pm.record_inducements(
            &pm.home_team_id.clone(),
            &[
                (InducementId("SP1".into()), 1),
                (InducementId("SP2".into()), 1),
                (InducementId("SP3".into()), 1),
            ],
            1_000_000,
            &specs,
            &[],
            CoachId::new(),
        );
        assert!(matches!(result, Err(DomainError::StarPlayerLimitExceeded)));
    }

    #[test]
    fn record_inducements_fails_on_star_player_conflict() {
        let pm = make_pm(1000, 1000);
        let specs = vec![spec("SP1", 1, 10_000, true)];
        let opponent = vec![InducementId("SP1".into())];
        let result = pm.record_inducements(
            &pm.home_team_id.clone(),
            &[(InducementId("SP1".into()), 1)],
            1_000_000,
            &specs,
            &opponent,
            CoachId::new(),
        );
        assert!(matches!(
            result,
            Err(DomainError::StarPlayerConflict { .. })
        ));
    }

    #[test]
    fn record_inducements_with_empty_purchases_succeeds() {
        let pm = make_pm(1000, 1000);
        let result =
            pm.record_inducements(&pm.home_team_id.clone(), &[], 0, &[], &[], CoachId::new());
        assert!(result.is_ok());
        let (updated, events) = result.unwrap();
        assert!(updated.home_inducements.as_ref().unwrap().is_empty());
        assert_eq!(events.len(), 1);
    }

    /// Un achat sans spécification **refuse**, il ne s'évapore plus.
    ///
    /// C'est le maillon qui rendait le défaut silencieux : le coup de pouce
    /// était facturé, puis jeté par un `filter_map` sans erreur ni journal
    /// (carte 406). Le domaine ne journalise pas — il refuse, et le contrôleur
    /// en fait un 422.
    #[test]
    fn un_achat_sans_specification_est_refuse() {
        let pm = make_pm(1000, 1000);
        let achats = vec![(InducementId("MERCO:INCONNU:base".to_string()), 1)];

        let erreur = pm
            .record_inducements(
                &pm.home_team_id.clone(),
                &achats,
                100_000,
                &[],
                &[],
                CoachId::new(),
            )
            .expect_err("un achat sans spec est une incohérence, pas une donnée à ignorer");

        assert!(
            matches!(erreur, DomainError::UnknownInducement { ref uid } if uid == "MERCO:INCONNU:base"),
            "l'erreur doit nommer le coup de pouce : {erreur:?}"
        );
    }

    /// Un plafond de corpus au-delà de dix ne disqualifie plus le coup de
    /// pouce. Le corpus d'exemple plafonne à cinq, ce qui masquait le défaut
    /// sur ce chemin — il aurait mordu au premier référentiel plus généreux.
    #[test]
    fn un_plafond_de_corpus_a_douze_reste_utilisable() {
        let pm = make_pm(1000, 1000);
        let specs = vec![spec("APOTHICAIRE", 12, 50, false)];
        let achats = vec![(InducementId("APOTHICAIRE".to_string()), 10)];

        let (mise_a_jour, _) = pm
            .record_inducements(
                &pm.home_team_id.clone(),
                &achats,
                100_000,
                &specs,
                &[],
                CoachId::new(),
            )
            .expect("douze est un plafond légitime");

        assert_eq!(mise_a_jour.home_inducements.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn record_inducements_emits_star_player_engaged_per_star() {
        let pm = make_pm(1000, 1000);
        let specs = vec![spec("SP1", 1, 10_000, true), spec("SP2", 1, 10_000, true)];
        let (_, events) = pm
            .record_inducements(
                &pm.home_team_id.clone(),
                &[
                    (InducementId("SP1".into()), 1),
                    (InducementId("SP2".into()), 1),
                ],
                1_000_000,
                &specs,
                &[],
                CoachId::new(),
            )
            .unwrap();
        assert_eq!(events.len(), 3);
        assert!(matches!(
            &events[1],
            MatchReportDomainEvent::StarPlayerEngaged { .. }
        ));
        assert!(matches!(
            &events[2],
            MatchReportDomainEvent::StarPlayerEngaged { .. }
        ));
    }

    #[test]
    fn record_inducements_no_star_player_engaged_when_none_hired() {
        let pm = make_pm(1000, 1000);
        let specs = vec![spec("BRIBE", 2, 10_000, false)];
        let (_, events) = pm
            .record_inducements(
                &pm.home_team_id.clone(),
                &[(InducementId("BRIBE".into()), 1)],
                1_000_000,
                &specs,
                &[],
                CoachId::new(),
            )
            .unwrap();
        assert_eq!(events.len(), 1);
    }

    // ── step3-4 : temp players ────────────────────────────────────────────────

    use crate::app::match_report::domain::value_objects::{TempPlayerId, TempPlayerKind};

    fn make_journeyman(pm: &MatchReportPreMatch) -> TempPlayer {
        TempPlayer {
            id: TempPlayerId("tp-01".to_string()),
            team_id: pm.home_team_id.clone(),
            kind: TempPlayerKind::Journeyman {
                position_uid: "LIN".to_string(),
            },
            display_name: None,
        }
    }

    #[test]
    fn init_temp_players_sets_list() {
        let pm = make_pm(1000, 1000);
        let player = make_journeyman(&pm);
        let (updated, event) = pm.init_temp_players(&pm.home_team_id.clone(), vec![player]);
        assert_eq!(updated.home_temp_players.len(), 1);
        assert!(updated.away_temp_players.is_empty());
        assert!(matches!(
            event,
            MatchReportDomainEvent::TempPlayersInitialized { .. }
        ));
        assert_eq!(updated.version, pm.version + 1);
    }

    /// Carte 455 — le reset **nomme** les journaliers qu'il retire.
    ///
    /// Le publisher s'exécute après l'append : l'agrégat ne les porte déjà plus
    /// quand il lit l'événement. Sans cette liste, un repassage sur l'écran des
    /// coups de pouce laisserait derrière lui des journaliers orphelins.
    #[test]
    fn le_reset_nomme_les_journaliers_retires() {
        let pm = make_pm(1000, 1000);
        let journalier = make_journeyman(&pm);
        let id = journalier.id.clone();
        let (avec, _) = pm.init_temp_players(&pm.home_team_id.clone(), vec![journalier]);

        let (_, event) = avec.reset_temp_players(&avec.home_team_id.clone());

        let MatchReportDomainEvent::TempPlayersReset { withdrawn, .. } = event else {
            panic!("un reset");
        };
        assert_eq!(withdrawn, vec![id]);
    }

    #[test]
    fn reset_temp_players_clears_list() {
        let pm = make_pm(1000, 1000);
        let player = make_journeyman(&pm);
        let (with_players, _) = pm.init_temp_players(&pm.home_team_id.clone(), vec![player]);
        let (reset, event) = with_players.reset_temp_players(&pm.home_team_id.clone());
        assert!(reset.home_temp_players.is_empty());
        assert!(matches!(
            event,
            MatchReportDomainEvent::TempPlayersReset { .. }
        ));
    }

    // ── step3-4 : actions ─────────────────────────────────────────────────────

    use crate::app::match_report::domain::value_objects::{
        ActionId, ActionPlayer, MatchActionType, TeamSide, TurnNumber,
    };
    use crate::app::shared_kernel::bloodbowl::ids::PlayerId;

    fn make_action(
        pm: &MatchReportPreMatch,
        side: TeamSide,
    ) -> (MatchReportPreMatch, MatchReportDomainEvent) {
        pm.record_action(
            side,
            TurnNumber::try_new(3).unwrap(),
            ActionPlayer::Regular(PlayerId::new()),
            MatchActionType::Touchdown,
            "Jean Dupont (#5)".to_string(),
            "Blitzeur".to_string(),
            ActionId("act-01".to_string()),
            CoachId::new(),
        )
        .expect("action de test valide")
    }

    #[test]
    fn record_action_pushes_to_home_actions() {
        let pm = make_pm(1000, 1000);
        let (updated, _) = make_action(&pm, TeamSide::Home);
        assert_eq!(updated.home_actions.len(), 1);
        assert!(updated.away_actions.is_empty());
    }

    #[test]
    fn record_action_pushes_to_away_actions() {
        let pm = make_pm(1000, 1000);
        let (updated, _) = make_action(&pm, TeamSide::Away);
        assert_eq!(updated.away_actions.len(), 1);
        assert!(updated.home_actions.is_empty());
    }

    #[test]
    fn record_two_actions_same_player_same_turn() {
        let pm = make_pm(1000, 1000);
        let (pm2, _) = make_action(&pm, TeamSide::Home);
        let (pm3, _) = make_action(&pm2, TeamSide::Home);
        assert_eq!(pm3.home_actions.len(), 2);
    }

    #[test]
    fn record_two_mvp_same_team() {
        // Carte 637 : le second MVP d'un camp est refusé — l'ancien test
        // fixait l'inverse, quand aucune règle n'existait.
        let (pm2, _) = mvp(&make_pm(1000, 1000), TeamSide::Home, "a1").unwrap();
        assert_eq!(
            mvp(&pm2, TeamSide::Home, "a2").unwrap_err(),
            DomainError::MvpAlreadyRecorded
        );
        assert_eq!(pm2.home_actions.len(), 1);
    }

    // ── Le MVP (carte 637) ───────────────────────────────────────────────

    fn mvp(
        pm: &MatchReportPreMatch,
        side: TeamSide,
        id: &str,
    ) -> Result<(MatchReportPreMatch, MatchReportDomainEvent), DomainError> {
        pm.record_mvp(
            side,
            ActionPlayer::Regular(PlayerId::new()),
            "Thorgrim (#4)".into(),
            "Tueur de trolls".into(),
            ActionId(id.into()),
            CoachId::new(),
        )
    }

    #[test]
    fn le_mvp_s_inscrit_au_tour_16() {
        let (pm, event) = mvp(&make_pm(1000, 1000), TeamSide::Away, "m1").unwrap();
        assert_eq!(pm.away_actions.len(), 1);
        assert_eq!(pm.away_actions[0].turn.value(), 16);
        assert!(matches!(pm.away_actions[0].action, MatchActionType::Mvp));
        let MatchReportDomainEvent::ActionRecorded { turn, action, .. } = event else {
            panic!("attendu ActionRecorded");
        };
        assert_eq!(turn.value(), 16);
        assert!(matches!(action, MatchActionType::Mvp));
    }

    #[test]
    fn un_mvp_par_camp_et_non_par_match() {
        let (pm, _) = mvp(&make_pm(1000, 1000), TeamSide::Home, "m1").unwrap();
        let (pm, _) = mvp(&pm, TeamSide::Away, "m2").unwrap();
        assert!(pm.has_mvp(TeamSide::Home) && pm.has_mvp(TeamSide::Away));
    }

    /// Un MVP saisi avant la règle, au tour 7 : il compte, quel que soit son
    /// tour.
    #[test]
    fn un_ancien_mvp_hors_tour_16_bloque_le_second() {
        let mut pm = make_pm(1000, 1000);
        pm.home_actions.push(MatchAction {
            id: ActionId("ancien".into()),
            turn: TurnNumber::try_new(7).unwrap(),
            player: ActionPlayer::Regular(PlayerId::new()),
            action: MatchActionType::Mvp,
            player_display_name: "Borin".into(),
            player_position: String::new(),
        });
        assert_eq!(
            mvp(&pm, TeamSide::Home, "m1").unwrap_err(),
            DomainError::MvpAlreadyRecorded
        );
    }

    #[test]
    fn supprimer_le_mvp_permet_d_en_designer_un_autre() {
        let (pm, _) = mvp(&make_pm(1000, 1000), TeamSide::Home, "m1").unwrap();
        let (pm, _) = pm
            .delete_action(&ActionId("m1".into()), CoachId::new())
            .unwrap();
        assert!(mvp(&pm, TeamSide::Home, "m2").is_ok());
    }

    /// Chaque camp avec son MVP : l'après-match peut se clore (carte 638).
    fn with_both_mvps(pm: MatchReportPreMatch) -> MatchReportPreMatch {
        let (pm, _) = mvp(&pm, TeamSide::Home, "mvp-home").unwrap();
        let (pm, _) = mvp(&pm, TeamSide::Away, "mvp-away").unwrap();
        pm
    }

    // ── Le tirage au sort du MVP (carte 638) ──────────────────────────────

    fn six_designes() -> Vec<DesignatedPlayer> {
        ["Thorgrim", "Borin", "Durin", "Balin", "Nori", "Gimli"]
            .iter()
            .map(|name| DesignatedPlayer {
                player: ActionPlayer::Regular(PlayerId::new()),
                display_name: name.to_string(),
                position: "Bloqueur".into(),
            })
            .collect()
    }

    fn draw(
        pm: &MatchReportPreMatch,
        designated: Vec<DesignatedPlayer>,
        seed: u64,
    ) -> Result<(MatchReportPreMatch, Vec<MatchReportDomainEvent>), DomainError> {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        pm.draw_mvp(
            TeamSide::Home,
            designated,
            &mut rng,
            ActionId("tire".into()),
            CoachId::new(),
        )
    }

    #[test]
    fn le_tirage_exige_six_designes() {
        let mut cinq = six_designes();
        cinq.pop();
        assert_eq!(
            draw(&make_pm(1000, 1000), cinq, 1).unwrap_err(),
            DomainError::DrawNeedsSixPlayers(5)
        );
    }

    #[test]
    fn le_tirage_refuse_un_doublon() {
        let mut list = six_designes();
        list[4] = list[0].clone();
        assert_eq!(
            draw(&make_pm(1000, 1000), list, 1).unwrap_err(),
            DomainError::DuplicateDesignatedPlayer
        );
    }

    /// Sur plusieurs graines fixes, le tiré est toujours l'un des six, au tour
    /// 16, et l'agrégat retient le tirage.
    #[test]
    fn le_tire_est_l_un_des_six_au_tour_16() {
        for seed in 0..20 {
            let designated = six_designes();
            let (pm, _) = draw(&make_pm(1000, 1000), designated.clone(), seed).unwrap();
            let action = &pm.home_actions[0];
            assert_eq!(action.turn.value(), 16);
            assert!(matches!(action.action, MatchActionType::Mvp));
            assert!(designated.iter().any(|d| d.player == action.player));
            let drawn = pm.drawn_mvp(&action.id).expect("le tirage est retenu");
            assert_eq!(drawn.designated, designated);
        }
    }

    #[test]
    fn le_tirage_rend_l_action_puis_le_tirage() {
        let (pm, events) = draw(&make_pm(1000, 1000), six_designes(), 3).unwrap();
        let [MatchReportDomainEvent::ActionRecorded { action_id, .. }, MatchReportDomainEvent::MvpDrawn {
            action_id: drawn_id,
            team_side,
            designated,
        }] = events.as_slice()
        else {
            panic!("attendu ActionRecorded puis MvpDrawn : {events:?}");
        };
        assert_eq!(action_id, drawn_id);
        assert_eq!(*team_side, TeamSide::Home);
        assert_eq!(designated.len(), 6);
        // Deux événements, deux versions.
        assert_eq!(pm.version, make_pm(1000, 1000).version + 2);
    }

    #[test]
    fn un_camp_qui_a_son_mvp_ne_tire_pas() {
        let (pm, _) = mvp(&make_pm(1000, 1000), TeamSide::Home, "m1").unwrap();
        assert_eq!(
            draw(&pm, six_designes(), 1).unwrap_err(),
            DomainError::MvpAlreadyRecorded
        );
    }

    #[test]
    fn un_mvp_tire_ne_se_supprime_pas() {
        let (pm, _) = draw(&make_pm(1000, 1000), six_designes(), 5).unwrap();
        assert_eq!(
            pm.delete_action(&ActionId("tire".into()), CoachId::new())
                .unwrap_err(),
            DomainError::DrawnMvpIsFinal
        );
    }

    /// Un MVP saisi à la main, lui, se supprime toujours (carte 637).
    #[test]
    fn un_mvp_saisi_a_la_main_se_supprime_encore() {
        let (pm, _) = mvp(&make_pm(1000, 1000), TeamSide::Home, "m1").unwrap();
        assert!(pm
            .delete_action(&ActionId("m1".into()), CoachId::new())
            .is_ok());
    }

    // ── Le MVP obligatoire (carte 638) ────────────────────────────────────

    fn gains() -> (
        MatchGain,
        MatchGain,
        NoStallingBonuses,
        FanFactorMod,
        FanFactorMod,
    ) {
        (
            MatchGain::try_new(10).unwrap(),
            MatchGain::try_new(10).unwrap(),
            NoStallingBonuses::default(),
            FanFactorMod::try_new(0).unwrap(),
            FanFactorMod::try_new(0).unwrap(),
        )
    }

    fn close(pm: &MatchReportPreMatch) -> Result<MatchReportReadyToPublish, DomainError> {
        let (home, away, bonuses, home_mod, away_mod) = gains();
        pm.record_post_match(
            home,
            away,
            bonuses,
            home_mod,
            away_mod,
            None,
            None,
            CoachId::new(),
        )
        .map(|(ready, _)| ready)
    }

    #[test]
    fn sans_mvp_l_apres_match_ne_se_clot_pas() {
        let pm = make_pm(1000, 1000);
        assert_eq!(pm.first_side_without_mvp(), Some(TeamSide::Home));
        assert_eq!(
            close(&pm).unwrap_err(),
            DomainError::MvpMissing(TeamSide::Home)
        );
    }

    #[test]
    fn le_visiteur_sans_mvp_bloque_aussi() {
        let (pm, _) = mvp(&make_pm(1000, 1000), TeamSide::Home, "m1").unwrap();
        assert_eq!(pm.first_side_without_mvp(), Some(TeamSide::Away));
        assert_eq!(
            close(&pm).unwrap_err(),
            DomainError::MvpMissing(TeamSide::Away)
        );
    }

    #[test]
    fn avec_ses_deux_mvp_l_apres_match_se_clot() {
        let pm = with_both_mvps(make_pm(1000, 1000));
        assert_eq!(pm.first_side_without_mvp(), None);
        assert!(close(&pm).is_ok());
    }

    /// En correction, un MVP supprimé bloque de nouveau la clôture.
    #[test]
    fn en_correction_le_mvp_reste_obligatoire() {
        let ready = close(&with_both_mvps(make_pm(1000, 1000))).unwrap();
        let (pm, _) = ready
            .into_pre_match()
            .delete_action(&ActionId("mvp-away".into()), CoachId::new())
            .unwrap();
        let (home, away, bonuses, home_mod, away_mod) = gains();
        let ready_again = MatchReportReadyToPublish::from_pre_match(
            &pm, home, away, bonuses, home_mod, away_mod, None, None,
        );
        assert_eq!(ready_again.first_side_without_mvp(), Some(TeamSide::Away));
        let (home, away, bonuses, home_mod, away_mod) = gains();
        let refus = ready_again.record_post_match(
            home,
            away,
            bonuses,
            home_mod,
            away_mod,
            None,
            None,
            CoachId::new(),
        );
        assert_eq!(refus.unwrap_err(), DomainError::MvpMissing(TeamSide::Away));
    }

    #[test]
    fn une_action_mvp_est_refusee() {
        let r = make_pm(1000, 1000).record_action(
            TeamSide::Home,
            TurnNumber::try_new(16).unwrap(),
            ActionPlayer::Regular(PlayerId::new()),
            MatchActionType::Mvp,
            "A".into(),
            String::new(),
            ActionId("a1".into()),
            CoachId::new(),
        );
        assert_eq!(r.unwrap_err(), DomainError::MvpOutsideMvpStep);
    }

    #[test]
    fn delete_action_removes_entry() {
        let pm = make_pm(1000, 1000);
        let (pm2, _) = make_action(&pm, TeamSide::Home);
        let action_id = pm2.home_actions[0].id.clone();
        let (pm3, event) = pm2.delete_action(&action_id, CoachId::new()).unwrap();
        assert!(pm3.home_actions.is_empty());
        assert!(matches!(
            event,
            MatchReportDomainEvent::ActionDeleted { .. }
        ));
    }

    #[test]
    fn delete_action_fails_when_not_found() {
        let pm = make_pm(1000, 1000);
        let result = pm.delete_action(&ActionId("missing".to_string()), CoachId::new());
        assert!(matches!(result, Err(DomainError::ActionNotFound(_))));
    }

    #[test]
    fn actions_for_returns_correct_side() {
        let pm = make_pm(1000, 1000);
        let (pm2, _) = make_action(&pm, TeamSide::Home);
        assert_eq!(pm2.actions_for(TeamSide::Home).len(), 1);
        assert_eq!(pm2.actions_for(TeamSide::Away).len(), 0);
    }

    #[test]
    fn star_player_uids_for_returns_engaged_uids() {
        let mut pm = make_pm(1000, 1000);
        let uid = InducementId("SP1".to_string());
        pm.star_engagements
            .push((pm.home_team_id.clone(), uid.clone()));
        let result = pm.star_player_uids_for(&pm.home_team_id.clone());
        assert_eq!(result, vec![uid]);
        assert!(pm.star_player_uids_for(&pm.away_team_id.clone()).is_empty());
    }

    #[test]
    fn purchases_for_returns_team_inducements() {
        let mut pm = make_pm(1000, 1000);
        pm.home_inducements = Some(vec![]);
        assert_eq!(pm.purchases_for(&pm.home_team_id.clone()).len(), 0);
        assert_eq!(pm.purchases_for(&pm.away_team_id.clone()).len(), 0);
    }

    // ── Mercenaires ───────────────────────────────────────────────────────────

    fn merco_spec(
        position_uid: &str,
        level: &str,
        max_qty: u8,
        cost: u32,
    ) -> AllowedInducementSpec {
        spec(
            &format!("MERCO:{position_uid}:{level}"),
            max_qty,
            cost,
            false,
        )
    }

    #[test]
    fn record_inducements_fails_when_more_than_3_mercos() {
        let pm = make_pm(1000, 1000);
        let specs = vec![
            merco_spec("pos-a", "base", 4, 100),
            merco_spec("pos-b", "base", 4, 100),
        ];
        let result = pm.record_inducements(
            &pm.home_team_id.clone(),
            &[
                (InducementId("MERCO:pos-a:base".into()), 2),
                (InducementId("MERCO:pos-b:base".into()), 2),
            ],
            10_000,
            &specs,
            &[],
            CoachId::new(),
        );
        assert!(matches!(
            result,
            Err(DomainError::TooManyMercenaries {
                requested: 4,
                max: 3
            })
        ));
    }

    #[test]
    fn record_inducements_with_exactly_3_mercos_succeeds() {
        let pm = make_pm(1000, 1000);
        let specs = vec![merco_spec("pos-a", "base", 3, 100)];
        let result = pm.record_inducements(
            &pm.home_team_id.clone(),
            &[(InducementId("MERCO:pos-a:base".into()), 3)],
            10_000,
            &specs,
            &[],
            CoachId::new(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn record_inducements_merco_count_is_sum_of_qtys() {
        let pm = make_pm(1000, 1000);
        let specs = vec![
            merco_spec("pos-a", "base", 3, 100),
            merco_spec("pos-b", "lvl1", 3, 180),
        ];
        let result = pm.record_inducements(
            &pm.home_team_id.clone(),
            &[
                (InducementId("MERCO:pos-a:base".into()), 2),
                (InducementId("MERCO:pos-b:lvl1".into()), 2),
            ],
            10_000,
            &specs,
            &[],
            CoachId::new(),
        );
        assert!(matches!(
            result,
            Err(DomainError::TooManyMercenaries {
                requested: 4,
                max: 3
            })
        ));
    }

    #[test]
    fn record_inducements_merco_respects_position_max_qty() {
        let pm = make_pm(1000, 1000);
        let specs = vec![merco_spec("pos-a", "base", 1, 100)];
        let result = pm.record_inducements(
            &pm.home_team_id.clone(),
            &[(InducementId("MERCO:pos-a:base".into()), 2)],
            10_000,
            &specs,
            &[],
            CoachId::new(),
        );
        assert!(matches!(result, Err(DomainError::MaxQtyExceeded { .. })));
    }

    #[test]
    fn record_inducements_merco_cost_counts_toward_budget() {
        let pm = make_pm(1000, 1000);
        let specs = vec![merco_spec("pos-a", "base", 1, 180)];
        let result = pm.record_inducements(
            &pm.home_team_id.clone(),
            &[(InducementId("MERCO:pos-a:base".into()), 1)],
            100,
            &specs,
            &[],
            CoachId::new(),
        );
        assert!(matches!(result, Err(DomainError::BudgetExceeded { .. })));
    }

    #[test]
    fn record_inducements_with_mercos_and_classic_succeed() {
        let pm = make_pm(1000, 1000);
        let specs = vec![
            spec("BRIBE", 1, 50, false),
            merco_spec("pos-a", "base", 2, 130),
        ];
        let result = pm.record_inducements(
            &pm.home_team_id.clone(),
            &[
                (InducementId("BRIBE".into()), 1),
                (InducementId("MERCO:pos-a:base".into()), 2),
            ],
            1_000,
            &specs,
            &[],
            CoachId::new(),
        );
        assert!(result.is_ok());
        let (updated, _) = result.unwrap();
        assert_eq!(updated.home_inducements.as_ref().unwrap().len(), 2);
    }

    // ── Haine (carte 400) ────────────────────────────────────────────────────

    use crate::app::match_report::domain::value_objects::{HatredKeyword, SequelStat};

    fn enregistrer_blessure(
        injury: InjuryType,
        hatred: Option<HatredKeyword>,
    ) -> Result<(MatchReportPreMatch, MatchReportDomainEvent), DomainError> {
        let pm = make_pm(1000, 1000);
        pm.record_action(
            TeamSide::Home,
            TurnNumber::try_new(1).unwrap(),
            ActionPlayer::Regular(PlayerId::new()),
            MatchActionType::Blesse {
                injury,
                hatred,
                hatred_skill_uid: None,
            },
            "A".into(),
            "B".into(),
            ActionId("h1".into()),
            CoachId::new(),
        )
    }

    fn haine() -> Option<HatredKeyword> {
        Some(HatredKeyword::try_new("DARK_ELF").unwrap())
    }

    #[test]
    fn une_haine_est_acceptee_sur_amoche_serieuse_et_sequelle() {
        for injury in [
            InjuryType::Amoche,
            InjuryType::BlessureSerieuse,
            InjuryType::Sequel {
                stat: SequelStat::MinusMa,
            },
        ] {
            assert!(
                enregistrer_blessure(injury.clone(), haine()).is_ok(),
                "{injury:?} doit permettre une Haine"
            );
        }
    }

    #[test]
    fn une_haine_est_refusee_sur_commotion_et_sur_mort() {
        for injury in [InjuryType::Commotion, InjuryType::Mort] {
            assert_eq!(
                enregistrer_blessure(injury.clone(), haine()).err(),
                Some(DomainError::HatredNotAllowedForInjury),
                "{injury:?} ne doit pas permettre de Haine"
            );
        }
    }

    /// La règle interdit la Haine sur une Commotion, **pas la Commotion**.
    ///
    /// Sans ce test, un domaine qui refuserait toute blessure légère passerait
    /// les deux précédents sans qu'on le voie.
    #[test]
    fn une_blessure_sans_haine_reste_acceptee_sur_les_cinq_types() {
        for injury in [
            InjuryType::Commotion,
            InjuryType::Amoche,
            InjuryType::BlessureSerieuse,
            InjuryType::Sequel {
                stat: SequelStat::MinusSt,
            },
            InjuryType::Mort,
        ] {
            assert!(
                enregistrer_blessure(injury.clone(), None).is_ok(),
                "{injury:?} sans Haine doit rester enregistrable"
            );
        }
    }

    #[test]
    fn les_six_autres_actions_restent_enregistrables() {
        let pm = make_pm(1000, 1000);
        // Le MVP n'en est plus : il entre par `record_mvp` (carte 637).
        for action in [
            MatchActionType::Touchdown,
            MatchActionType::Passe,
            MatchActionType::Interception,
            MatchActionType::Agression,
            MatchActionType::Lancer,
            MatchActionType::Sortie,
        ] {
            let r = pm.record_action(
                TeamSide::Home,
                TurnNumber::try_new(1).unwrap(),
                ActionPlayer::Regular(PlayerId::new()),
                action.clone(),
                "A".into(),
                "B".into(),
                ActionId("x".into()),
                CoachId::new(),
            );
            assert!(r.is_ok(), "{action:?} doit rester enregistrable");
        }
    }

    #[test]
    fn une_sequelle_donne_une_haine_quelle_que_soit_la_stat() {
        for stat in [
            SequelStat::MinusAv,
            SequelStat::MinusMa,
            SequelStat::MinusPa,
            SequelStat::MinusAg,
            SequelStat::MinusSt,
        ] {
            assert!(InjuryType::Sequel { stat }.peut_donner_haine());
        }
    }

    /// Les actions sont persistées en JSON. Une blessure écrite avant cette carte
    /// ne porte ni `hatred` ni `hatred_skill_uid` : sa relecture doit réussir,
    /// sinon le rejeu de tout rapport ancien échoue avec elle.
    #[test]
    fn une_action_historique_sans_haine_se_deserialise() {
        let action: MatchActionType = serde_json::from_str(r#"{"Blesse":{"injury":"Amoche"}}"#)
            .expect("le JSON d'avant la Haine doit rester relisible");
        match action {
            MatchActionType::Blesse {
                hatred,
                hatred_skill_uid,
                ..
            } => {
                assert!(hatred.is_none());
                assert!(hatred_skill_uid.is_none());
            }
            other => panic!("attendu une blessure, obtenu {other:?}"),
        }
    }

    /// Touche `UID_MOT_CLEF`. Ce n'est pas décoratif : nutype ne compile une
    /// expression passée par constante qu'au premier usage — une faute de syntaxe
    /// ne casserait pas `cargo build`, elle paniquerait en production.
    #[test]
    fn un_uid_de_mot_clef_respecte_la_forme_du_corpus() {
        assert!(HatredKeyword::try_new("DARK_ELF").is_ok());
        assert!(HatredKeyword::try_new("BEASTMAN").is_ok());
        assert!(
            HatredKeyword::try_new("dark elf").is_err(),
            "un libellé n'est pas un uid de corpus"
        );
        assert!(HatredKeyword::try_new("").is_err());
        assert!(HatredKeyword::try_new("1DARK").is_err());
    }

    // ── step5 : compute_score ────────────────────────────────────────────────

    fn add_td(pm: &MatchReportPreMatch, side: TeamSide) -> MatchReportPreMatch {
        pm.record_action(
            side,
            TurnNumber::try_new(1).unwrap(),
            ActionPlayer::Regular(PlayerId::new()),
            MatchActionType::Touchdown,
            "A".into(),
            "B".into(),
            ActionId(format!("td-{}", rand_id())),
            CoachId::new(),
        )
        .expect("action de test valide")
        .0
    }

    fn add_sortie(pm: &MatchReportPreMatch, side: TeamSide) -> MatchReportPreMatch {
        pm.record_action(
            side,
            TurnNumber::try_new(1).unwrap(),
            ActionPlayer::Regular(PlayerId::new()),
            MatchActionType::Sortie,
            "A".into(),
            "B".into(),
            ActionId(format!("so-{}", rand_id())),
            CoachId::new(),
        )
        .expect("action de test valide")
        .0
    }

    fn add_blesse(pm: &MatchReportPreMatch, side: TeamSide) -> MatchReportPreMatch {
        pm.record_action(
            side,
            TurnNumber::try_new(1).unwrap(),
            ActionPlayer::Regular(PlayerId::new()),
            MatchActionType::Blesse {
                injury: InjuryType::Commotion,
                hatred: None,
                hatred_skill_uid: None,
            },
            "A".into(),
            "B".into(),
            ActionId(format!("bl-{}", rand_id())),
            CoachId::new(),
        )
        .expect("action de test valide")
        .0
    }

    fn rand_id() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos() as u64
    }

    #[test]
    fn compute_score_counts_touchdowns_only() {
        let pm = make_pm(1000, 1000);
        let pm = add_td(&pm, TeamSide::Home);
        let pm = add_td(&pm, TeamSide::Home);
        let pm = add_td(&pm, TeamSide::Away);
        assert_eq!(pm.compute_score(), (2, 1));
    }

    #[test]
    fn compute_score_zero_when_no_touchdowns() {
        let pm = make_pm(1000, 1000);
        let pm = add_sortie(&pm, TeamSide::Home);
        assert_eq!(pm.compute_score(), (0, 0));
    }

    #[test]
    fn compute_cas_counts_sorties_only() {
        let pm = make_pm(1000, 1000);
        let pm = add_sortie(&pm, TeamSide::Home);
        let pm = add_sortie(&pm, TeamSide::Home);
        let pm = add_blesse(&pm, TeamSide::Home);
        let pm = add_sortie(&pm, TeamSide::Away);
        assert_eq!(pm.compute_cas(), (2, 1));
    }

    #[test]
    fn compute_cas_ignores_blesse() {
        let pm = make_pm(1000, 1000);
        let pm = add_blesse(&pm, TeamSide::Home);
        let pm = add_blesse(&pm, TeamSide::Away);
        assert_eq!(pm.compute_cas(), (0, 0));
    }

    // ── step5 : suggest_gains ────────────────────────────────────────────────

    fn pm_with_fans(
        home_dedicated: u32,
        away_dedicated: u32,
        home_roll: u8,
        away_roll: u8,
    ) -> MatchReportPreMatch {
        let mut pm = make_pm(1000, 1000);
        pm.home_dedicated_fans = DedicatedFans::try_new(home_dedicated).unwrap();
        pm.away_dedicated_fans = DedicatedFans::try_new(away_dedicated).unwrap();
        pm.home_fan_roll = Some(D3Roll::try_new(home_roll).unwrap());
        pm.away_fan_roll = Some(D3Roll::try_new(away_roll).unwrap());
        pm
    }

    #[test]
    fn suggest_gains_no_tds() {
        // fans_home = 10+2=12, fans_away = 10+1=11, base = (12+11)/2 = 11 → 11 * 10 = 110 kPo
        let pm = pm_with_fans(10, 10, 2, 1);
        let (home, away) = pm.suggest_gains();
        assert_eq!(home, 110);
        assert_eq!(away, 110);
    }

    #[test]
    fn suggest_gains_with_touchdowns() {
        // base = 11 * 10 = 110 kPo ; home 2 TDs, away 1 TD
        let pm = pm_with_fans(10, 10, 2, 1);
        let pm = add_td(&pm, TeamSide::Home);
        let pm = add_td(&pm, TeamSide::Home);
        let pm = add_td(&pm, TeamSide::Away);
        let (home, away) = pm.suggest_gains();
        assert_eq!(home, 110 + 2 * 10);
        assert_eq!(away, 110 + 1 * 10);
    }

    #[test]
    fn suggest_gains_zero_fans_zero_tds() {
        let mut pm = make_pm(1000, 1000);
        pm.home_fan_roll = Some(D3Roll::try_new(1).unwrap());
        pm.away_fan_roll = Some(D3Roll::try_new(1).unwrap());
        let (home, away) = pm.suggest_gains();
        // fans = 0+1=1 chaque, base = (1+1)/2 * 10 = 10 kPo
        assert_eq!(home, 10);
        assert_eq!(away, 10);
    }

    // ── step5 : record_post_match ────────────────────────────────────────────

    #[test]
    fn record_post_match_emits_event_and_returns_ready_to_publish() {
        let pm = with_both_mvps(pm_with_fans(10, 10, 2, 1));
        let home_gain = MatchGain::try_new(130).unwrap();
        let away_gain = MatchGain::try_new(110).unwrap();
        let home_mod = FanFactorMod::try_new(1).unwrap();
        let away_mod = FanFactorMod::try_new(-1).unwrap();
        let (ready, event) = pm
            .record_post_match(
                home_gain,
                away_gain,
                crate::app::match_report::domain::value_objects::NoStallingBonuses::default(),
                home_mod,
                away_mod,
                Some("Titre".into()),
                None,
                CoachId::new(),
            )
            .unwrap();
        assert_eq!(ready.home_gain, home_gain);
        assert_eq!(ready.away_gain, away_gain);
        assert_eq!(ready.home_fan_mod, home_mod);
        assert_eq!(ready.away_fan_mod, away_mod);
        assert!(matches!(
            event,
            MatchReportDomainEvent::PostMatchRecorded { .. }
        ));
    }

    // ── Bonus pour non temporisation (carte 481) ─────────────────────────────

    fn record_with(home: bool, away: bool) -> (MatchReportReadyToPublish, MatchReportDomainEvent) {
        use crate::app::match_report::domain::value_objects::{NoStallingBonus, NoStallingBonuses};
        with_both_mvps(pm_with_fans(10, 10, 2, 1))
            .record_post_match(
                MatchGain::try_new(130).unwrap(),
                MatchGain::try_new(110).unwrap(),
                NoStallingBonuses {
                    home: NoStallingBonus::new(home),
                    away: NoStallingBonus::new(away),
                },
                FanFactorMod::try_new(0).unwrap(),
                FanFactorMod::try_new(0).unwrap(),
                None,
                None,
                CoachId::new(),
            )
            .unwrap()
    }

    /// **Le test qui compte** : le gain enregistré reste la base saisie. Un
    /// montant déjà augmenté se relirait comme une base à l'étape 5, et chaque
    /// correction ajouterait 10 kPo de plus.
    #[test]
    fn the_recorded_gain_stays_the_entered_amount() {
        let (ready, _) = record_with(true, true);
        assert_eq!(ready.home_gain.into_inner(), 130);
        assert_eq!(ready.total_home_gain_kpo(), 140);
    }

    #[test]
    fn both_teams_have_independent_bonuses() {
        let (ready, _) = record_with(true, false);
        assert_eq!(ready.total_home_gain_kpo(), 140);
        assert_eq!(ready.total_away_gain_kpo(), 110);
    }

    #[test]
    fn the_post_match_event_carries_both_flags() {
        let (_, event) = record_with(false, true);
        assert!(matches!(
            event,
            MatchReportDomainEvent::PostMatchRecorded {
                home_no_stalling_bonus,
                away_no_stalling_bonus,
                ..
            } if !home_no_stalling_bonus.into_inner() && away_no_stalling_bonus.into_inner()
        ));
    }

    /// Avant la carte 481, l'événement ne portait pas les drapeaux : il se
    /// relit sans bonus — c'est la décision, aucun match déjà joué ne l'a pris.
    #[test]
    fn an_event_without_the_fields_reads_without_bonus() {
        let (_, event) = record_with(true, true);
        let mut json = serde_json::to_value(&event).unwrap();
        let champs = json.as_object_mut().unwrap();
        champs.remove("home_no_stalling_bonus");
        champs.remove("away_no_stalling_bonus");
        let relu: MatchReportDomainEvent = serde_json::from_value(json).unwrap();
        assert!(matches!(
            relu,
            MatchReportDomainEvent::PostMatchRecorded {
                home_no_stalling_bonus,
                away_no_stalling_bonus,
                ..
            } if !home_no_stalling_bonus.into_inner() && !away_no_stalling_bonus.into_inner()
        ));
    }

    // ── carte 626 : annuler les coups de pouce ───────────────────────────────

    /// Domicile favori (1100 contre 1000) : il a acheté pour 50 kPo et aligné
    /// un journalier ; l'outsider a validé aussi, avec une vedette.
    fn validated_both_sides() -> MatchReportPreMatch {
        let mut pm = make_pm(1100, 1000);
        pm.home_inducements = Some(vec![achat(50, 1)]);
        pm.away_inducements = Some(vec![]);
        pm.star_engagements = vec![(pm.away_team_id.clone(), InducementId("MORG".into()))];
        pm.home_temp_players = vec![make_journeyman(&pm)];
        pm
    }

    #[test]
    fn annuler_efface_les_achats_des_deux_camps() {
        let pm = validated_both_sides();
        let (updated, _) = pm.cancel_inducements(CoachId::new()).unwrap();
        assert!(updated.home_inducements.is_none());
        assert!(updated.away_inducements.is_none());
        assert!(updated.star_engagements.is_empty());
        assert!(updated.home_temp_players.is_empty());
    }

    /// R5 : sans achats du favori, la petite monnaie retombe sur l'écart de
    /// valeur, et le budget de l'outsider baisse d'autant.
    #[test]
    fn annuler_ramene_la_petite_monnaie_a_l_ecart_de_valeur() {
        let pm = validated_both_sides();
        assert_eq!(pm.petty_cash(), 150);
        let (updated, _) = pm.cancel_inducements(CoachId::new()).unwrap();
        assert_eq!(updated.petty_cash(), 100);
        let underdog = updated.away_team_id.clone();
        assert_eq!(updated.inducement_budget_for(&underdog, 0), 100);
    }

    /// Un reset pour le seul camp qui a des joueurs temporaires, et il nomme
    /// le journalier retiré.
    #[test]
    fn annuler_nomme_les_journaliers_retires() {
        let pm = validated_both_sides();
        let home = pm.home_team_id.clone();
        let (updated, events) = pm.cancel_inducements(CoachId::new()).unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            MatchReportDomainEvent::InducementsCancelled { .. }
        ));
        assert!(matches!(
            &events[1],
            MatchReportDomainEvent::TempPlayersReset { team_id, withdrawn }
                if team_id == &home && withdrawn == &vec![TempPlayerId("tp-01".into())]
        ));
        assert_eq!(updated.version, pm.version + 2);
    }

    #[test]
    fn annuler_est_refuse_apres_une_action() {
        let (pm, _) = make_action(&validated_both_sides(), TeamSide::Away);
        let result = pm.cancel_inducements(CoachId::new());
        assert_eq!(result.unwrap_err(), DomainError::MatchAlreadyStarted);
    }

    /// Décision du 2026-11-01 : une action supprimée ne compte pas.
    #[test]
    fn annuler_est_permis_apres_la_suppression_de_l_action() {
        let (pm, _) = make_action(&validated_both_sides(), TeamSide::Home);
        let (pm, _) = pm
            .delete_action(&ActionId("act-01".into()), CoachId::new())
            .unwrap();
        assert!(pm.cancel_inducements(CoachId::new()).is_ok());
    }

    /// R4 : personne n'a validé, rien à annuler.
    #[test]
    fn annuler_est_refuse_sans_aucune_validation() {
        let pm = make_pm(1100, 1000);
        let result = pm.cancel_inducements(CoachId::new());
        assert_eq!(result.unwrap_err(), DomainError::NoInducementsToCancel);
    }

    /// « Passer » vaut validation : un achat vide s'annule.
    #[test]
    fn annuler_un_passer_est_permis() {
        let mut pm = make_pm(1100, 1000);
        pm.home_inducements = Some(vec![]);
        assert!(pm.cancel_inducements(CoachId::new()).is_ok());
    }

    // ── carte 627 : des achats validés ne se modifient plus ──────────────────

    /// Enregistre un achat vide — « Passer » — pour l'équipe donnée.
    fn pass(
        pm: &MatchReportPreMatch,
        team_id: &TeamId,
    ) -> Result<MatchReportPreMatch, DomainError> {
        pm.record_inducements(team_id, &[], 0, &[], &[], CoachId::new())
            .map(|(updated, _)| updated)
    }

    /// Un achat de 50 kPo pour l'équipe donnée.
    fn buy(pm: &MatchReportPreMatch, team_id: &TeamId) -> Result<MatchReportPreMatch, DomainError> {
        let specs = vec![spec("BRIBE", 2, 50, false)];
        let purchases = [(InducementId("BRIBE".into()), 1)];
        pm.record_inducements(team_id, &purchases, 1_000, &specs, &[], CoachId::new())
            .map(|(updated, _)| updated)
    }

    #[test]
    fn passer_vaut_validation() {
        let pm = make_pm(1100, 1000);
        let home = pm.home_team_id.clone();
        assert!(!pm.has_recorded_inducements(&home));
        assert!(pass(&pm, &home).unwrap().has_recorded_inducements(&home));
    }

    /// R2 : un second envoi est refusé, avant toute validation d'achat — même
    /// un panier hors budget répond « déjà validé ».
    #[test]
    fn racheter_apres_un_achat_est_refuse() {
        let pm = make_pm(1100, 1000);
        let home = pm.home_team_id.clone();
        let pm = buy(&pm, &home).unwrap();
        assert_eq!(
            buy(&pm, &home).unwrap_err(),
            DomainError::InducementsAlreadyRecorded
        );
        let over_budget = pm.record_inducements(
            &home,
            &[(InducementId("BRIBE".into()), 1)],
            0,
            &[spec("BRIBE", 2, 50, false)],
            &[],
            CoachId::new(),
        );
        assert_eq!(
            over_budget.unwrap_err(),
            DomainError::InducementsAlreadyRecorded
        );
    }

    #[test]
    fn racheter_apres_passer_est_refuse() {
        let pm = make_pm(1100, 1000);
        let home = pm.home_team_id.clone();
        let pm = pass(&pm, &home).unwrap();
        assert_eq!(
            buy(&pm, &home).unwrap_err(),
            DomainError::InducementsAlreadyRecorded
        );
    }

    /// Un `ReadyToPublish` reconverti — il n'a plus de valeurs d'équipe — est
    /// refusé pour ce qu'il est : des achats validés.
    #[test]
    fn racheter_depuis_un_ready_to_publish_est_refuse() {
        let pm = with_both_mvps(validated_both_sides());
        let home = pm.home_team_id.clone();
        let (ready, _) = pm
            .record_post_match(
                MatchGain::try_new(10).unwrap(),
                MatchGain::try_new(10).unwrap(),
                NoStallingBonuses::default(),
                FanFactorMod::try_new(0).unwrap(),
                FanFactorMod::try_new(0).unwrap(),
                None,
                None,
                CoachId::new(),
            )
            .unwrap();
        let reconverted = ready.into_pre_match();
        assert_eq!(
            pass(&reconverted, &home).unwrap_err(),
            DomainError::InducementsAlreadyRecorded
        );
    }

    #[test]
    fn l_autre_equipe_achete_toujours() {
        let pm = make_pm(1100, 1000);
        let (home, away) = (pm.home_team_id.clone(), pm.away_team_id.clone());
        let pm = buy(&pm, &home).unwrap();
        let pm = buy(&pm, &away).unwrap();
        assert!(pm.has_recorded_inducements(&away));
    }

    /// R9 : l'outsider attend le favori.
    #[test]
    fn l_outsider_est_refuse_avant_le_favori() {
        let pm = make_pm(1100, 1000);
        let away = pm.away_team_id.clone();
        assert_eq!(
            pass(&pm, &away).unwrap_err(),
            DomainError::TopdogHasNotChosen
        );
    }

    /// R9 suit le favori, pas le domicile : favori à l'extérieur, c'est le
    /// domicile qui attend.
    #[test]
    fn le_domicile_outsider_attend_le_favori_exterieur() {
        let pm = make_pm(1000, 1100);
        let (home, away) = (pm.home_team_id.clone(), pm.away_team_id.clone());
        assert_eq!(
            buy(&pm, &home).unwrap_err(),
            DomainError::TopdogHasNotChosen
        );
        assert!(buy(&pass(&pm, &away).unwrap(), &home).is_ok());
    }

    #[test]
    fn l_outsider_est_admis_apres_un_achat_du_favori() {
        let pm = make_pm(1100, 1000);
        let (home, away) = (pm.home_team_id.clone(), pm.away_team_id.clone());
        assert!(pass(&buy(&pm, &home).unwrap(), &away).is_ok());
    }

    #[test]
    fn l_outsider_est_admis_apres_passer_du_favori() {
        let pm = make_pm(1100, 1000);
        let (home, away) = (pm.home_team_id.clone(), pm.away_team_id.clone());
        assert!(buy(&pass(&pm, &home).unwrap(), &away).is_ok());
    }

    /// Données de production : un outsider qui a acheté avant le favori garde
    /// ses achats, et le favori choisit encore les siens.
    #[test]
    fn le_favori_achete_encore_apres_un_outsider_premier() {
        let mut pm = make_pm(1100, 1000);
        pm.away_inducements = Some(vec![achat(30, 1)]);
        let home = pm.home_team_id.clone();
        let updated = buy(&pm, &home).unwrap();
        assert_eq!(updated.away_inducements, Some(vec![achat(30, 1)]));
    }

    /// Après l'annulation (626), le favori rachète, puis l'outsider.
    #[test]
    fn apres_une_annulation_on_rachete_favori_d_abord() {
        let (pm, _) = validated_both_sides()
            .cancel_inducements(CoachId::new())
            .unwrap();
        let (home, away) = (pm.home_team_id.clone(), pm.away_team_id.clone());
        assert_eq!(
            pass(&pm, &away).unwrap_err(),
            DomainError::TopdogHasNotChosen
        );
        let pm = buy(&pm, &home).unwrap();
        assert!(pass(&pm, &away).is_ok());
    }

    #[test]
    fn le_tour_passe_du_favori_a_l_outsider() {
        let pm = make_pm(1100, 1000);
        let (home, away) = (pm.home_team_id.clone(), pm.away_team_id.clone());
        assert_eq!(pm.inducements_turn(), InducementsTurn::Team(&home));
        let pm = pass(&pm, &home).unwrap();
        assert_eq!(pm.inducements_turn(), InducementsTurn::Team(&away));
        let pm = pass(&pm, &away).unwrap();
        assert_eq!(pm.inducements_turn(), InducementsTurn::BothRecorded);
    }

    /// R7 : l'outsider qui a validé seul, avant le favori, renvoie au favori.
    #[test]
    fn le_tour_revient_au_favori_quand_l_outsider_a_valide_seul() {
        let mut pm = make_pm(1100, 1000);
        pm.away_inducements = Some(vec![]);
        assert_eq!(
            pm.inducements_turn(),
            InducementsTurn::Team(&pm.home_team_id)
        );
    }

    // ── carte 628 : le récapitulatif ─────────────────────────────────────────

    /// R3 en question : permis en avant-match sans action vivante, refusé
    /// après une action, rendu après sa suppression.
    #[test]
    fn l_annulation_se_propose_tant_qu_aucune_action_ne_vit() {
        let pm = validated_both_sides();
        assert!(pm.can_cancel_inducements());
        let (pm, _) = make_action(&pm, TeamSide::Home);
        assert!(!pm.can_cancel_inducements());
        let (pm, _) = pm
            .delete_action(&ActionId("act-01".into()), CoachId::new())
            .unwrap();
        assert!(pm.can_cancel_inducements());
    }

    /// R4 : rien de validé, rien à proposer.
    #[test]
    fn l_annulation_ne_se_propose_pas_sans_validation() {
        assert!(!make_pm(1100, 1000).can_cancel_inducements());
    }

    /// Décision 5 : l'auteur de chaque validation est gardé, camp par camp,
    /// et l'annulation l'efface.
    #[test]
    fn l_auteur_de_chaque_validation_est_garde_puis_efface() {
        let pm = make_pm(1100, 1000);
        let (home, away) = (pm.home_team_id.clone(), pm.away_team_id.clone());
        let (favori, outsider) = (CoachId::new(), CoachId::new());
        let (pm, _) = pm
            .record_inducements(&home, &[], 0, &[], &[], favori)
            .unwrap();
        let (pm, _) = pm
            .record_inducements(&away, &[], 0, &[], &[], outsider)
            .unwrap();
        assert_eq!(pm.inducements_recorded_by(&home), Some(favori));
        assert_eq!(pm.inducements_recorded_by(&away), Some(outsider));
        let (pm, _) = pm.cancel_inducements(CoachId::new()).unwrap();
        assert_eq!(pm.inducements_recorded_by(&home), None);
        assert_eq!(pm.inducements_recorded_by(&away), None);
    }

    /// Décision 6 : la nature d'un achat, sans que le port en dise rien.
    #[test]
    fn la_nature_d_un_achat() {
        let mut pm = make_pm(1100, 1000);
        let home = pm.home_team_id.clone();
        pm.star_engagements
            .push((home.clone(), InducementId("STAR_X".into())));
        let with_uid = |uid: &str| InducementPurchase {
            uid: InducementId(uid.into()),
            ..achat(10, 1)
        };
        assert_eq!(
            pm.purchase_kind(&home, &with_uid("BRIBE")),
            PurchaseKind::Inducement
        );
        assert_eq!(
            pm.purchase_kind(&home, &with_uid("STAR_X")),
            PurchaseKind::StarPlayer
        );
        assert_eq!(
            pm.purchase_kind(&home, &with_uid("MERCO:pos-a:lvl1")),
            PurchaseKind::Mercenary {
                position_uid: "pos-a".into(),
                skilled: true
            }
        );
        assert_eq!(
            pm.purchase_kind(&home, &with_uid("MERCO:pos-a:base")),
            PurchaseKind::Mercenary {
                position_uid: "pos-a".into(),
                skilled: false
            }
        );
        let away = pm.away_team_id.clone();
        assert_eq!(
            pm.purchase_kind(&away, &with_uid("STAR_X")),
            PurchaseKind::Inducement,
            "la vedette engagée par l'autre camp n'est pas la sienne"
        );
    }
}
