use crate::app::teams::domain::recruitment_basket::{
    BasketLine, BasketVersion, CatalogPosition, CrossLimit, OwnedStaff, RecruitmentBasket,
    RosterCatalog, RosterLineId, SquadMember, SquadSnapshot, StaffCatalogEntry,
};
use crate::app::teams::domain::team::{GamePhase, Team};
use crate::app::teams::domain::value_objects::Kpo;
use crate::app::teams::ports::{
    IPhaseBasketRepository, IRosterCatalogPort, ISquadPort, RepositoryError, RosterCatalogDto,
    SquadMemberDto,
};

#[derive(Debug)]
pub enum HydrationError {
    RosterNotFound,
    CorruptedBasket(String),
    Repository(RepositoryError),
}

impl std::fmt::Display for HydrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RosterNotFound => write!(f, "roster introuvable dans le catalogue"),
            Self::CorruptedBasket(e) => write!(f, "panier illisible : {e}"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

/// Reconstitue le panier de recrutement à partir de ses quatre sources.
///
/// **C'est le seul endroit du BC où les DTOs de port sont manipulés.** Au-delà,
/// tout est domaine : ni handler ni template ne voit un `RosterCatalogDto` ou un
/// `SquadMemberDto`.
///
/// L'hydratation se fait **contre l'état du jour** — prix, effectif et
/// trésorerie rechargés à chaque fois, jamais ceux de la constitution du panier.
/// C'est ce qui fait qu'un panier vieux de dix minutes est évalué contre les
/// données d'aujourd'hui.
pub async fn hydrate_recruitment_basket(
    team: &Team,
    basket_repo: &dyn IPhaseBasketRepository,
    catalog_port: &dyn IRosterCatalogPort,
    squad_port: &dyn ISquadPort,
) -> Result<RecruitmentBasket, HydrationError> {
    let team_id = team.id.to_string();
    let roster_id = team.roster_id.to_string();

    let persiste = basket_repo
        .load(&team_id, &GamePhase::Recruitment)
        .await
        .map_err(HydrationError::Repository)?;

    // Un panier absent n'est pas une erreur : le coach n'a simplement rien mis
    // dedans. On hydrate un panier vide, à la version zéro — celle que `save`
    // attend pour créer la ligne.
    let (version, lines) = match persiste {
        Some(etat) => (
            BasketVersion(etat.version),
            serde_json::from_value::<Vec<BasketLine>>(etat.state)
                .map_err(|e| HydrationError::CorruptedBasket(e.to_string()))?,
        ),
        None => (BasketVersion(0), Vec::new()),
    };

    let catalogue = catalog_port
        .find_catalog(&roster_id)
        .ok_or(HydrationError::RosterNotFound)?;

    let effectif = squad_port.find_squad(&team_id).await;

    Ok(RecruitmentBasket::hydrate(
        team_id,
        version,
        lines,
        to_domain_catalog(catalogue),
        to_domain_squad(effectif),
        owned_staff_of(team),
        team.treasury,
    ))
}

fn to_domain_catalog(dto: RosterCatalogDto) -> RosterCatalog {
    RosterCatalog {
        positions: dto
            .positions
            .into_iter()
            .map(|p| CatalogPosition {
                uid: RosterLineId(p.uid),
                position_name: p.position_name,
                cost: Kpo(p.cost),
                max_quantity: p.max_quantity,
            })
            .collect(),
        cross_limits: dto
            .cross_limits
            .into_iter()
            .map(|c| CrossLimit {
                max: c.max,
                position_uids: c.position_uids.into_iter().map(RosterLineId).collect(),
            })
            .collect(),
        allowed_staff: dto.allowed_staff,
        staff: dto
            .staff_prices
            .into_iter()
            .map(|s| StaffCatalogEntry {
                uid: s.uid,
                price: Kpo(s.price),
                max_quantity: s.max_quantity,
            })
            .collect(),
        reroll_base_cost: Kpo(dto.reroll_base_cost),
    }
}

/// Tous les joueurs comptent pour les quotas, disponibles ou non : un blessé
/// occupe toujours sa place dans l'effectif. `available_for_next_match` sert au
/// calcul de valeur d'équipe, pas au recrutement.
fn to_domain_squad(membres: Vec<SquadMemberDto>) -> SquadSnapshot {
    SquadSnapshot {
        members: membres
            .into_iter()
            .map(|m| SquadMember {
                roster_line: RosterLineId(m.roster_line_id),
            })
            .collect(),
    }
}

fn owned_staff_of(team: &Team) -> OwnedStaff {
    OwnedStaff {
        rerolls: team.rerolls.0 as u32,
        apothecaries: team.apothecaries.0 as u32,
        assistants: team.assistants.0 as u32,
        cheerleaders: team.cheerleaders.0 as u32,
    }
}
