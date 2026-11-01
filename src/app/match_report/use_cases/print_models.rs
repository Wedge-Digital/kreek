//! Ce que les impressions de `match_report` donnent à leurs vues (épic E16).
//!
//! **Les DTO de port s'arrêtent ici** (règle « Domain services pour données
//! inter-BCs ») : `match_sheet_service` et `inducements_print_service` les
//! convertissent en ces types, et les widgets comme les gabarits ne voient
//! qu'eux. Une évolution du port touche ces conversions, pas l'affichage.

use crate::app::match_report::ports::{
    AlignablePlayerDto, InducementLabelDto, InducementSpecDto, MatchRemindersDto, RoundContextDto,
    SppScaleDto, StarPlayerDto, TierRulesDto,
};

/// Un joueur aligné sur la feuille de match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetPlayer {
    pub jersey: Option<i16>,
    pub name: String,
    pub is_journeyman: bool,
}

impl From<AlignablePlayerDto> for SheetPlayer {
    fn from(dto: AlignablePlayerDto) -> Self {
        Self {
            jersey: dto.jersey,
            name: dto.name,
            is_journeyman: dto.is_journeyman,
        }
    }
}

/// Le barème SPP du roster d'une équipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SppScale {
    pub touchdown: u8,
    pub casualty: u8,
    pub interception: u8,
    pub pass: u8,
    pub mvp: u8,
}

impl From<SppScaleDto> for SppScale {
    fn from(dto: SppScaleDto) -> Self {
        Self {
            touchdown: dto.touchdown,
            casualty: dto.casualty,
            interception: dto.interception,
            pass: dto.pass,
            mvp: dto.mvp,
        }
    }
}

/// Les rappels du corpus — absents s'il ne les fournit pas (R9).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatchReminders {
    pub mvp: Option<String>,
    pub fans: Option<String>,
    pub weather: Vec<String>,
}

impl From<MatchRemindersDto> for MatchReminders {
    fn from(dto: MatchRemindersDto) -> Self {
        Self {
            mvp: dto.mvp,
            fans: dto.fans,
            weather: dto.weather,
        }
    }
}

/// La compétition, la saison et la journée imprimées en tête de feuille.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundContext {
    pub competition_name: String,
    pub season_name: String,
    pub round_name: String,
}

impl From<RoundContextDto> for RoundContext {
    fn from(dto: RoundContextDto) -> Self {
        Self {
            competition_name: dto.competition_name,
            season_name: dto.season_name,
            round_name: dto.round_name,
        }
    }
}

/// Ce que le palier d'une équipe lui permet d'acheter, au prix de son roster.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TierOffer {
    pub allowed_inducements: Vec<OfferedInducement>,
    pub allowed_star_players: Vec<OfferedInducement>,
}

impl From<TierRulesDto> for TierOffer {
    fn from(dto: TierRulesDto) -> Self {
        let offers = |specs: Vec<InducementSpecDto>| specs.into_iter().map(Into::into).collect();
        Self {
            allowed_inducements: offers(dto.allowed_inducements),
            allowed_star_players: offers(dto.allowed_star_players),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferedInducement {
    pub uid: String,
    pub max_qty: u8,
    pub unit_cost: u32,
}

impl From<InducementSpecDto> for OfferedInducement {
    fn from(dto: InducementSpecDto) -> Self {
        Self {
            uid: dto.uid,
            max_qty: dto.max_qty,
            unit_cost: dto.unit_cost,
        }
    }
}

/// Le libellé d'un coup de pouce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InducementLabel {
    pub uid: String,
    pub name: String,
    pub description: String,
}

impl From<InducementLabelDto> for InducementLabel {
    fn from(dto: InducementLabelDto) -> Self {
        Self {
            uid: dto.uid,
            name: dto.name,
            description: dto.description,
        }
    }
}

/// Un star player, compétences résolues en noms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarPlayer {
    pub uid: String,
    pub name: String,
    pub player_type: String,
    pub cost: u32,
    pub ma: u8,
    pub st: u8,
    pub ag: String,
    pub pa: String,
    pub av: String,
    pub skills: Vec<String>,
    pub ability_name: String,
    pub ability_description: String,
}

impl From<StarPlayerDto> for StarPlayer {
    fn from(dto: StarPlayerDto) -> Self {
        Self {
            uid: dto.uid,
            name: dto.name,
            player_type: dto.player_type,
            cost: dto.cost,
            ma: dto.ma,
            st: dto.st,
            ag: dto.ag,
            pa: dto.pa,
            av: dto.av,
            skills: dto.skills,
            ability_name: dto.ability_name,
            ability_description: dto.ability_description,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_palier_garde_ses_prix_et_ses_star_players() {
        let spec = |uid: &str, cost| InducementSpecDto {
            uid: uid.into(),
            max_qty: 2,
            unit_cost: cost,
        };
        let offer = TierOffer::from(TierRulesDto {
            allowed_inducements: vec![spec("BRIBE", 100)],
            allowed_star_players: vec![spec("STAR", 0)],
        });
        assert_eq!(offer.allowed_inducements[0].unit_cost, 100);
        assert_eq!(offer.allowed_star_players[0].uid, "STAR");
    }
}
