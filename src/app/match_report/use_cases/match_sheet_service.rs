//! La feuille de match imprimée (carte 608) — avant qu'aucun rapport n'existe.
//!
//! Pour chaque équipe : identité, VE courante (R4), fans, joueurs alignables
//! journaliers compris (R6), barème SPP de son roster. Le statut financier
//! — favori, ou outsider et sa petite monnaie — sort des **fonctions pures du
//! domaine** (`underdog`, carte 602), celles-là mêmes que l'agrégat applique
//! (R10, R11). Les rappels viennent du corpus (R9).

use crate::app::match_report::domain::underdog::{self, Side};
use crate::app::match_report::domain::value_objects::TeamValue;
use crate::app::match_report::ports::{
    AlignablePlayerDto, ICompetitionDataPort, IPlayerDataPort, IReferenceCatalogPort,
    ITeamDataPort, MatchRemindersDto, RoundContextDto, SppScaleDto,
};
use crate::app::match_report::use_cases::inducements_print_service::PrintError;

pub struct SheetTeam {
    pub name: String,
    pub roster_name: String,
    pub coach_name: String,
    pub team_value_kpo: u32,
    pub dedicated_fans: u32,
    pub players: Vec<AlignablePlayerDto>,
    pub spp: SppScaleDto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoneyStatus {
    TopDog {
        own_tv: u32,
        other_tv: u32,
    },
    /// L'écart de VE ; ce que le favori dépensera n'est pas encore connu (R11).
    Underdog {
        petty_cash_kpo: u32,
    },
    /// Pas d'adversaire, ou une VE illisible : la ligne se remplit à la main.
    Unknown,
}

pub struct MatchSheet {
    pub own: SheetTeam,
    pub other: Option<SheetTeam>,
    pub own_money: MoneyStatus,
    pub other_money: MoneyStatus,
    pub round: Option<RoundContextDto>,
    pub reminders: MatchRemindersDto,
}

/// Les deux équipes d'une feuille, et qui reçoit (R10).
pub struct SheetRequest<'a> {
    pub own_id: &'a str,
    pub other_id: Option<&'a str>,
    pub own_is_home: bool,
    pub round_id: Option<&'a str>,
}

pub struct SheetPorts<'a> {
    pub team_data: &'a dyn ITeamDataPort,
    pub player_data: &'a dyn IPlayerDataPort,
    pub competition_data: &'a dyn ICompetitionDataPort,
    pub catalog: &'a dyn IReferenceCatalogPort,
}

// arch:no-instrument — service de lecture : assemble la feuille de match imprimée, sans intention métier
pub async fn build(
    ports: &SheetPorts<'_>,
    req: &SheetRequest<'_>,
) -> Result<MatchSheet, PrintError> {
    let own = sheet_team(ports, req.own_id).await?;
    let other = match req.other_id {
        Some(id) => Some(sheet_team(ports, id).await?),
        None => None,
    };
    let (own_money, other_money) = match &other {
        Some(o) => money_status(own.team_value_kpo, o.team_value_kpo, req.own_is_home),
        None => (MoneyStatus::Unknown, MoneyStatus::Unknown),
    };
    Ok(MatchSheet {
        own,
        other,
        own_money,
        other_money,
        round: round_context(ports, req).await,
        reminders: ports.catalog.find_match_reminders(),
    })
}

async fn sheet_team(ports: &SheetPorts<'_>, team_id: &str) -> Result<SheetTeam, PrintError> {
    let info = ports
        .team_data
        .find_team_info(team_id)
        .await
        .ok_or(PrintError::TeamNotFound)?;
    Ok(SheetTeam {
        team_value_kpo: ports.team_data.find_team_value(team_id).await.unwrap_or(0),
        players: ports.player_data.list_alignable_players(team_id).await,
        spp: ports.catalog.find_spp_scale(&info.roster_id),
        name: info.team_name,
        roster_name: info.roster_name,
        coach_name: info.coach_name,
        dedicated_fans: info.dedicated_fans,
    })
}

/// La journée n'est retenue que si elle appartient à la saison de mon équipe :
/// une valeur forgée dans l'URL est ignorée, la feuille imprime alors des
/// lignes à remplir.
async fn round_context(ports: &SheetPorts<'_>, req: &SheetRequest<'_>) -> Option<RoundContextDto> {
    let round_id = req.round_id?;
    let season = ports.team_data.find_team_season(req.own_id).await?;
    ports
        .competition_data
        .find_round_context(&season, round_id)
        .await
}

/// Le statut de chaque camp, par les fonctions du domaine.
pub fn money_status(own_tv: u32, other_tv: u32, own_is_home: bool) -> (MoneyStatus, MoneyStatus) {
    let (Ok(own), Ok(other)) = (TeamValue::try_new(own_tv), TeamValue::try_new(other_tv)) else {
        return (MoneyStatus::Unknown, MoneyStatus::Unknown);
    };
    let (home, away) = if own_is_home {
        (own, other)
    } else {
        (other, own)
    };
    let own_is_top = (underdog::top_dog(home, away) == Side::Home) == own_is_home;
    let gap = underdog::tv_gap(home, away);
    let top = |a, b| MoneyStatus::TopDog {
        own_tv: a,
        other_tv: b,
    };
    let under = MoneyStatus::Underdog {
        petty_cash_kpo: gap,
    };
    if own_is_top {
        (top(own_tv, other_tv), under)
    } else {
        (under, top(other_tv, own_tv))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_ve_la_plus_forte_est_favorite_et_l_autre_recoit_l_ecart() {
        let (moi, autre) = money_status(1440, 1200, true);
        assert_eq!(
            moi,
            MoneyStatus::TopDog {
                own_tv: 1440,
                other_tv: 1200
            }
        );
        assert_eq!(
            autre,
            MoneyStatus::Underdog {
                petty_cash_kpo: 240
            }
        );

        let (moi, autre) = money_status(1100, 1300, true);
        assert_eq!(
            moi,
            MoneyStatus::Underdog {
                petty_cash_kpo: 200
            }
        );
        assert_eq!(
            autre,
            MoneyStatus::TopDog {
                own_tv: 1300,
                other_tv: 1100
            }
        );
    }

    /// R10 : à égalité, l'équipe à domicile est favorite.
    #[test]
    fn a_egalite_l_equipe_a_domicile_est_favorite() {
        let (moi, _) = money_status(1200, 1200, true);
        assert!(matches!(moi, MoneyStatus::TopDog { .. }));
        let (moi, autre) = money_status(1200, 1200, false);
        assert_eq!(moi, MoneyStatus::Underdog { petty_cash_kpo: 0 });
        assert!(matches!(autre, MoneyStatus::TopDog { .. }));
    }

    #[test]
    fn une_ve_hors_bornes_rend_le_statut_inconnu() {
        assert_eq!(money_status(9999, 1000, true).0, MoneyStatus::Unknown);
    }
}
