//! Ce qu'il advient d'un journalier : embauché, perdu, ou désaligné.
//!
//! # Trois causes, trois faits
//!
//! | Cause | Fait | Ce que le coach a fait |
//! |---|---|---|
//! | il a été gardé au panier | `JourneymanHired` | il l'a payé |
//! | la phase de recrutement se clôt | `JourneymanLost` | il ne l'a pas retenu |
//! | le rapport est annulé | `JourneymanWithdrawn` | rien — le match n'aura pas lieu |
//!
//! **L'embauche est la seule qui le garde**, et elle vit ici parce qu'elle est
//! le pendant exact de la perte : deux sorties de l'état journalier, décidées
//! par `teams`, appliquées par `players`. Les séparer aurait éloigné deux
//! moitiés d'une même bascule.
//!
//! Les deux départs posent `membership: Dismissed`, et c'est **volontairement**
//! le même état : toutes les lectures d'effectif filtrent dessus, et un
//! journalier qui part est un journalier qui part. Ce qui diffère est
//! l'histoire, et l'event store la garde — c'est aussi pourquoi ni l'un ni
//! l'autre n'est un `PlayerDismissed`, qui raconterait une décision de renvoi
//! jamais prise.
//!
//! # Pourquoi le ménage ne peut pas perdre un joueur qu'on vient de payer
//!
//! `RecruitmentPhaseValidated` est le **dernier** événement du lot que
//! `validate_recruitment_phase_use_case` appende : les `JourneymanRecruited`
//! le précèdent. Les app events sortent dans cet ordre, ce listener les traite
//! dans cet ordre, et quand la clôture arrive les gardés sont déjà `Active`.
//!
//! **Cet ordre a été affirmé avant d'exister.** Le bras de publisher qui fait
//! sortir `JourneymanRecruited` manquait : le coach payait et perdait son
//! joueur, alors que ce commentaire disait déjà le contraire. C'est le parcours
//! de bout en bout qui l'a trouvé, et
//! `test_le_journalier_recrute_reste_dans_l_effectif` qui le tient désormais.
//!
//! `init(app_event_bus: …)` : c'est cette signature que l'axe 5 de
//! `check-arch` reconnaît comme listener cross-BC.

use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::player::{Player, PlayerId, TeamId};
use crate::app::players::io::app_events::player_creation::ListenerError;
use crate::app::players::io::repository::player_repository::{
    insert_player_event, upsert_player_projection,
};
use crate::app::players::ports::IPlayerRepository;
use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportAppEvent;
use crate::app::shared_kernel::app_events::teams_app_events::TeamsAppEvent;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::Instrument;

pub fn init(app_event_bus: &EventBus, pool: PgPool, repository: Arc<dyn IPlayerRepository>) {
    let mut rx = app_event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    match serde_json::from_value::<TeamsAppEvent>(envelope.payload.clone()) {
                        Ok(TeamsAppEvent::JourneymanRecruited {
                            team_id, player_id, ..
                        }) => {
                            embaucher(
                                &player_id.to_string(),
                                &team_id.to_string(),
                                &pool,
                                repository.as_ref(),
                            )
                            .instrument(span)
                            .await;
                            continue;
                        }
                        Ok(TeamsAppEvent::RecruitmentPhaseValidated { team_id, .. }) => {
                            perdre_les_restants(&team_id.to_string(), &pool, repository.as_ref())
                                .instrument(span)
                                .await;
                            continue;
                        }
                        _ => {}
                    }
                    if let Ok(MatchReportAppEvent::MatchReportCancelled {
                        home_team_id,
                        journeymen,
                        ..
                    }) = serde_json::from_value::<MatchReportAppEvent>(envelope.payload.clone())
                    {
                        retirer_les_nommes(&home_team_id, journeymen, &pool, repository.as_ref())
                            .instrument(span)
                            .await;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("players journeyman_departure_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// Le journalier que le coach a gardé devient permanent.
///
/// **La seule sortie qui ne le perd pas.** Sans elle, la clôture de phase le
/// trouve encore journalier et l'emporte — alors que sa trésorerie a déjà été
/// débitée.
async fn embaucher(
    player_id: &str,
    team_id: &str,
    pool: &PgPool,
    repository: &dyn IPlayerRepository,
) {
    let id = PlayerId(player_id.to_string());
    let joueur = match repository.find_by_id(&id).await {
        Ok(Some(j)) => j,
        Ok(None) => {
            tracing::warn!("players journeyman_departure_listener: {player_id} introuvable");
            return;
        }
        Err(e) => {
            tracing::error!("players journeyman_departure_listener: lecture {player_id}: {e}");
            return;
        }
    };
    // Déjà embauché — l'événement rejoué, ou reçu deux fois. Sans cette garde,
    // l'append échouerait sur la version pour un état correct.
    if !est_journalier(&&joueur) {
        return;
    }
    let event = PlayerDomainEvent::JourneymanHired {
        player_id: id,
        team_id: TeamId(team_id.to_string()),
    };
    appliquer(pool, &event, joueur.version + 1, player_id).await;
}

/// Tous les journaliers qui restent dans l'effectif à la clôture de la phase.
async fn perdre_les_restants(team_id: &str, pool: &PgPool, repository: &dyn IPlayerRepository) {
    let restants = match repository
        .find_by_team_id(&TeamId(team_id.to_string()))
        .await
    {
        Ok(joueurs) => joueurs,
        Err(e) => {
            tracing::error!("players journeyman_departure_listener: effectif {team_id}: {e}");
            return;
        }
    };
    for joueur in restants.iter().filter(est_journalier) {
        let event = PlayerDomainEvent::JourneymanLost {
            player_id: joueur.id.clone(),
            team_id: TeamId(team_id.to_string()),
        };
        appliquer(pool, &event, joueur.version + 1, &joueur.id.0).await;
    }
}

fn est_journalier(joueur: &&Player) -> bool {
    use crate::app::players::domain::player::RosterMembership;
    joueur.membership == RosterMembership::Journeyman
}

/// Les journaliers que l'événement d'annulation nomme.
///
/// **Nommés plutôt que retrouvés par équipe**, et c'est ce qui protège les
/// autres : chercher tous les journaliers de l'équipe retirerait aussi ceux
/// d'un rapport antérieur non encore traité — cas rare, mais destructeur.
///
/// L'équipe n'est là que pour l'événement ; le joueur porte la sienne, et c'est
/// elle qui sera écrite.
async fn retirer_les_nommes(
    team_id: &str,
    journeymen: Vec<String>,
    pool: &PgPool,
    repository: &dyn IPlayerRepository,
) {
    for id in journeymen {
        let player_id = PlayerId(id);
        let joueur = match repository.find_by_id(&player_id).await {
            Ok(Some(j)) => j,
            Ok(None) => continue,
            Err(e) => {
                tracing::error!(
                    "players journeyman_departure_listener: lecture {}: {e}",
                    player_id.0
                );
                continue;
            }
        };
        // Un journalier déjà parti — recruté, ou retiré par un autre chemin —
        // n'a rien à défaire. Sans cette garde, l'append échouerait sur la
        // version et remplirait le journal d'erreurs pour un état correct.
        if !est_journalier(&&joueur) {
            continue;
        }
        let event = PlayerDomainEvent::JourneymanWithdrawn {
            player_id: player_id.clone(),
            team_id: TeamId(team_id.to_string()),
        };
        appliquer(pool, &event, joueur.version + 1, &player_id.0).await;
    }
}

/// L'événement et sa projection dans **une seule transaction**, comme toute
/// écriture event-sourcée de ce BC. L'échec de l'un ne prive pas les autres.
async fn appliquer(pool: &PgPool, event: &PlayerDomainEvent, version: i32, player_id: &str) {
    let resultat: Result<(), ListenerError> = async {
        let mut tx = pool.begin().await.map_err(ListenerError::Database)?;
        insert_player_event(&mut tx, event, version).await?;
        upsert_player_projection(&mut tx, event).await?;
        tx.commit().await.map_err(ListenerError::Database)?;
        Ok(())
    }
    .await;
    if let Err(e) = resultat {
        tracing::error!("players journeyman_departure_listener: {player_id}: {e}");
    }
}
