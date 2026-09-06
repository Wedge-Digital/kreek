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
    AcquisitionMode, PlayerId, RosterMembership, Spp, TeamId, ValueKpo, SOLITAIRE_DU_JOURNALIER,
};
use crate::app::players::domain::value_objects::{
    JerseyVo, PersonalName, PositionNameVo, RosterLineId, SkillId, SkillName, SppCost,
};
use crate::app::players::io::app_events::team_created_listener::skill_category_css;
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
        base_skills: resolve_base_skills(roster_line_id, catalog),
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

    // Version 2 — le trait du journalier, dans sa propre transaction comme le
    // fait `team_created_listener` pour les compétences offertes.
    if membership == RosterMembership::Journeyman {
        if let Some(trait_) = solitaire_du_journalier(player_id, team_id, catalog) {
            let mut tx = pool.begin().await.map_err(ListenerError::Database)?;
            insert_player_event(&mut tx, &trait_, 2).await?;
            upsert_player_projection(&mut tx, &trait_).await?;
            tx.commit().await.map_err(ListenerError::Database)?;
        }
    }
    Ok(())
}

/// Solitaire (4+), en **compétence initiale bonus**.
///
/// La carte 502 l'avait posé dans `base_skills`, et personne ne l'a jamais vu :
/// le tableau d'effectif affiche les compétences **du poste**, pas celles du
/// joueur. Rangé là, le trait n'avait aucun écran pour le lire.
///
/// `InitialSkillEarned` est aussi le modèle **plus juste** : Solitaire n'est pas
/// une compétence de poste — un Trois-quart embauché ne l'a pas. C'est une
/// compétence de ce joueur-là, qu'il perd en devenant permanent.
///
/// **`value_delta` et `spp_cost` valent zéro.** Une compétence offerte augmente
/// la valeur du joueur ; celle-ci est un trait, pas un gain. Le prix d'un
/// journalier **est** sa valeur courante : la renchérir ferait afficher
/// « 65 + 20 d'amélioration » pour quelque chose qu'il n'a pas gagné au match.
fn solitaire_du_journalier(
    player_id: &str,
    team_id: &str,
    catalog: &dyn ISkillCatalogPort,
) -> Option<PlayerDomainEvent> {
    let nom = catalog
        .find_skill(SOLITAIRE_DU_JOURNALIER)
        .map(|s| s.name)
        .unwrap_or_else(|| "Solitaire (4+)".to_string());
    Some(PlayerDomainEvent::InitialSkillEarned {
        player_id: PlayerId(player_id.to_string()),
        team_id: TeamId(team_id.to_string()),
        skill_id: SkillId::try_new(SOLITAIRE_DU_JOURNALIER.to_string()).ok()?,
        skill_name: SkillName::try_new(nom).ok()?,
        category_css: skill_category_css("TRAITS").to_string(),
        // **`Customised`, et le mode décide de plus qu'un libellé.**
        //
        // `est_une_amelioration()` s'en sert pour compter le niveau du joueur,
        // donc le prix de sa compétence **suivante**. En `Chosen`, Solitaire y
        // comptait : le journalier recruté payait sa première vraie compétence
        // un niveau plus cher, pour un trait que le règlement lui a donné —
        // « faire payer un cadeau », le défaut exact de la carte 482.
        //
        // `Customised` dit ici « porté sans l'avoir payé », ce qu'il est.
        mode: AcquisitionMode::Customised,
        spp_cost: SppCost::try_new(0).ok()?,
        is_primary: false,
        is_elite: false,
        value_delta: ValueKpo(0),
    })
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
    use crate::app::players::domain::match_impact::StatKind;
    use crate::app::players::ports::{
        PositionAccessDto, PositionCatalogEntryDto, SkillCatalogEntryDto, SkillCostLevelDto,
        SppScaleDto,
    };

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

    /// **Le mode décide du prix de la compétence suivante**, pas seulement du
    /// libellé de la pastille — c'est pourquoi ce test vise le mode et non
    /// l'affichage. `est_une_amelioration()` compte les niveaux, et la 504
    /// avait posé `Chosen` : le journalier payait un cadeau.
    ///
    /// Le catalogue est muet à dessein — le libellé retombe alors sur le repli,
    /// et rien dans le test ne dépend du référentiel.
    #[test]
    fn solitaire_est_pose_en_customisation() {
        let Some(PlayerDomainEvent::InitialSkillEarned {
            mode,
            spp_cost,
            value_delta,
            skill_id,
            skill_name,
            ..
        }) = solitaire_du_journalier("p1", "t1", &CatalogueMuet)
        else {
            panic!("le trait doit être posé");
        };

        assert_eq!(mode, AcquisitionMode::Customised, "il ne l'a pas payé");
        assert!(
            !mode.est_une_amelioration(),
            "il ne doit pas faire monter le joueur d'un niveau"
        );
        assert_eq!(spp_cost.into_inner(), 0);
        assert_eq!(value_delta, ValueKpo(0));
        assert_eq!(skill_id.as_ref(), SOLITAIRE_DU_JOURNALIER);
        assert_eq!(skill_name.as_ref(), "Solitaire (4+)");
    }

    /// Muet : le trait ne dépend que de sa constante, pas du référentiel.
    struct CatalogueMuet;
    impl ISkillCatalogPort for CatalogueMuet {
        fn find_skill(&self, _: &str) -> Option<SkillCatalogEntryDto> {
            None
        }
        fn list_all_skills(&self) -> Vec<SkillCatalogEntryDto> {
            vec![]
        }
        fn find_position(&self, _: &str) -> Option<PositionCatalogEntryDto> {
            None
        }
        fn position_access(&self, _: &str) -> Option<PositionAccessDto> {
            None
        }
        fn cost_for_level(&self, _: u8, _: bool) -> Option<SkillCostLevelDto> {
            None
        }
        fn skill_value_delta(&self, _: bool, _: bool) -> u32 {
            0
        }
        fn stat_value_delta(&self, _: StatKind) -> u32 {
            0
        }
        fn spp_scale_for_roster_line(&self, _: &str) -> SppScaleDto {
            SppScaleDto {
                touchdown: 3,
                pass: 1,
                interception: 2,
                casualty: 2,
                mvp: 4,
            }
        }
    }
}
