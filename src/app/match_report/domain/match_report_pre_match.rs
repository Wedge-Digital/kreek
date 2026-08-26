use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_draft::MatchReportDraft;
use crate::app::match_report::domain::match_report_ready_to_publish::MatchReportReadyToPublish;
use crate::app::match_report::domain::value_objects::TurnNumber;
use crate::app::match_report::domain::value_objects::{
    ActionId, ActionPlayer, AllowedInducementSpec, D3Roll, DedicatedFans, FanFactorMod,
    InducementMaxQty, InducementPurchase, InducementQty, MatchAction, MatchActionType, MatchGain,
    MatchReportOrigin, TeamSide, TeamValue, TempPlayer,
};
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::bloodbowl::inducement_definition::InducementId;

/// Ce que l'underdog peut ajouter à sa petite monnaie, pris à sa trésorerie.
/// Plafond de règlement, pas de configuration.
const MAX_TREASURY_TOP_UP: u32 = 50;
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
    pub star_engagements: Vec<(TeamId, InducementId)>,
    pub home_temp_players: Vec<TempPlayer>,
    pub away_temp_players: Vec<TempPlayer>,
    pub home_actions: Vec<MatchAction>,
    pub away_actions: Vec<MatchAction>,
    pub version: u64,
}

impl MatchReportPreMatch {
    /// Annule un rapport dont la sélection est déjà confirmée — les deux
    /// équipes sont verrouillées en saisie, le listener `teams` les libère à
    /// partir des ids portés par l'événement.
    pub fn cancel(self, reason: String) -> MatchReportDomainEvent {
        MatchReportDomainEvent::MatchReportCancelled {
            reason,
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
        let home_tv = self.home_team_value.as_ref().expect("TV not recorded");
        let away_tv = self.away_team_value.as_ref().expect("TV not recorded");
        if away_tv > home_tv {
            &self.away_team_id
        } else {
            &self.home_team_id
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
        let tv_diff = self
            .home_team_value
            .unwrap()
            .into_inner()
            .abs_diff(self.away_team_value.unwrap().into_inner());
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
        validate_purchases(purchases, allowed_specs, opponent_star_uids, budget)?;
        let purchase_list = build_purchase_list(purchases, allowed_specs)?;
        let events = build_inducement_events(team_id, &purchase_list, allowed_specs, recorded_by);
        let mut updated = self.clone();
        set_inducements_for(&mut updated, team_id, purchase_list);
        updated.version += events.len() as u64;
        Ok((updated, events))
    }

    fn inducements_for(&self, team_id: &TeamId) -> Option<&Vec<InducementPurchase>> {
        if team_id == &self.home_team_id {
            self.home_inducements.as_ref()
        } else {
            self.away_inducements.as_ref()
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

    pub fn reset_temp_players(&self, team_id: &TeamId) -> (Self, MatchReportDomainEvent) {
        let event = MatchReportDomainEvent::TempPlayersReset {
            team_id: team_id.clone(),
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
        if let MatchActionType::Blesse {
            injury,
            hatred: Some(_),
            ..
        } = &action
        {
            if !injury.peut_donner_haine() {
                return Err(DomainError::HatredNotAllowedForInjury);
            }
        }

        let event = MatchReportDomainEvent::ActionRecorded {
            action_id: action_id.clone(),
            team_side,
            turn,
            player: player.clone(),
            action: action.clone(),
            player_display_name: player_display_name.clone(),
            player_position: player_position.clone(),
            recorded_by,
        };
        let entry = MatchAction {
            id: action_id,
            turn,
            player,
            action,
            player_display_name,
            player_position,
        };
        let mut updated = self.clone();
        match team_side {
            TeamSide::Home => updated.home_actions.push(entry),
            TeamSide::Away => updated.away_actions.push(entry),
        }
        updated.version += 1;
        Ok((updated, event))
    }

    pub fn delete_action(
        &self,
        action_id: &ActionId,
        deleted_by: CoachId,
    ) -> Result<(Self, MatchReportDomainEvent), DomainError> {
        let team_side = if self.home_actions.iter().any(|a| &a.id == action_id) {
            TeamSide::Home
        } else if self.away_actions.iter().any(|a| &a.id == action_id) {
            TeamSide::Away
        } else {
            return Err(DomainError::ActionNotFound(action_id.0.clone()));
        };
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
        home_fan_mod: FanFactorMod,
        away_fan_mod: FanFactorMod,
        summary_title: Option<String>,
        summary_body: Option<String>,
        recorded_by: CoachId,
    ) -> (MatchReportReadyToPublish, MatchReportDomainEvent) {
        let event = MatchReportDomainEvent::PostMatchRecorded {
            home_gain,
            away_gain,
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
            home_fan_mod,
            away_fan_mod,
            summary_title,
            summary_body,
        );
        (ready, event)
    }

    pub fn from_draft(draft: MatchReportDraft) -> Self {
        Self {
            id: draft.id,
            space_id: draft.space_id,
            competition_id: draft.competition_id,
            season_id: draft.season_id,
            round_id: draft.round_id,
            home_team_id: draft.home_team_id,
            away_team_id: draft.away_team_id,
            created_by: draft.created_by,
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
            star_engagements: vec![],
            home_temp_players: vec![],
            away_temp_players: vec![],
            home_actions: vec![],
            away_actions: vec![],
            version: draft.version + 1,
        }
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
    if total > 3 {
        Err(DomainError::TooManyMercenaries {
            requested: total,
            max: 3,
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
    if star_count > 2 {
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
) {
    if team_id == &pm.home_team_id {
        pm.home_inducements = Some(purchases);
    } else {
        pm.away_inducements = Some(purchases);
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
            star_engagements: vec![],
            home_temp_players: vec![],
            away_temp_players: vec![],
            home_actions: vec![],
            away_actions: vec![],
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
        let pm = make_pm(1000, 1000);
        let (pm2, _) = pm
            .record_action(
                TeamSide::Home,
                TurnNumber::try_new(1).unwrap(),
                ActionPlayer::Regular(PlayerId::new()),
                MatchActionType::Mvp,
                "A".to_string(),
                String::new(),
                ActionId("a1".to_string()),
                CoachId::new(),
            )
            .expect("action de test valide");
        let (pm3, _) = pm2
            .record_action(
                TeamSide::Home,
                TurnNumber::try_new(1).unwrap(),
                ActionPlayer::Regular(PlayerId::new()),
                MatchActionType::Mvp,
                "B".to_string(),
                String::new(),
                ActionId("a2".to_string()),
                CoachId::new(),
            )
            .expect("action de test valide");
        assert_eq!(pm3.home_actions.len(), 2);
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
    fn les_sept_autres_actions_restent_enregistrables() {
        let pm = make_pm(1000, 1000);
        for action in [
            MatchActionType::Touchdown,
            MatchActionType::Passe,
            MatchActionType::Interception,
            MatchActionType::Agression,
            MatchActionType::Lancer,
            MatchActionType::Sortie,
            MatchActionType::Mvp,
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
        let pm = pm_with_fans(10, 10, 2, 1);
        let home_gain = MatchGain::try_new(130).unwrap();
        let away_gain = MatchGain::try_new(110).unwrap();
        let home_mod = FanFactorMod::try_new(1).unwrap();
        let away_mod = FanFactorMod::try_new(-1).unwrap();
        let (ready, event) = pm.record_post_match(
            home_gain,
            away_gain,
            home_mod,
            away_mod,
            Some("Titre".into()),
            None,
            CoachId::new(),
        );
        assert_eq!(ready.home_gain, home_gain);
        assert_eq!(ready.away_gain, away_gain);
        assert_eq!(ready.home_fan_mod, home_mod);
        assert_eq!(ready.away_fan_mod, away_mod);
        assert!(matches!(
            event,
            MatchReportDomainEvent::PostMatchRecorded { .. }
        ));
    }
}
