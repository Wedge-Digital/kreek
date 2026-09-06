//! Ce que deux chemins de création de joueur ont en commun.
//!
//! `TeamCreated` en crée N d'un coup, `PlayerRecruited` un seul. Le corps est
//! le même : résoudre les compétences de base et la valeur de départ depuis le
//! catalogue, puis appender l'événement de création avec sa projection, dans
//! une seule transaction.
//!
//! Le code vient de `team_created_listener`, déplacé ici sans réécriture.

use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::player::{
    PlayerId, RosterMembership, Spp, TeamId, ValueKpo, SOLITAIRE_DU_JOURNALIER,
};
use crate::app::players::domain::value_objects::{
    JerseyVo, PersonalName, PositionNameVo, RosterLineId, SkillId,
};
use crate::app::players::io::repository::player_repository::{
    insert_player_event, upsert_player_projection,
};
use crate::app::players::ports::{IPlayerProjectionRepository, ISkillCatalogPort, RepositoryError};
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
/// `membership` distingue les deux naissances : `Active` pour un joueur que le
/// coach a recruté, `Journeyman` pour un journalier aligné le temps d'un match.
/// Il est **explicite au lieu d'être déduit** — un défaut à `Active` ferait
/// naître un journalier permanent le jour où un appelant l'oublie, et rien ne
/// le signalerait.
#[allow(clippy::too_many_arguments)]
pub async fn creer_joueur(
    team_id: &str,
    space_id: &str,
    player_id: &str,
    roster_line_id: &str,
    position_name: &str,
    jersey: Option<u16>,
    membership: RosterMembership,
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
        base_skills: competences_de_naissance(roster_line_id, membership, catalog),
        starting_spp: Spp(0),
        starting_value: ValueKpo(base_position_kpo(roster_line_id, catalog)),
        starting_membership: membership,
        starting_personal_name: match membership {
            RosterMembership::Journeyman => PersonalName::try_new(nom_de_journalier(jersey)).ok(),
            _ => None,
        },
    };

    let mut tx = pool.begin().await.map_err(ListenerError::Database)?;
    insert_player_event(&mut tx, &created, 1).await?;
    upsert_player_projection(&mut tx, &created).await?;
    tx.commit().await.map_err(ListenerError::Database)?;
    Ok(())
}

/// Les compétences qu'un joueur porte en naissant.
///
/// Celles de son poste, plus **Solitaire (4+) pour un journalier** : le LRB le
/// lui donne d'office, et le lui retire quand il est embauché. C'est la seule
/// compétence de base de l'application qui puisse disparaître — voir
/// `PlayerDomainEvent::JourneymanHired`.
fn competences_de_naissance(
    roster_line_id: &str,
    membership: RosterMembership,
    catalog: &dyn ISkillCatalogPort,
) -> Vec<SkillId> {
    let mut skills = resolve_base_skills(roster_line_id, catalog);
    if membership == RosterMembership::Journeyman {
        if let Ok(solitaire) = SkillId::try_new(SOLITAIRE_DU_JOURNALIER.to_string()) {
            skills.push(solitaire);
        }
    }
    skills
}

/// Le nom d'un journalier, qui n'en a pas reçu de son coach.
///
/// **Sans lui, deux journaliers d'un même poste sont indiscernables** : le
/// nom retombe sur celui du poste, et l'écran de recrutement affiche deux
/// « Trois-quart » qu'on ne peut pas distinguer.
///
/// Il **survit à l'embauche**, et c'est voulu : le coach reconnaît celui qu'il a
/// gardé, et peut le renommer comme n'importe quel joueur. Le remettre à vide
/// lui retirerait son seul repère.
pub fn nom_de_journalier(jersey: Option<u16>) -> String {
    match jersey {
        Some(n) => format!("Journalier #{n}"),
        // Un journalier sans maillot ne devrait pas exister — les seize numéros
        // ne peuvent pas tous être pris, puisqu'il vient combler un trou.
        None => "Journalier".to_string(),
    }
}

// ── Maillots ─────────────────────────────────────────────────────────────────

/// Le plus petit numéro non pris dans l'équipe.
///
/// Un trou laissé par un départ est donc rebouché, ce qui est le comportement
/// attendu : les numéros sont une ressource de seize places, pas une suite
/// chronologique.
///
/// Les numéros portés viennent du repository et non d'une requête écrite ici.
/// La carte 265 promettait qu'un maillot libéré par un renvoi redeviendrait
/// disponible « d'office, puisque le joueur `Dismissed` n'est plus lu par
/// `find_by_team_id` » — c'était faux tant que cette fonction lisait
/// `players_proj` par elle-même. Le filtre d'appartenance ne vaut que là où
/// toutes les lectures passent.
pub async fn prochain_maillot_libre(
    team_id: &str,
    projections: &dyn IPlayerProjectionRepository,
) -> Option<u16> {
    let pris = projections
        .jerseys_by_team_id(&TeamId(team_id.to_string()))
        .await
        .ok()?;
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

#[cfg(test)]
mod tests_journalier {
    use super::*;

    /// Sans nom, deux journaliers d'un même poste sont indiscernables : le nom
    /// retombe sur celui du poste, et l'écran affiche deux « Trois-quart ».
    #[test]
    fn un_journalier_est_nomme_avec_son_maillot() {
        assert_eq!(nom_de_journalier(Some(13)), "Journalier #13");
        assert_eq!(nom_de_journalier(Some(1)), "Journalier #1");
    }

    /// Un journalier sans maillot ne devrait pas exister — les seize numéros ne
    /// peuvent pas tous être pris, puisqu'il vient combler un trou. Le repli
    /// existe pour ne pas afficher « Journalier # » tout court.
    #[test]
    fn sans_maillot_il_garde_un_nom_lisible() {
        assert_eq!(nom_de_journalier(None), "Journalier");
    }
}
