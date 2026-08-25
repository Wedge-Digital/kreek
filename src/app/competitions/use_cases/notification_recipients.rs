//! Qui reçoit quoi, borné par l'espace.
//!
//! # R7 est tenue par le chemin de données, pas par une vérification
//!
//! Les cinq cas commencent par la **même opération** : lister les membres de
//! l'espace, et n'en sortir personne. Ce n'est pas une précaution ajoutée —
//! c'est le seul chemin vers une adresse e-mail. Ni `invited_coaches` ni
//! `find_enrolled_teams` n'en portent ; seul `list_space_members` en a. Un
//! invité qui a quitté l'espace, ou une équipe dont le coach n'y est plus,
//! tombe donc naturellement, sans qu'aucune ligne ne vérifie quoi que ce soit.
//!
//! C'est la troisième règle de cette épic tenue par une structure plutôt que par
//! de la vigilance : R9 par la signature de `due_today()`, R2 par la
//! `target_date` de la clé, R7 ici. Ces trois-là ne peuvent pas être oubliées
//! lors d'une modification future.
//!
//! # Pourquoi ce service existe
//!
//! La date limite est le seul cas où l'ensemble n'existe dans aucun port : ni
//! « les invités qui ne se sont pas inscrits », ni « les membres qui n'ont pas
//! bougé » ne se demandent. Sans ce service, cette soustraction finirait dans la
//! CLI, où elle n'a rien à faire.
//!
//! # Il vit dans `use_cases/` et non dans `domain/`
//!
//! Il consomme des ports, donc il n'est pas du domaine pur — cf. CLAUDE.md,
//! « Domain services pour données inter-BCs ».

use crate::app::competitions::domain::competition_invitations::{
    AccessMode, CompetitionInvitations,
};
use crate::app::competitions::domain::notification_delivery::NotificationType;
use crate::app::competitions::domain::notification_schedule::RoundRef;
use crate::app::competitions::ports::{
    ICompetitionSpaceMemberPort, ITeamInfoPort, SpaceMemberDto, TeamInfoDto,
};
use crate::app::shared_kernel::identity::ids::SpaceId;
use std::collections::HashSet;

/// Ce que ce coach joue cette journée.
///
/// **Un enum et non un `Option<Fixture>`** : rien n'empêche un coach d'inscrire
/// deux équipes dans la même saison, et la clé d'idempotence ne portant pas
/// d'équipe, il reçoit **un seul** e-mail. Un `Option` aurait donc perdu le
/// second match en silence.
///
/// L'enum garde par ailleurs ce que l'`Option` apportait : le gabarit doit
/// traiter les deux branches, donc R4 reste tenue par le type et non par une
/// consigne. `NotPlaying` est une **information**, pas une absence de donnée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoundParticipation {
    NotPlaying,
    Playing(Vec<Fixture>),
}

/// Pas de `match_url`, contrairement à la phase 4 de la spec : le construire
/// obligerait un service de `use_cases/` à connaître `AppRoutes`, qui est de la
/// couche web. Le gabarit reçoit déjà `app_url` et sait le composer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fixture {
    /// L'équipe **du coach** dans cet appariement — c'est elle qui distingue
    /// deux fixtures d'un coach qui aligne deux équipes le même jour.
    pub team_name: String,
    pub home_team: String,
    pub away_team: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipient {
    pub coach_id: String,
    pub coach_name: String,
    pub email: String,
    pub participation: RoundParticipation,
}

pub struct SeasonContext<'a> {
    pub space_id: &'a SpaceId,
    pub season_id: &'a str,
    pub invitations: Option<&'a CompetitionInvitations>,
}

// Ce qui mérite une ligne de journal, c'est l'envoi lui-même : le use case
// d'expédition (carte 339) l'écrit, avec le nombre de destinataires que ce
// service lui a rendus. Un span de plus ici n'émettrait rien — sans `FmtSpan`,
// un span ne produit aucune ligne — et le `rid` relie déjà les deux.
//
// arch:no-instrument — service de résolution : assemble une liste depuis deux ports, sans mutation, sans évènement, sans persistance
pub async fn resolve(
    notification: NotificationType,
    season: &SeasonContext<'_>,
    round: Option<&RoundRef>,
    teams: &dyn ITeamInfoPort,
    members: &dyn ICompetitionSpaceMemberPort,
) -> Vec<Recipient> {
    let membres = members.list_space_members(season.space_id).await;
    let inscrites = teams
        .find_enrolled_teams(season.season_id)
        .await
        .unwrap_or_default();

    let retenus = match notification {
        NotificationType::RegistrationOpen => ouverture(&membres, season.invitations),
        NotificationType::RoundEve | NotificationType::RoundClosing => {
            inscrits(&membres, &inscrites)
        }
        NotificationType::RegistrationDeadline => non_inscrits(&membres, season, &inscrites),
    };

    retenus
        .into_iter()
        .map(|m| destinataire(m, &inscrites, round))
        .collect()
}

/// Mode `invitation` : les invités **∩** les membres. Mode `open` : tous les
/// membres — « tous ceux qui peuvent s'inscrire » désignerait sinon la
/// plateforme entière.
fn ouverture(
    membres: &[SpaceMemberDto],
    invitations: Option<&CompetitionInvitations>,
) -> Vec<SpaceMemberDto> {
    match invitations.map(|i| &i.access_mode) {
        Some(AccessMode::Open) | None => membres.to_vec(),
        Some(AccessMode::Invitation) => {
            let invites: HashSet<String> = invitations
                .map(|i| i.invited_coaches.iter().map(|c| c.id.to_string()).collect())
                .unwrap_or_default();
            membres
                .iter()
                .filter(|m| invites.contains(&m.coach_id))
                .cloned()
                .collect()
        }
    }
}

fn inscrits(membres: &[SpaceMemberDto], inscrites: &[TeamInfoDto]) -> Vec<SpaceMemberDto> {
    let coachs: HashSet<&str> = inscrites.iter().map(|t| t.coach_id.as_str()).collect();
    membres
        .iter()
        .filter(|m| coachs.contains(m.coach_id.as_str()))
        .cloned()
        .collect()
}

/// Le seul ensemble qu'aucun port ne rend : les candidats **moins** les
/// inscrits. C'est ce qui justifie l'existence de ce service.
fn non_inscrits(
    membres: &[SpaceMemberDto],
    season: &SeasonContext<'_>,
    inscrites: &[TeamInfoDto],
) -> Vec<SpaceMemberDto> {
    let deja: HashSet<&str> = inscrites.iter().map(|t| t.coach_id.as_str()).collect();
    ouverture(membres, season.invitations)
        .into_iter()
        .filter(|m| !deja.contains(m.coach_id.as_str()))
        .collect()
}

/// Un coach à deux équipes rend **un** destinataire portant **deux** fixtures.
fn destinataire(
    m: SpaceMemberDto,
    inscrites: &[TeamInfoDto],
    round: Option<&RoundRef>,
) -> Recipient {
    let participation = match round {
        None => RoundParticipation::NotPlaying,
        Some(r) => fixtures(&m.coach_id, inscrites, r),
    };
    Recipient {
        coach_id: m.coach_id,
        coach_name: m.coach_name,
        email: m.email,
        participation,
    }
}

/// Les appariements de la journée, croisés avec les équipes du coach.
///
/// Un coach dont aucune équipe n'apparaît est `NotPlaying` — c'est une
/// **information**, pas une absence de donnée (R4), et le gabarit doit la
/// traiter.
fn fixtures(coach_id: &str, inscrites: &[TeamInfoDto], round: &RoundRef) -> RoundParticipation {
    let siennes: Vec<&TeamInfoDto> = inscrites
        .iter()
        .filter(|t| t.coach_id == coach_id)
        .collect();

    let nom = |id: &str| {
        inscrites
            .iter()
            .find(|t| t.team_id == id)
            .map(|t| t.team_name.clone())
            .unwrap_or_default()
    };

    let mut jouees = Vec::new();
    for p in &round.pairings {
        for t in &siennes {
            if t.team_id == p.home_team_id.to_string() || t.team_id == p.away_team_id.to_string() {
                jouees.push(Fixture {
                    team_name: t.team_name.clone(),
                    home_team: nom(&p.home_team_id.to_string()),
                    away_team: nom(&p.away_team_id.to_string()),
                });
            }
        }
    }

    if jouees.is_empty() {
        RoundParticipation::NotPlaying
    } else {
        RoundParticipation::Playing(jouees)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::MatchDayName;
    use crate::app::competitions::domain::match_day::{MatchDayType, Pairing};
    use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId};
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use async_trait::async_trait;

    // ── Doublures ────────────────────────────────────────────────────────────

    struct Membres(Vec<SpaceMemberDto>);

    #[async_trait]
    impl ICompetitionSpaceMemberPort for Membres {
        async fn list_space_members(&self, _: &SpaceId) -> Vec<SpaceMemberDto> {
            self.0.clone()
        }
        async fn find_member_profile(
            &self,
            _: &crate::app::shared_kernel::identity::ids::CoachId,
            _: &SpaceId,
        ) -> Option<crate::app::shared_kernel::identity::authorization::SpaceProfile> {
            None
        }
        async fn find_all_spaces(
            &self,
        ) -> Vec<crate::app::shared_kernel::identity::space_definition::SpaceDefinition> {
            Vec::new()
        }
    }

    struct Equipes(Vec<TeamInfoDto>);

    #[async_trait]
    impl ITeamInfoPort for Equipes {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(self.0.clone())
        }
        async fn find_team_names(&self, _: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(Vec::new())
        }
    }

    fn membre(id: &str, nom: &str) -> SpaceMemberDto {
        SpaceMemberDto {
            coach_id: id.to_string(),
            coach_name: nom.to_string(),
            email: format!("{nom}@example.test"),
        }
    }

    fn equipe(team_id: &str, nom: &str, coach_id: &str) -> TeamInfoDto {
        TeamInfoDto {
            team_id: team_id.to_string(),
            team_name: nom.to_string(),
            coach_id: coach_id.to_string(),
            coach_name: String::new(),
            roster_name: String::new(),
            logo_url: None,
        }
    }

    fn espace() -> SpaceId {
        SpaceId::try_new("01KZVCJZ35JZWQB0KNTA9JJEPX").unwrap()
    }

    fn contexte<'a>(
        space: &'a SpaceId,
        inv: Option<&'a CompetitionInvitations>,
    ) -> SeasonContext<'a> {
        SeasonContext {
            space_id: space,
            season_id: "01KZVCKDG19DXZHJA295WSJGMV",
            invitations: inv,
        }
    }

    fn invitations(mode: &str, invites: &[&str], deadline: Option<&str>) -> CompetitionInvitations {
        let coaches: Vec<String> = invites
            .iter()
            .map(|id| format!(r#"{{"id":"{id}","coach_name":"Coach","initials":"AB"}}"#))
            .collect();
        let d = deadline.map_or("null".into(), |x| format!("\"{x}\""));
        serde_json::from_str(&format!(
            r#"{{"access_mode":"{mode}","invited_coaches":[{}],
                 "max_participants":null,"registration_deadline":{d}}}"#,
            coaches.join(",")
        ))
        .unwrap()
    }

    fn journee(pairings: Vec<Pairing>) -> RoundRef {
        RoundRef {
            round_id: MatchId::try_new("01KZVCKDG19DXZHJA295WSJGMX").unwrap(),
            round_name: MatchDayName::try_new("Journée 3").unwrap(),
            date_start: crate::app::shared_kernel::bloodbowl::date_string::DateString::try_new(
                "2026-09-11",
            )
            .unwrap(),
            date_end: None,
            day_type: MatchDayType::FixedDate,
            pairings,
        }
    }

    fn appariement(home: &str, away: &str) -> Pairing {
        Pairing {
            id: PairingId::try_new("01KZVCKDG19DXZHJA295WSJGP1").unwrap(),
            home_team_id: TeamId::try_new(home).unwrap(),
            away_team_id: TeamId::try_new(away).unwrap(),
        }
    }

    const A: &str = "01KZVCKDG19DXZHJA295WSJGM1";
    const B: &str = "01KZVCKDG19DXZHJA295WSJGM2";
    const T1: &str = "01KZVCKDG19DXZHJA295WSJGT1";
    const T2: &str = "01KZVCKDG19DXZHJA295WSJGT2";

    // ── R7 ───────────────────────────────────────────────────────────────────

    /// **Le test de R7.** Le coach B est inscrit avec une équipe mais n'est pas
    /// membre de l'espace : il n'a donc aucune adresse, et ne peut pas être
    /// destinataire. Ce n'est pas un filtre qu'on aurait pu oublier — c'est le
    /// seul chemin vers un e-mail qui ne passe pas par lui.
    #[tokio::test]
    async fn un_coach_hors_espace_n_est_jamais_destinataire() {
        let space = espace();
        let membres = Membres(vec![membre(A, "alice")]);
        let equipes = Equipes(vec![equipe(T1, "Les Uns", A), equipe(T2, "Les Deux", B)]);

        let r = resolve(
            NotificationType::RoundEve,
            &contexte(&space, None),
            Some(&journee(vec![])),
            &equipes,
            &membres,
        )
        .await;

        assert_eq!(r.len(), 1);
        assert_eq!(r[0].coach_id, A);
    }

    // ── Date limite ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn la_date_limite_exclut_ceux_qui_se_sont_deja_inscrits() {
        let space = espace();
        let inv = invitations("open", &[], Some("2026-09-13"));
        let membres = Membres(vec![membre(A, "alice"), membre(B, "bob")]);
        let equipes = Equipes(vec![equipe(T1, "Les Uns", A)]);

        let r = resolve(
            NotificationType::RegistrationDeadline,
            &contexte(&space, Some(&inv)),
            None,
            &equipes,
            &membres,
        )
        .await;

        assert_eq!(r.len(), 1, "seul le non-inscrit doit être relancé");
        assert_eq!(r[0].coach_id, B);
    }

    #[tokio::test]
    async fn en_mode_invitation_l_ouverture_ne_touche_que_les_invites_encore_membres() {
        let space = espace();
        let inv = invitations("invitation", &[A], None);
        let membres = Membres(vec![membre(A, "alice"), membre(B, "bob")]);

        let r = resolve(
            NotificationType::RegistrationOpen,
            &contexte(&space, Some(&inv)),
            None,
            &Equipes(vec![]),
            &membres,
        )
        .await;

        assert_eq!(r.len(), 1);
        assert_eq!(r[0].coach_id, A);
    }

    // ── Participation ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn un_coach_inscrit_sans_match_ce_jour_la_est_not_playing() {
        let space = espace();
        let membres = Membres(vec![membre(A, "alice")]);
        let equipes = Equipes(vec![equipe(T1, "Les Uns", A)]);

        let r = resolve(
            NotificationType::RoundEve,
            &contexte(&space, None),
            Some(&journee(vec![])),
            &equipes,
            &membres,
        )
        .await;

        assert_eq!(r[0].participation, RoundParticipation::NotPlaying);
    }

    /// **Le cas qui interdit un `Option<Fixture>`.** Un coach qui aligne deux
    /// équipes le même jour reçoit **un** e-mail — la clé d'idempotence ne porte
    /// pas d'équipe — et doit y voir **ses deux matchs**.
    #[tokio::test]
    async fn un_coach_a_deux_equipes_rend_un_destinataire_et_deux_fixtures() {
        let space = espace();
        let membres = Membres(vec![membre(A, "alice"), membre(B, "bob")]);
        let equipes = Equipes(vec![
            equipe(T1, "Les Uns", A),
            equipe(T2, "Les Deux", A),
            equipe("01KZVCKDG19DXZHJA295WSJGT3", "Les Trois", B),
        ]);
        let round = journee(vec![
            appariement(T1, "01KZVCKDG19DXZHJA295WSJGT3"),
            appariement("01KZVCKDG19DXZHJA295WSJGT3", T2),
        ]);

        let r = resolve(
            NotificationType::RoundEve,
            &contexte(&space, None),
            Some(&round),
            &equipes,
            &membres,
        )
        .await;

        let alice = r.iter().find(|x| x.coach_id == A).unwrap();
        assert_eq!(r.iter().filter(|x| x.coach_id == A).count(), 1);
        match &alice.participation {
            RoundParticipation::Playing(f) => {
                assert_eq!(f.len(), 2, "les deux matchs doivent y être");
                assert!(f.iter().any(|x| x.team_name == "Les Uns"));
                assert!(f.iter().any(|x| x.team_name == "Les Deux"));
                assert!(f.iter().all(|x| !x.away_team.is_empty()));
            }
            autre => panic!("attendu Playing, reçu {autre:?}"),
        }
    }
}
