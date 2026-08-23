//! Le second déclencheur : l'ouverture des inscriptions.
//!
//! # Pourquoi il n'est pas piloté par le cron
//!
//! R11. Les trois autres notifications se déclenchent sur une **date** comparée
//! à aujourd'hui ; celle-ci se déclenche sur un **fait** — la saison s'ouvre.
//! Il n'y a rien à comparer, et attendre le cron du lendemain ferait arriver
//! l'annonce un jour après l'ouverture.
//!
//! Ce que les deux chemins partagent — et c'est ce qui rend la scission
//! acceptable : le même use case d'expédition, le même journal, le même service
//! de destinataires, les mêmes gabarits. **Seul le déclencheur diffère.**
//!
//! # La date visée
//!
//! Celle de l'ouverture, pas celle de l'envoi — comme partout ailleurs dans
//! cette épic. Les deux coïncident ici, l'ouverture étant justement l'instant
//! du déclenchement, mais la clé reste de la même forme que les trois autres.

use crate::app::competitions::domain::notification_delivery::NotificationType;
use crate::app::competitions::use_cases::notification_dispatch::{
    dispatch, DispatchDeps, DispatchLabels, DispatchOutcome,
};
use crate::app::competitions::use_cases::notification_recipients::SeasonContext;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;

#[derive(Debug)]
pub struct SendRegistrationOpenCommand {
    pub season_id: String,
    pub space_id: String,
    pub competition_id: String,
    pub competition_name: String,
    pub season_name: String,
    pub space_name: String,
    /// L'e-mail dit « **X** t'invite à participer ». Vide, il disait
    /// « **** t'invite à participer » — ce qui est parti en vrai entre la carte
    /// 340 et cette correction.
    pub admin_name: String,
    pub opened_on: DateString,
}

#[tracing::instrument(skip_all, fields(season = %cmd.season_id, space = %cmd.space_id))]
pub async fn execute(
    cmd: SendRegistrationOpenCommand,
    invitations: Option<
        &crate::app::competitions::domain::competition_invitations::CompetitionInvitations,
    >,
    deps: &DispatchDeps<'_>,
) -> DispatchOutcome {
    let Ok(space) = crate::app::shared_kernel::identity::ids::SpaceId::try_new(&cmd.space_id)
    else {
        tracing::error!("identifiant d'espace invalide");
        return DispatchOutcome::default();
    };

    let season = SeasonContext {
        space_id: &space,
        season_id: &cmd.season_id,
        invitations,
    };
    let labels = DispatchLabels {
        competition_name: cmd.competition_name.clone(),
        season_name: cmd.season_name.clone(),
        space_name: cmd.space_name.clone(),
        admin_name: cmd.admin_name.clone(),
        competition_url: format!(
            "{}/app/{}/competitions/{}/{}",
            deps.app_url, cmd.space_id, cmd.competition_id, cmd.season_id
        ),
        registration_deadline: invitations
            .and_then(|i| i.registration_deadline.clone())
            .unwrap_or_default(),
        remaining_slots: String::new(),
    };
    // L'e-mail d'ouverture ne parle pas de places restantes : à l'ouverture, il
    // n'y a pas encore d'inscrit. C'est la relance de date limite qui le fait.

    // Pas de journée : l'ouverture concerne la saison, pas une date de jeu.
    // C'est le cas que l'index protège par `COALESCE(round_id, '')`.
    dispatch(
        NotificationType::RegistrationOpen,
        &season,
        None,
        &cmd.opened_on,
        &labels,
        deps,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::io::repository::notification_delivery_repository::NotificationDeliveryRepository;
    use crate::app::competitions::ports::{
        ICompetitionSpaceMemberPort, ITeamInfoPort, SpaceMemberDto, TeamInfoDto,
    };
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
    use crate::common::services::email::{EmailError, IEmailService};
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Espion(Mutex<Vec<String>>);

    #[async_trait]
    impl IEmailService for Espion {
        async fn send(&self, _: Vec<String>, _: String, html: String) -> Result<(), EmailError> {
            self.0.lock().unwrap().push(html);
            Ok(())
        }
    }

    struct Membres;
    #[async_trait]
    impl ICompetitionSpaceMemberPort for Membres {
        async fn list_space_members(&self, _: &SpaceId) -> Vec<SpaceMemberDto> {
            vec![SpaceMemberDto {
                coach_id: "01KZVCKDG19DXZHJA295WSJGM1".into(),
                coach_name: "Alice".into(),
                email: "alice@example.test".into(),
            }]
        }
        async fn find_member_profile(
            &self,
            _: &CoachId,
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

    struct Equipes;
    #[async_trait]
    impl ITeamInfoPort for Equipes {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(Vec::new())
        }
        async fn find_team_names(&self, _: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(Vec::new())
        }
    }

    /// **Le test qui manquait, et qui a laissé partir des e-mails troués.**
    ///
    /// Il ne vérifie pas qu'un gabarit rend ce qu'on lui donne — celui-là
    /// existait déjà et passait pendant que le défaut vivait. Il vérifie que la
    /// **commande** arrive jusqu'au HTML : entre les deux, trois champs étaient
    /// câblés en `String::new()`, et l'e-mail d'ouverture disait
    /// « **** t'invite à participer ».
    #[sqlx::test]
    async fn les_noms_de_la_commande_arrivent_dans_l_email(pool: sqlx::PgPool) {
        let espion = Espion::default();
        let journal = NotificationDeliveryRepository::new(pool);
        let deps = DispatchDeps {
            teams: &Equipes,
            members: &Membres,
            journal: &journal,
            email: &espion,
            app_url: "https://kreek.example",
        };

        execute(
            SendRegistrationOpenCommand {
                season_id: "01KZVCKDG19DXZHJA295WSJGMV".into(),
                space_id: "01KZVCJZ35JZWQB0KNTA9JJEPX".into(),
                competition_id: "01KZVCKDG19DXZHJA295WSJGMW".into(),
                competition_name: "Coupe de Fer".into(),
                season_name: "Saison 1".into(),
                space_name: "Ligue du Nord".into(),
                admin_name: "Nobbla".into(),
                opened_on: DateString::try_new("2026-09-10").unwrap(),
            },
            None,
            &deps,
        )
        .await;

        let envois = espion.0.lock().unwrap();
        assert_eq!(envois.len(), 1);
        for attendu in ["Nobbla", "Ligue du Nord", "Coupe de Fer", "Saison 1"] {
            assert!(
                envois[0].contains(attendu),
                "« {attendu} » n'est pas arrivé jusqu'à l'e-mail"
            );
        }
    }
}
