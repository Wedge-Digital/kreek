//! Doublures en mémoire des quatre ports du BC `teams`.
//!
//! Mutualisées ici parce que les use cases de panier — recrutement aujourd'hui,
//! renvois demain (carte 267) — s'éprouvent tous contre le même quatuor. Elles
//! n'imitent que ce que les tests observent : la garde de version, le rejeu de
//! l'event store, et la trace des appels.

use crate::app::teams::domain::basket::SquadPresence;
use crate::app::teams::domain::team::{GamePhase, Team, TeamDomainEvent};
use crate::app::teams::ports::{
    CatalogPositionDto, CrossLimitDto, IPhaseBasketRepository, IRosterCatalogPort, ISquadPort,
    ITeamRepository, MyTeamRow, PhaseBasketState, RepositoryError, RosterCatalogDto, SkillBadgeDto,
    SquadMemberDto, StaffPriceDto, TeamCardRow, TeamEnrollmentRow, TreasuryMovementRow,
};
use async_trait::async_trait;
use std::sync::Mutex;

// ── Event store ───────────────────────────────────────────────────────────────

/// Event store en mémoire. `find_by_id` rejoue vraiment les événements : un lot
/// mal ordonné ou une phase mal gardée se voit ici comme en base.
#[derive(Default)]
pub struct FakeTeamRepository {
    pub events: Mutex<Vec<TeamDomainEvent>>,
    /// Les lots successivement appendus, dans l'ordre — ce qui permet de
    /// vérifier qu'un refus n'a **rien** écrit.
    pub batches: Mutex<Vec<Vec<TeamDomainEvent>>>,
}

impl FakeTeamRepository {
    pub fn with_events(events: Vec<TeamDomainEvent>) -> Self {
        Self {
            events: Mutex::new(events),
            batches: Mutex::new(Vec::new()),
        }
    }

    pub fn appended(&self) -> Vec<TeamDomainEvent> {
        self.batches.lock().unwrap().concat()
    }

    pub fn batch_count(&self) -> usize {
        self.batches.lock().unwrap().len()
    }
}

#[async_trait]
impl ITeamRepository for FakeTeamRepository {
    /// Doublure : le contrôle d'appartenance est exercé par les tests de
    /// handler, sur une vraie base.
    async fn find_space_id(&self, _: &str) -> Result<Option<String>, RepositoryError> {
        Ok(None)
    }

    async fn list_treasury_movements(
        &self,
        _: &str,
    ) -> Result<Vec<TreasuryMovementRow>, RepositoryError> {
        Ok(Vec::new())
    }

    async fn append(
        &self,
        team_id: &str,
        event: &TeamDomainEvent,
        expected_version: u64,
    ) -> Result<u64, RepositoryError> {
        self.append_batch(team_id, std::slice::from_ref(event), expected_version)
            .await
    }

    async fn append_batch(
        &self,
        _team_id: &str,
        events: &[TeamDomainEvent],
        expected_version: u64,
    ) -> Result<u64, RepositoryError> {
        let mut store = self.events.lock().unwrap();
        if store.len() as u64 != expected_version {
            return Err(RepositoryError::ConcurrentWrite);
        }
        store.extend(events.iter().cloned());
        self.batches.lock().unwrap().push(events.to_vec());
        Ok(store.len() as u64)
    }

    async fn find_by_id(&self, _team_id: &str) -> Result<Option<Team>, RepositoryError> {
        Ok(Team::hydrate(&self.events.lock().unwrap()))
    }

    async fn find_by_season_and_status(
        &self,
        _season_id: &str,
        _status: &str,
    ) -> Result<Vec<TeamEnrollmentRow>, RepositoryError> {
        Ok(Vec::new())
    }

    async fn find_enrolled_for_season(
        &self,
        _season_id: &str,
    ) -> Result<Vec<TeamCardRow>, RepositoryError> {
        Ok(Vec::new())
    }

    async fn find_by_coach_and_space(
        &self,
        _coach_id: &str,
        _space_id: &str,
    ) -> Result<Vec<MyTeamRow>, RepositoryError> {
        Ok(Vec::new())
    }
}

// ── Panier de phase ───────────────────────────────────────────────────────────

#[derive(Default)]
pub struct FakeBasketRepository {
    pub state: Mutex<Option<PhaseBasketState>>,
    pub deletions: Mutex<Vec<GamePhase>>,
}

impl FakeBasketRepository {
    pub fn with(state: PhaseBasketState) -> Self {
        Self {
            state: Mutex::new(Some(state)),
            deletions: Mutex::new(Vec::new()),
        }
    }

    pub fn deleted(&self) -> Vec<GamePhase> {
        self.deletions.lock().unwrap().clone()
    }
}

#[async_trait]
impl IPhaseBasketRepository for FakeBasketRepository {
    async fn load(
        &self,
        _team_id: &str,
        _phase: &GamePhase,
    ) -> Result<Option<PhaseBasketState>, RepositoryError> {
        Ok(self.state.lock().unwrap().clone())
    }

    async fn save(
        &self,
        basket: &PhaseBasketState,
        expected_version: u32,
    ) -> Result<u32, RepositoryError> {
        let mut courant = self.state.lock().unwrap();
        let version_en_base = courant.as_ref().map(|b| b.version).unwrap_or(0);
        if version_en_base != expected_version {
            return Err(RepositoryError::ConcurrentWrite);
        }
        let nouvelle = expected_version + 1;
        *courant = Some(PhaseBasketState {
            version: nouvelle,
            ..basket.clone()
        });
        Ok(nouvelle)
    }

    async fn delete(&self, _team_id: &str, phase: &GamePhase) -> Result<(), RepositoryError> {
        *self.state.lock().unwrap() = None;
        self.deletions.lock().unwrap().push(phase.clone());
        Ok(())
    }
}

// ── Catalogue de roster ───────────────────────────────────────────────────────

pub const PIETAILLE: &str = "DEMO_GRANIT__PIETAILLE";
pub const PERCUTEUR: &str = "DEMO_GRANIT__PERCUTEUR";

fn poste_dto(uid: &str, nom: &str, cout: u32, max: u8, journalier: bool) -> CatalogPositionDto {
    CatalogPositionDto {
        uid: uid.into(),
        position_name: nom.into(),
        cost: cout,
        max_quantity: max,
        is_journeyman: journalier,
        ma: 6,
        st: 3,
        ag: 3,
        pa: 4,
        av: 9,
        skills: vec![SkillBadgeDto {
            name: "Blocage".into(),
            category: "GENERAL".into(),
        }],
    }
}

pub struct FakeRosterCatalogPort;

impl IRosterCatalogPort for FakeRosterCatalogPort {
    fn find_catalog(&self, _roster_id: &str) -> Option<RosterCatalogDto> {
        Some(RosterCatalogDto {
            logo: None,
            linemen_are_free: false,
            reroll_base_cost: 60,
            positions: vec![
                poste_dto(PIETAILLE, "Piétaille des Carrières", 50, 16, true),
                poste_dto(PERCUTEUR, "Percuteur", 90, 2, false),
            ],
            cross_limits: vec![CrossLimitDto {
                max: 2,
                position_uids: vec![PERCUTEUR.into()],
            }],
            allowed_staff: vec!["APOTHECARY".into(), "CHEERLEADERS".into()],
            staff_prices: vec![
                StaffPriceDto {
                    uid: "APOTHECARY".into(),
                    name: "Apothicaire".into(),
                    price: 50,
                    max_quantity: 1,
                },
                StaffPriceDto {
                    uid: "CHEERLEADERS".into(),
                    name: "Meneuses".into(),
                    price: 10,
                    max_quantity: 6,
                },
            ],
        })
    }
}

// ── Effectif ──────────────────────────────────────────────────────────────────

/// Effectif figé, énoncé en lignes de roster : c'est tout ce que le panier de
/// recrutement en regarde.
pub struct FakeSquadPort(pub Vec<&'static str>);

impl FakeSquadPort {
    pub fn empty() -> Self {
        Self(Vec::new())
    }
}

#[async_trait]
impl ISquadPort for FakeSquadPort {
    async fn find_squad(&self, _team_id: &str) -> Vec<SquadMemberDto> {
        self.0
            .iter()
            .enumerate()
            .map(|(i, ligne)| SquadMemberDto {
                // Un ULID valide, pas un `player-{i}` : l'hydratation refuse un
                // identifiant illisible plutôt que de sauter le membre, et une
                // doublure qui n'en produit pas ferait échouer tout use case
                // consommant l'effectif.
                player_id: format!("{i:0>26}"),
                jersey: Some(i as u8 + 1),
                roster_line_id: (*ligne).to_string(),
                personal_name: format!("Joueur {i}"),
                position_name: "Poste".into(),
                spp: 0,
                value_kpo: 50,
                presence: SquadPresence::Alignable,
            })
            .collect()
    }
}
