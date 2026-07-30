//! Ce que deux chemins de création de joueur ont en commun.
//!
//! `TeamCreated` en crée N d'un coup, `PlayerRecruited` un seul. Le corps est
//! le même : résoudre les compétences de base et la valeur de départ depuis le
//! catalogue, puis appender l'événement de création avec sa projection, dans
//! une seule transaction.
//!
//! Le code vient de `team_created_listener`, déplacé ici sans réécriture.

use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::player::{PlayerId, Spp, TeamId, ValueKpo};
use crate::app::players::domain::value_objects::{JerseyVo, PositionNameVo, RosterLineId, SkillId};
use crate::app::players::io::repository::player_repository::{
    insert_player_event, upsert_player_projection,
};
use crate::app::players::ports::{ISkillCatalogPort, RepositoryError};
use crate::app::shared_kernel::identity::ids::SpaceId;
use sqlx::PgPool;
use std::fmt;

#[derive(Debug)]
pub enum ListenerError {
    AlreadyProcessed,
    Repository(RepositoryError),
    Database(sqlx::Error),
}

impl fmt::Display for ListenerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyProcessed => write!(f, "joueur déjà créé (idempotence)"),
            Self::Repository(e) => write!(f, "repository : {e}"),
            Self::Database(e) => write!(f, "base de données : {e}"),
        }
    }
}

impl From<RepositoryError> for ListenerError {
    fn from(e: RepositoryError) -> Self {
        match e {
            RepositoryError::ConcurrentWrite => Self::AlreadyProcessed,
            other => Self::Repository(other),
        }
    }
}

// ── Résolution depuis le référentiel ─────────────────────────────────────────

pub fn resolve_base_skills(roster_line_id: &str, catalog: &dyn ISkillCatalogPort) -> Vec<SkillId> {
    catalog
        .find_position(roster_line_id)
        .map(|pos| {
            pos.base_skills
                .iter()
                .filter_map(|uid| SkillId::try_new(uid.clone()).ok())
                .collect()
        })
        .unwrap_or_default()
}

pub fn base_position_kpo(roster_line_id: &str, catalog: &dyn ISkillCatalogPort) -> u32 {
    catalog
        .find_position(roster_line_id)
        .map(|pos| pos.cost)
        .unwrap_or(0)
}

pub fn nom_de_poste(roster_line_id: &str, catalog: &dyn ISkillCatalogPort) -> String {
    catalog
        .find_position(roster_line_id)
        .map(|pos| pos.position_name)
        .unwrap_or_else(|| "Joueur".to_string())
}

// ── Création ─────────────────────────────────────────────────────────────────

/// Appende `PlayerCreated` en version 1, avec sa projection, dans une seule
/// transaction.
///
/// Un identifiant déjà connu remonte en `AlreadyProcessed` par la contrainte
/// d'unicité `(player_id, version)` : c'est ce qui rend l'opération idempotente,
/// et c'est la raison pour laquelle l'identifiant est frappé en amont plutôt
/// qu'ici.
pub async fn creer_joueur(
    team_id: &str,
    space_id: &str,
    player_id: &str,
    roster_line_id: &str,
    position_name: &str,
    jersey: Option<u16>,
    pool: &PgPool,
    catalog: &dyn ISkillCatalogPort,
) -> Result<(), ListenerError> {
    let created = PlayerDomainEvent::PlayerCreated {
        player_id: PlayerId(player_id.to_string()),
        team_id: TeamId(team_id.to_string()),
        space_id: SpaceId::try_new(space_id).unwrap_or_else(|_| SpaceId::new()),
        position_name: PositionNameVo::try_new(position_name.to_string())
            .unwrap_or_else(|_| PositionNameVo::try_new("Joueur".to_string()).unwrap()),
        roster_line_id: RosterLineId::try_new(roster_line_id.to_string())
            .unwrap_or_else(|_| RosterLineId::try_new("unknown".to_string()).unwrap()),
        jersey: jersey.and_then(|j| JerseyVo::try_new(j).ok()),
        base_skills: resolve_base_skills(roster_line_id, catalog),
        starting_spp: Spp(0),
        starting_value: ValueKpo(base_position_kpo(roster_line_id, catalog)),
    };

    let mut tx = pool.begin().await.map_err(ListenerError::Database)?;
    insert_player_event(&mut tx, &created, 1).await?;
    upsert_player_projection(&mut tx, &created).await?;
    tx.commit().await.map_err(ListenerError::Database)?;
    Ok(())
}

// ── Maillots ─────────────────────────────────────────────────────────────────

/// Le plus petit numéro non pris dans l'équipe.
///
/// Un trou laissé par un départ est donc rebouché, ce qui est le comportement
/// attendu : les numéros sont une ressource de seize places, pas une suite
/// chronologique.
pub async fn prochain_maillot_libre(team_id: &str, pool: &PgPool) -> Option<u16> {
    let lignes: Vec<(Option<i16>,)> =
        sqlx::query_as("SELECT jersey FROM players_proj WHERE team_id = $1")
            .bind(team_id)
            .fetch_all(pool)
            .await
            .ok()?;
    let pris: Vec<u16> = lignes
        .into_iter()
        .filter_map(|(j,)| j)
        .filter(|j| *j > 0)
        .map(|j| j as u16)
        .collect();
    premier_libre(&pris)
}

/// La règle, isolée de la base pour être éprouvable sans elle.
pub fn premier_libre(pris: &[u16]) -> Option<u16> {
    (1..=MAILLOTS).find(|n| !pris.contains(n))
}

/// Seize joueurs par équipe, donc seize numéros.
const MAILLOTS: u16 = 16;

#[cfg(test)]
mod tests {
    use super::premier_libre;

    /// Le lot est traité séquentiellement : chaque recrutement voit l'état
    /// laissé par le précédent, ce qui interdit à deux joueurs du même lot de
    /// réserver le même numéro.
    #[test]
    fn trois_recrutements_successifs_prennent_trois_numeros_distincts() {
        let mut pris: Vec<u16> = vec![];
        let mut attribues = vec![];
        for _ in 0..3 {
            let n = premier_libre(&pris).expect("un numéro reste libre");
            attribues.push(n);
            pris.push(n);
        }
        assert_eq!(attribues, vec![1, 2, 3]);
    }

    /// Un numéro qui n'est plus porté est repris. C'est le comportement voulu :
    /// les maillots sont une ressource de seize places, pas une suite
    /// chronologique.
    #[test]
    fn un_numero_libere_est_reattribue() {
        assert_eq!(premier_libre(&[1, 2, 4, 5]), Some(3));
    }

    #[test]
    fn une_equipe_complete_n_a_plus_de_numero() {
        let pleine: Vec<u16> = (1..=16).collect();
        assert_eq!(premier_libre(&pleine), None);
    }

    /// Les trous ne sont pas cherchés au-delà du plafond : un numéro hors
    /// bornes, s'il existait en base, ne décalerait pas l'attribution.
    #[test]
    fn un_numero_hors_bornes_ne_perturbe_pas_l_attribution() {
        assert_eq!(premier_libre(&[1, 2, 99]), Some(3));
    }
}
