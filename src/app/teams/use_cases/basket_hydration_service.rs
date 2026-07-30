use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
use crate::app::teams::domain::basket::{
    BasketVersion, CatalogPosition, CrossLimit, OwnedStaff, Player, RosterCatalog, RosterLineId,
    SkillBadge, Squad, StaffCatalogEntry,
};
use crate::app::teams::domain::dismissals_basket::{DismissalBasketLine, DismissalsBasket};
use crate::app::teams::domain::recruitment_basket::{BasketLine, RecruitmentBasket};
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
    /// Un identifiant de joueur illisible. L'hydratation échoue au lieu de
    /// sauter le membre : un effectif amputé compterait un éligible de moins,
    /// et le plancher des renvois autoriserait un renvoi qu'il doit refuser.
    /// Mieux vaut ne rien afficher qu'afficher un effectif faux.
    CorruptedSquad(String),
    Repository(RepositoryError),
}

impl std::fmt::Display for HydrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RosterNotFound => write!(f, "roster introuvable dans le catalogue"),
            Self::CorruptedBasket(e) => write!(f, "panier illisible : {e}"),
            Self::CorruptedSquad(e) => write!(f, "effectif illisible : {e}"),
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
    let (version, lines) =
        charger_lignes::<BasketLine>(&team_id, &GamePhase::Recruitment, basket_repo).await?;
    let catalogue = charger_catalogue(team, catalog_port)?;
    let effectif = squad_port.find_squad(&team_id).await;

    Ok(RecruitmentBasket::hydrate(
        team_id,
        version,
        lines,
        catalogue,
        to_domain_squad(effectif)?,
        owned_staff_of(team),
        team.treasury,
    ))
}

/// Le pendant pour les renvois. Mêmes sources, moins la trésorerie : un renvoi
/// ne rembourse rien, l'agrégat n'a aucune raison de la connaître.
pub async fn hydrate_dismissals_basket(
    team: &Team,
    basket_repo: &dyn IPhaseBasketRepository,
    catalog_port: &dyn IRosterCatalogPort,
    squad_port: &dyn ISquadPort,
) -> Result<DismissalsBasket, HydrationError> {
    let team_id = team.id.to_string();
    let (version, lines) =
        charger_lignes::<DismissalBasketLine>(&team_id, &GamePhase::Dismissals, basket_repo)
            .await?;
    let catalogue = charger_catalogue(team, catalog_port)?;
    let effectif = squad_port.find_squad(&team_id).await;

    Ok(DismissalsBasket::hydrate(
        team_id,
        version,
        lines,
        to_domain_squad(effectif)?,
        catalogue,
        owned_staff_of(team),
    ))
}

/// Les lignes persistées d'un panier de phase, quel que soit leur type.
///
/// Un panier absent n'est pas une erreur : le coach n'a simplement rien mis
/// dedans. On rend un panier vide, à la version zéro — celle que `save` attend
/// pour créer la ligne. Cette règle n'a qu'une écriture, partagée par les deux
/// phases.
async fn charger_lignes<L: serde::de::DeserializeOwned>(
    team_id: &str,
    phase: &GamePhase,
    basket_repo: &dyn IPhaseBasketRepository,
) -> Result<(BasketVersion, Vec<L>), HydrationError> {
    let persiste = basket_repo
        .load(team_id, phase)
        .await
        .map_err(HydrationError::Repository)?;

    match persiste {
        Some(etat) => Ok((
            BasketVersion(etat.version),
            serde_json::from_value::<Vec<L>>(etat.state)
                .map_err(|e| HydrationError::CorruptedBasket(e.to_string()))?,
        )),
        None => Ok((BasketVersion(0), Vec::new())),
    }
}

fn charger_catalogue(
    team: &Team,
    catalog_port: &dyn IRosterCatalogPort,
) -> Result<RosterCatalog, HydrationError> {
    catalog_port
        .find_catalog(&team.roster_id.to_string())
        .map(to_domain_catalog)
        .ok_or(HydrationError::RosterNotFound)
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
                ma: p.ma,
                st: p.st,
                ag: p.ag,
                pa: p.pa,
                av: p.av,
                skills: p
                    .skills
                    .into_iter()
                    .map(|s| SkillBadge {
                        name: s.name,
                        category: s.category,
                    })
                    .collect(),
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
/// occupe toujours sa place dans l'effectif. `available_for_next_match` ne sert
/// ni au recrutement ni au calcul de valeur d'équipe — c'est le plancher des
/// renvois qui le lit.
///
/// L'effectif est rapporté **entier**. Un identifiant illisible fait échouer
/// l'hydratation plutôt que de sauter le membre : un effectif amputé compterait
/// un éligible de moins, et le plancher laisserait passer un renvoi qu'il doit
/// refuser.
fn to_domain_squad(membres: Vec<SquadMemberDto>) -> Result<Squad, HydrationError> {
    let mut members = Vec::with_capacity(membres.len());
    for m in membres {
        members.push(Player {
            player_id: PlayerId::try_new(&m.player_id)
                .map_err(|e| HydrationError::CorruptedSquad(format!("{} : {e}", m.player_id)))?,
            roster_line: RosterLineId(m.roster_line_id),
            personal_name: m.personal_name,
            position_name: m.position_name,
            spp: m.spp,
            value_kpo: Kpo(m.value_kpo),
            available_for_next_match: m.available_for_next_match,
        });
    }
    Ok(Squad { members })
}

fn owned_staff_of(team: &Team) -> OwnedStaff {
    OwnedStaff {
        rerolls: team.rerolls.0 as u32,
        apothecaries: team.apothecaries.0 as u32,
        assistants: team.assistants.0 as u32,
        cheerleaders: team.cheerleaders.0 as u32,
    }
}
