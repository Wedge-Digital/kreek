//! Déplacer une rencontre sur une autre journée (carte 557).
//!
//! # Pourquoi c'est `competitions` qui décide
//!
//! Les journées et les appariements lui appartiennent, et la règle « une
//! équipe joue une fois par journée » vit dans `MatchDay` depuis la carte 551.
//! Le rapport de match, le classement et l'historique des joueurs **suivent**,
//! par l'événement `PairingMoved` — ce use case ne les connaît pas.
//!
//! # Ce qui est vérifié, dans l'ordre
//!
//! 1. la rencontre existe dans cette saison — c'est aussi ce qui donne sa
//!    journée de départ, que l'appelant n'a pas à connaître ;
//! 2. la journée d'arrivée existe, appartient à la même saison, et n'est pas
//!    celle de départ ;
//! 3. aucune des deux équipes n'y joue déjà — refus nommé, comme pour un ajout.
//!
//! Le domaine porte la troisième : `MatchDay::accueillir` rend l'appariement
//! bloquant, et ce use case ne fait que le mettre en mots.

use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::competitions::domain::match_day::{MatchDay, Pairing};
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::competitions::ports::ITeamInfoPort;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{EventId, SpaceId};
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct MovePairingCommand {
    pub pairing_id: PairingId,
    pub to_round_id: MatchId,
    pub season_id: SeasonId,
    pub space_id: SpaceId,
}

#[derive(Debug)]
pub enum MovePairingError {
    PairingNotFound,
    RoundNotFound,
    /// La journée d'arrivée est celle de départ : rien à faire, et le dire
    /// vaut mieux qu'un succès qui n'a rien déplacé.
    SameRound,
    /// La journée d'arrivée existe, mais dans une autre saison.
    RoundOutsideSeason,
    /// Carte 551 — l'une des deux équipes joue déjà la journée d'arrivée.
    TeamAlreadyScheduled {
        equipe: String,
        adversaire: String,
    },
    Repository(String),
}

/// Ce que l'appelant affiche : le nom de la journée d'arrivée.
#[derive(Debug)]
pub struct MovedPairing {
    pub to_round_name: String,
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: MovePairingCommand,
    match_day_repo: &dyn IMatchDayRepository,
    team_port: &dyn ITeamInfoPort,
    event_bus: &EventBus,
) -> Result<MovedPairing, MovePairingError> {
    let (from, pairing) = journee_de_depart(&cmd, match_day_repo).await?;
    let mut to = journee_d_arrivee(&cmd, &from, match_day_repo).await?;

    if let Err(bloquant) = to.accueillir(pairing.clone()) {
        return Err(refus_nomme(&bloquant, &pairing, team_port).await);
    }

    match_day_repo
        .move_pairing(&cmd.pairing_id.to_string(), &to)
        .await
        .map_err(|e| MovePairingError::Repository(e.to_string()))?;

    emettre(
        event_bus,
        evenement(&cmd, &from, &to, &pairing).to_enveloppe(),
    );
    Ok(MovedPairing {
        to_round_name: to.name.as_ref().to_string(),
    })
}

/// La journée qui porte la rencontre aujourd'hui, la rencontre retirée.
async fn journee_de_depart(
    cmd: &MovePairingCommand,
    match_day_repo: &dyn IMatchDayRepository,
) -> Result<(MatchDay, Pairing), MovePairingError> {
    let journees = match_day_repo
        .find_by_season(&cmd.season_id.to_string())
        .await
        .map_err(|e| MovePairingError::Repository(e.to_string()))?;

    let mut from = journees
        .into_iter()
        .find(|j| j.pairings.iter().any(|p| p.id == cmd.pairing_id))
        .ok_or(MovePairingError::PairingNotFound)?;
    let pairing = from
        .liberer(&cmd.pairing_id)
        .ok_or(MovePairingError::PairingNotFound)?;
    Ok((from, pairing))
}

async fn journee_d_arrivee(
    cmd: &MovePairingCommand,
    from: &MatchDay,
    match_day_repo: &dyn IMatchDayRepository,
) -> Result<MatchDay, MovePairingError> {
    let to = match_day_repo
        .find_by_id(&cmd.to_round_id.to_string())
        .await
        .map_err(|e| MovePairingError::Repository(e.to_string()))?
        .ok_or(MovePairingError::RoundNotFound)?;

    if to.id == from.id {
        return Err(MovePairingError::SameRound);
    }
    if to.season_id != cmd.season_id {
        return Err(MovePairingError::RoundOutsideSeason);
    }
    Ok(to)
}

/// Le refus nomme l'équipe déjà prise et son adversaire — sans quoi
/// l'administrateur ne saurait pas quel match libérer.
async fn refus_nomme(
    bloquant: &Pairing,
    deplacee: &Pairing,
    team_port: &dyn ITeamInfoPort,
) -> MovePairingError {
    let engagee = if bloquant.engage(&deplacee.home_team_id) {
        &deplacee.home_team_id
    } else {
        &deplacee.away_team_id
    };
    let adversaire = bloquant
        .adversaire_de(engagee)
        .expect("l'appariement bloquant engage cette équipe");
    let noms = noms_des_equipes(&[engagee.clone(), adversaire.clone()], team_port).await;
    MovePairingError::TeamAlreadyScheduled {
        equipe: noms.0,
        adversaire: noms.1,
    }
}

/// Un échec de résolution des noms ne fait pas échouer le refus : on retombe
/// sur les identifiants.
async fn noms_des_equipes(ids: &[TeamId; 2], team_port: &dyn ITeamInfoPort) -> (String, String) {
    let voulus: Vec<String> = ids.iter().map(|t| t.to_string()).collect();
    let trouves = team_port.find_team_names(&voulus).await.unwrap_or_default();
    let nom = |id: &str| {
        trouves
            .iter()
            .find(|t| t.team_id == id)
            .map(|t| t.team_name.clone())
            .unwrap_or_else(|| id.to_string())
    };
    (nom(&voulus[0]), nom(&voulus[1]))
}

fn evenement(
    cmd: &MovePairingCommand,
    from: &MatchDay,
    to: &MatchDay,
    pairing: &Pairing,
) -> CompetitionsDomainEvent {
    CompetitionsDomainEvent::PairingMoved {
        event_id: EventId::new(),
        pairing_id: pairing.id.to_string(),
        season_id: cmd.season_id.to_string(),
        space_id: cmd.space_id.to_string(),
        home_team_id: pairing.home_team_id.to_string(),
        away_team_id: pairing.away_team_id.to_string(),
        from_round_id: from.id.to_string(),
        to_round_id: to.id.to_string(),
        to_round_name: to.name.as_ref().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{
        MatchDayName, MatchDayPosition, MatchDayType,
    };
    use crate::app::competitions::domain::match_day_repository_port::{
        MatchDayRepositoryError, NewPairingProjection, PairingDisplayDto,
    };
    use crate::app::competitions::ports::TeamInfoDto;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    struct FakeTeamPort;

    #[async_trait]
    impl ITeamInfoPort for FakeTeamPort {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
        }
        async fn find_team_names(&self, team_ids: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(team_ids
                .iter()
                .map(|id| TeamInfoDto {
                    team_id: id.clone(),
                    team_name: format!("Équipe {id}"),
                    coach_id: String::new(),
                    coach_name: String::new(),
                    roster_name: String::new(),
                    logo_url: None,
                })
                .collect())
        }
        async fn find_team_enrollment(
            &self,
            _: &str,
        ) -> Result<Option<crate::app::competitions::ports::TeamEnrollmentDto>, String> {
            Ok(None)
        }
    }

    /// Enregistre les déplacements demandés : `(pairing_id, journée d'arrivée)`.
    #[derive(Default)]
    struct FakeMatchDayRepo {
        moved: Arc<Mutex<Vec<(String, String)>>>,
        days: Vec<MatchDay>,
    }

    #[async_trait]
    impl IMatchDayRepository for FakeMatchDayRepo {
        async fn delete_pairing(&self, _: &str) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn move_pairing(
            &self,
            pairing_id: &str,
            to: &MatchDay,
        ) -> Result<(), MatchDayRepositoryError> {
            self.moved
                .lock()
                .unwrap()
                .push((pairing_id.to_string(), to.id.to_string()));
            Ok(())
        }
        async fn find_by_season(&self, _: &str) -> Result<Vec<MatchDay>, MatchDayRepositoryError> {
            Ok(self.days.clone())
        }
        async fn find_by_id(&self, id: &str) -> Result<Option<MatchDay>, MatchDayRepositoryError> {
            Ok(self.days.iter().find(|d| d.id.to_string() == id).cloned())
        }
        async fn save_match_day(&self, _: &MatchDay) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn delete_match_day(&self, _: &str) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn save_pairing(
            &self,
            _: &str,
            _: &Pairing,
            _: &NewPairingProjection,
        ) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn find_pairing_id(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<Option<String>, MatchDayRepositoryError> {
            Ok(None)
        }
        async fn ensure_match_days_from_structure(
            &self,
            _: &str,
            _: &[(String, String, String, Option<String>, Option<String>)],
        ) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn list_resultats(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn list_calendrier(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn list_team_matches(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn list_latest_completed_results(
            &self,
            _: &str,
            _: i64,
        ) -> Result<
            Vec<crate::app::competitions::domain::match_day_repository_port::LatestResultDto>,
            MatchDayRepositoryError,
        > {
            Ok(vec![])
        }
    }

    fn bus() -> EventBus {
        crate::common::services::event_bus::event_bus::new_bus()
    }

    fn journee(season: &SeasonId, position: i32, pairings: Vec<Pairing>) -> MatchDay {
        MatchDay {
            id: MatchId::new(),
            season_id: season.clone(),
            name: MatchDayName::try_new(format!("J{}", position + 1)).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(position).unwrap(),
            pairings,
        }
    }

    fn rencontre(a: &TeamId, b: &TeamId) -> Pairing {
        Pairing {
            id: PairingId::new(),
            home_team_id: a.clone(),
            away_team_id: b.clone(),
        }
    }

    fn commande(pairing: &Pairing, to: &MatchDay, season: &SeasonId) -> MovePairingCommand {
        MovePairingCommand {
            pairing_id: pairing.id.clone(),
            to_round_id: to.id.clone(),
            season_id: season.clone(),
            space_id: SpaceId::new(),
        }
    }

    #[tokio::test]
    async fn un_match_part_vers_une_journee_libre() {
        let season = SeasonId::new();
        let (a, b) = (TeamId::new(), TeamId::new());
        let deplacee = rencontre(&a, &b);
        let depart = journee(&season, 0, vec![deplacee.clone()]);
        let arrivee = journee(&season, 1, vec![]);
        let repo = FakeMatchDayRepo {
            days: vec![depart, arrivee.clone()],
            ..Default::default()
        };
        let bus = bus();
        let mut rx = bus.subscribe();

        let fait = execute(
            commande(&deplacee, &arrivee, &season),
            &repo,
            &FakeTeamPort,
            &bus,
        )
        .await
        .expect("journée libre");

        assert_eq!(fait.to_round_name, "J2");
        assert_eq!(
            *repo.moved.lock().unwrap(),
            vec![(deplacee.id.to_string(), arrivee.id.to_string())]
        );
        let emis = rx.try_recv().expect("un événement émis");
        assert_eq!(emis.event_type, "PairingMoved");
    }

    /// Le refus nomme l'équipe déjà prise et son adversaire, et **rien n'est
    /// écrit** : c'est la règle de la carte 551, qu'un déplacement ne contourne
    /// pas.
    #[tokio::test]
    async fn une_equipe_deja_prise_a_l_arrivee_refuse_en_nommant_le_match() {
        let season = SeasonId::new();
        let (a, b, c) = (TeamId::new(), TeamId::new(), TeamId::new());
        let deplacee = rencontre(&a, &b);
        let depart = journee(&season, 0, vec![deplacee.clone()]);
        let arrivee = journee(&season, 1, vec![rencontre(&c, &b)]);
        let repo = FakeMatchDayRepo {
            days: vec![depart, arrivee.clone()],
            ..Default::default()
        };

        let refus = execute(
            commande(&deplacee, &arrivee, &season),
            &repo,
            &FakeTeamPort,
            &bus(),
        )
        .await
        .expect_err("b joue déjà");

        match refus {
            MovePairingError::TeamAlreadyScheduled { equipe, adversaire } => {
                assert_eq!(equipe, format!("Équipe {b}"));
                assert_eq!(adversaire, format!("Équipe {c}"));
            }
            autre => panic!("refus inattendu : {autre:?}"),
        }
        assert!(repo.moved.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn la_journee_de_depart_est_refusee_comme_arrivee() {
        let season = SeasonId::new();
        let deplacee = rencontre(&TeamId::new(), &TeamId::new());
        let depart = journee(&season, 0, vec![deplacee.clone()]);
        let repo = FakeMatchDayRepo {
            days: vec![depart.clone()],
            ..Default::default()
        };

        let refus = execute(
            commande(&deplacee, &depart, &season),
            &repo,
            &FakeTeamPort,
            &bus(),
        )
        .await
        .expect_err("même journée");
        assert!(matches!(refus, MovePairingError::SameRound));
    }

    #[tokio::test]
    async fn une_journee_d_une_autre_saison_est_refusee() {
        let season = SeasonId::new();
        let deplacee = rencontre(&TeamId::new(), &TeamId::new());
        let depart = journee(&season, 0, vec![deplacee.clone()]);
        let ailleurs = journee(&SeasonId::new(), 0, vec![]);
        let repo = FakeMatchDayRepo {
            days: vec![depart, ailleurs.clone()],
            ..Default::default()
        };

        let refus = execute(
            commande(&deplacee, &ailleurs, &season),
            &repo,
            &FakeTeamPort,
            &bus(),
        )
        .await
        .expect_err("autre saison");
        assert!(matches!(refus, MovePairingError::RoundOutsideSeason));
    }

    #[tokio::test]
    async fn un_appariement_inconnu_est_refuse() {
        let season = SeasonId::new();
        let arrivee = journee(&season, 1, vec![]);
        let repo = FakeMatchDayRepo {
            days: vec![journee(&season, 0, vec![]), arrivee.clone()],
            ..Default::default()
        };
        let inconnue = rencontre(&TeamId::new(), &TeamId::new());

        let refus = execute(
            commande(&inconnue, &arrivee, &season),
            &repo,
            &FakeTeamPort,
            &bus(),
        )
        .await
        .expect_err("inconnu");
        assert!(matches!(refus, MovePairingError::PairingNotFound));
    }
}
