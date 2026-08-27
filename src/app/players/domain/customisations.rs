//! Ce qui reste vrai des customisations d'un joueur, lu depuis son flux.
//!
//! **Trois consommateurs, un seul filtre.** Le rejeu du use case de retrait, la
//! liste « Customisations appliquées » du panneau, et le journal d'évolution
//! posent tous la même question : quelles customisations tiennent encore ?
//!
//! L'event sourcing ne réécrit jamais l'histoire — la customisation retirée et
//! son retrait restent tous deux dans l'event store. C'est ici qu'ils cessent
//! d'exister pour tout ce qui regarde le joueur.

use crate::app::players::domain::events::{PlayerDomainEvent, UndoEffect};
use crate::app::players::domain::match_impact::StatKind;
use crate::app::players::domain::player::Player;
use crate::app::players::domain::value_objects::{
    CustomisationId, KpoDelta, SkillId, SkillName, SppAmount,
};

/// Une customisation encore appliquée, dans les termes du domaine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomisationAppliquee {
    pub id: CustomisationId,
    pub famille: FamilleCustomisation,
    pub auteur: String,
    /// `None` quand la customisation est retirable. `Some` porte la raison du
    /// blocage — celle que l'écran affiche sous la croix grisée.
    pub blocage: Option<MotifBlocage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FamilleCustomisation {
    Skill { skill_id: SkillId, nom: SkillName },
    Stat { stat: StatKind, offset: i8 }, // arch:ok offset brut, celui de l'événement
    Value { delta: KpoDelta },
    Spp { amount: SppAmount },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotifBlocage {
    /// Le commissaire a offert `offerts` SPP, le coach en a dépensé assez pour
    /// qu'il n'en reste que `restants`. Retirer l'offre mettrait le joueur en
    /// négatif.
    SppDepenses { restants: u32, offerts: u32 },
}

/// Le flux **effectif** : privé des customisations retirées, de leurs retraits,
/// et de `exclure` si elle est fournie.
///
/// C'est ce que le use case de retrait rejoue pour obtenir la valeur absolue à
/// poser. **Retirer le seul événement visé ne suffit pas** : un retrait de prix
/// antérieur a posé une valeur *absolue*, calculée dans un monde où la
/// customisation qu'on retire aujourd'hui existait encore. La rejouer telle
/// quelle rendrait la valeur courante, et le retrait serait un no-op silencieux.
///
/// ```text
/// créé(100) · C1 prix +50 → 150 · C2 prix +30 → 180 · retrait C2 pose 150
/// retrait de C1, flux privé du seul C1 : [créé(100), C2(+30), retraitC2(150)] → 150 ✗
/// retrait de C1, flux effectif        : [créé(100)]                           → 100 ✓
/// ```
pub fn flux_effectif<'a>(
    events: &'a [PlayerDomainEvent],
    exclure: Option<&CustomisationId>,
) -> Vec<&'a PlayerDomainEvent> {
    let retirees = identifiants_retires(events);
    events
        .iter()
        .filter(|e| !est_un_retrait(e))
        .filter(|e| match identifiant_de_customisation(e) {
            None => true,
            Some(id) => !retirees.contains(&id) && Some(id) != exclure,
        })
        .collect()
}

/// Les customisations qui tiennent encore, dans l'ordre où elles ont été posées.
///
/// `player` sert au seul blocage existant — la réserve de SPP. Les trois autres
/// familles ne se refusent jamais : retirer une compétence, une caractéristique
/// ou un ajustement de prix ne peut mettre le joueur dans aucun état impossible.
pub fn appliquees(events: &[PlayerDomainEvent], player: &Player) -> Vec<CustomisationAppliquee> {
    flux_effectif(events, None)
        .into_iter()
        .filter_map(|e| decrire(e, player))
        .collect()
}

/// Ce qu'il faut défaire pour retirer cette customisation.
///
/// `value_after` vient du rejeu — le domaine ne sait pas le calculer seul, un
/// événement ne connaissant pas le flux qui l'entoure.
pub fn undo_pour(
    customisation: &PlayerDomainEvent,
    value_after: crate::app::players::domain::player::ValueKpo,
) -> Option<UndoEffect> {
    match customisation {
        PlayerDomainEvent::PlayerSkillCustomised { skill_id, .. } => Some(UndoEffect::Skill {
            skill_id: skill_id.clone(),
        }),
        PlayerDomainEvent::PlayerStatCustomised { stat, offset, .. } => Some(UndoEffect::Stat {
            stat: *stat,
            offset: *offset,
        }),
        PlayerDomainEvent::PlayerValueCustomised { .. } => Some(UndoEffect::Value { value_after }),
        PlayerDomainEvent::PlayerSppCustomised { amount, .. } => {
            Some(UndoEffect::Spp { amount: *amount })
        }
        _ => None,
    }
}

/// L'événement de customisation portant cet identifiant, dans le flux effectif.
///
/// Passer par le flux effectif et non par le flux brut est ce qui refuse un
/// **second retrait** du même identifiant : la customisation n'y est plus.
pub fn trouver<'a>(
    events: &'a [PlayerDomainEvent],
    id: &CustomisationId,
) -> Option<&'a PlayerDomainEvent> {
    flux_effectif(events, None)
        .into_iter()
        .find(|e| identifiant_de_customisation(e) == Some(id))
}

// ── Rouages ───────────────────────────────────────────────────────────────────

fn identifiants_retires(events: &[PlayerDomainEvent]) -> Vec<&CustomisationId> {
    events
        .iter()
        .filter_map(|e| match e {
            PlayerDomainEvent::PlayerCustomisationReverted {
                customisation_id, ..
            } => Some(customisation_id),
            _ => None,
        })
        .collect()
}

fn est_un_retrait(event: &PlayerDomainEvent) -> bool {
    matches!(event, PlayerDomainEvent::PlayerCustomisationReverted { .. })
}

/// `None` pour tout ce qui n'est pas une customisation — la très grande
/// majorité du flux.
fn identifiant_de_customisation(event: &PlayerDomainEvent) -> Option<&CustomisationId> {
    match event {
        PlayerDomainEvent::PlayerSkillCustomised {
            customisation_id, ..
        }
        | PlayerDomainEvent::PlayerStatCustomised {
            customisation_id, ..
        }
        | PlayerDomainEvent::PlayerValueCustomised {
            customisation_id, ..
        }
        | PlayerDomainEvent::PlayerSppCustomised {
            customisation_id, ..
        } => Some(customisation_id),
        _ => None,
    }
}

fn decrire(event: &PlayerDomainEvent, player: &Player) -> Option<CustomisationAppliquee> {
    let (id, famille, auteur) = match event {
        PlayerDomainEvent::PlayerSkillCustomised {
            customisation_id,
            skill_id,
            skill_name,
            author,
            ..
        } => (
            customisation_id,
            FamilleCustomisation::Skill {
                skill_id: skill_id.clone(),
                nom: skill_name.clone(),
            },
            author,
        ),
        PlayerDomainEvent::PlayerStatCustomised {
            customisation_id,
            stat,
            offset,
            author,
            ..
        } => (
            customisation_id,
            FamilleCustomisation::Stat {
                stat: *stat,
                offset: *offset,
            },
            author,
        ),
        PlayerDomainEvent::PlayerValueCustomised {
            customisation_id,
            delta,
            author,
            ..
        } => (
            customisation_id,
            FamilleCustomisation::Value { delta: *delta },
            author,
        ),
        PlayerDomainEvent::PlayerSppCustomised {
            customisation_id,
            amount,
            author,
            ..
        } => (
            customisation_id,
            FamilleCustomisation::Spp { amount: *amount },
            author,
        ),
        _ => return None,
    };
    Some(CustomisationAppliquee {
        id: id.clone(),
        blocage: blocage_pour(&famille, player),
        famille,
        auteur: auteur.clone(),
    })
}

fn blocage_pour(famille: &FamilleCustomisation, player: &Player) -> Option<MotifBlocage> {
    let FamilleCustomisation::Spp { amount } = famille else {
        return None;
    };
    let offerts = amount.into_inner() as u32;
    let restants = player.spp_remaining();
    (restants < offerts).then_some(MotifBlocage::SppDepenses { restants, offerts })
}
