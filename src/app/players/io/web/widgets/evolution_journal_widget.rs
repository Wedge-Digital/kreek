use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::match_impact::MatchContext;
use crate::app::players::domain::match_impact::StatKind;
use crate::app::players::domain::player::{AcquisitionMode, PlayerId, ValueKpo};
use crate::app::players::domain::value_objects::SppCost;
use crate::app::players::io::web::player_loader::charger_joueur;
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

// ── View models ───────────────────────────────────────────────────────────────

pub struct EvolutionLogRowVm {
    pub label: String,
    pub mode_label: &'static str,
    /// Classe de la pastille : la customisation ne se confond pas visuellement
    /// avec une progression normale.
    pub mode_css: &'static str,
    /// Vides — et non « 0 » — là où la notion n'existe pas : une compétence
    /// customisée ne coûte aucun SPP et n'ajoute aucune valeur. Un zéro
    /// laisserait croire à un calcul là où il n'y en a pas.
    pub cost: String,
    pub value: String,
    /// Dynamique depuis la customisation : l'origine nomme le commissaire, ce
    /// qui est toute la raison d'être de la traçabilité posée en phase 1.
    pub origin: String,
}

pub struct EvolutionJournalVm {
    pub player_name: String,
    pub spp_reserve: u32,
    pub evolution_log: Vec<EvolutionLogRowVm>,
    pub can_spend: bool,
    /// Une saisie de customisation périmée vient d'être supprimée. Le dire
    /// discrètement plutôt que de laisser croire qu'elle n'a jamais existé.
    pub abandoned: bool,
    pub spp_spending_widget_url: String,
}

/// Reconstruit le journal directement depuis les events bruts (même principe
/// que `match_history_service::build_match_history`) — c'est le seul moyen de
/// distinguer un bonus de création (`InitialSkillEarned`) d'un achat via les
/// SPP (`PlayerSkillPurchased`) : une fois repliés dans `player.acquired_skills`,
/// les deux events produisent une structure identique, sans champ d'origine.
fn evolution_log_vm(events: &[PlayerDomainEvent]) -> Vec<EvolutionLogRowVm> {
    events.iter().filter_map(evolution_log_row).collect()
}

fn evolution_log_row(event: &PlayerDomainEvent) -> Option<EvolutionLogRowVm> {
    match event {
        PlayerDomainEvent::InitialSkillEarned {
            skill_name,
            mode,
            spp_cost,
            value_delta,
            ..
        } => Some(skill_row(
            skill_name.to_string(),
            *mode,
            *spp_cost,
            *value_delta,
            "Compétence initiale bonus",
        )),
        PlayerDomainEvent::PlayerSkillPurchased {
            skill_name,
            mode,
            spp_cost,
            value_delta,
            ..
        } => Some(skill_row(
            skill_name.to_string(),
            *mode,
            *spp_cost,
            *value_delta,
            "Progression normale",
        )),
        PlayerDomainEvent::PlayerStatIncreased {
            stat,
            spp_cost,
            value_delta,
            ..
        } => Some(EvolutionLogRowVm {
            label: format!("Caractéristique : {}", stat_label(*stat)),
            mode_label: "Choisie",
            mode_css: "mode-chip-chosen",
            cost: format!("{} SPP", spp_cost.into_inner()),
            value: format!("+{} kPo", value_delta.0),
            origin: "Progression normale".to_string(),
        }),
        PlayerDomainEvent::PlayerSkillCustomised {
            skill_name, author, ..
        } => Some(ligne_customisee(skill_name.to_string(), author)),
        PlayerDomainEvent::PlayerHatredGained {
            skill_name,
            context,
            ..
        } => Some(ligne_de_haine(skill_name.to_string(), context)),
        PlayerDomainEvent::PlayerStatCustomised {
            stat,
            offset,
            author,
            ..
        } => Some(EvolutionLogRowVm {
            label: format!(
                "Caractéristique : {} {}",
                stat_label(*stat),
                signe(*offset as i32)
            ),
            ..ligne_customisee(String::new(), author)
        }),
        PlayerDomainEvent::PlayerValueCustomised { delta, author, .. } => Some(EvolutionLogRowVm {
            label: "Prix ajusté".to_string(),
            value: format!("{} kPo", signe(delta.into_inner())),
            ..ligne_customisee(String::new(), author)
        }),
        PlayerDomainEvent::PlayerSppCustomised { amount, author, .. } => Some(EvolutionLogRowVm {
            label: "SPP crédités".to_string(),
            cost: format!("+{} SPP", amount.into_inner()),
            ..ligne_customisee(String::new(), author)
        }),
        _ => None,
    }
}

/// Le socle commun des quatre familles : colonnes vides, mode « Customisation »,
/// et l'auteur nommé. Chaque famille ne surcharge que ce qui la distingue.
/// Un trait gagné en encaissant un coup.
///
/// Coût et valeur restent **vides**, comme pour une customisation : ni « 0 SPP »
/// ni « +0 kPo ». Le modèle ne porte pas ces champs — ils n'existent pas, ils ne
/// valent pas zéro — et l'affichage doit dire la même chose : un zéro affiché
/// invite à croire qu'un calcul a eu lieu.
fn ligne_de_haine(label: String, context: &MatchContext) -> EvolutionLogRowVm {
    EvolutionLogRowVm {
        label,
        mode_label: "Blessure",
        mode_css: "mode-chip-chosen",
        cost: String::new(),
        value: String::new(),
        origin: format!("Blessé contre {}", context.opponent_team_name),
    }
}

fn ligne_customisee(label: String, author: &str) -> EvolutionLogRowVm {
    EvolutionLogRowVm {
        label,
        mode_label: "🛠️ Customisation",
        mode_css: "mode-chip-custom",
        cost: String::new(),
        value: String::new(),
        origin: format!("Customisation par {author}"),
    }
}

/// Signe explicite dans les deux sens : « +10 » se lit comme un ajout, « 10 »
/// se lirait comme une valeur.
///
/// Prend un `i32` : `KpoDelta` en est un, et le réduire en `i8` ferait d'un
/// −300 un +212 sans que rien ne proteste.
fn signe(v: i32) -> String {
    match v >= 0 {
        true => format!("+{v}"),
        false => v.to_string(),
    }
}

fn skill_row(
    skill_name: String,
    mode: AcquisitionMode,
    spp_cost: SppCost,
    value_delta: ValueKpo,
    origin: &'static str,
) -> EvolutionLogRowVm {
    let mode_label = match mode {
        AcquisitionMode::Chosen => "Choisie",
        AcquisitionMode::Random => "Aléatoire",
        AcquisitionMode::Customised => "Customisation",
        AcquisitionMode::Injury => "Blessure",
    };
    let mode_css = match mode {
        AcquisitionMode::Random => "mode-chip-random",
        _ => "mode-chip-chosen",
    };
    EvolutionLogRowVm {
        label: skill_name,
        mode_label,
        mode_css,
        cost: format!("{} SPP", spp_cost.into_inner()),
        value: format!("+{} kPo", value_delta.0),
        origin: origin.to_string(),
    }
}

fn stat_label(stat: StatKind) -> &'static str {
    match stat {
        StatKind::Ma => "Mouvement",
        StatKind::St => "Force",
        StatKind::Ag => "Agilité",
        StatKind::Pa => "Passe",
        StatKind::Av => "Armure",
    }
}

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "evolution-journal-widget.html")]
pub struct EvolutionJournalTemplate {
    pub vm: EvolutionJournalVm,
}

impl IntoResponse for EvolutionJournalTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("evolution_journal_widget render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

#[derive(Deserialize, Default)]
pub struct EvolutionJournalParams {
    #[serde(default)]
    pub can_spend: bool,
    #[serde(default)]
    pub abandoned: bool,
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn evolution_journal_widget(
    Path((space_id, player_id)): Path<(String, String)>,
    Query(params): Query<EvolutionJournalParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let player = match charger_joueur(&state, &player_id).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let events = match state
        .players
        .repository
        .find_events_by_id(&PlayerId(player_id.clone()))
        .await
    {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("evolution_journal_widget find_events_by_id {player_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // can_spend est fourni par la page appelante (player_detail_controller, qui
    // a déjà vérifié phase + autorisation) — aucun enjeu de sécurité à le faire
    // transiter tel quel ici : ce widget est en lecture seule, le vrai gardien
    // reste spp_spending_widget (qui revérifie tout indépendamment).
    let vm = EvolutionJournalVm {
        player_name: player.position_name.to_string(),
        spp_reserve: player.spp_remaining(),
        evolution_log: evolution_log_vm(&events),
        can_spend: params.can_spend,
        abandoned: params.abandoned,
        spp_spending_widget_url: AppRoutes::default()
            .players
            .spp_spending_widget(&space_id, &player_id),
    };

    EvolutionJournalTemplate { vm }.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::player::TeamId;
    use crate::app::players::domain::value_objects::{CustomisationId, SkillId, SkillName};
    use crate::app::shared_kernel::identity::ids::SpaceId;

    fn initial_skill_event() -> PlayerDomainEvent {
        PlayerDomainEvent::InitialSkillEarned {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            skill_id: SkillId::try_new("block".to_string()).unwrap(),
            skill_name: SkillName::try_new("Bloc".to_string()).unwrap(),
            category_css: "type-general".into(),
            mode: AcquisitionMode::Chosen,
            spp_cost: SppCost::try_new(0).unwrap(),
            is_primary: true,
            is_elite: false,
            value_delta: ValueKpo(0),
        }
    }

    fn purchased_skill_event() -> PlayerDomainEvent {
        PlayerDomainEvent::PlayerSkillPurchased {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            skill_id: SkillId::try_new("dodge".to_string()).unwrap(),
            skill_name: SkillName::try_new("Esquive".to_string()).unwrap(),
            category_css: "type-general".into(),
            mode: AcquisitionMode::Chosen,
            spp_cost: SppCost::try_new(6).unwrap(),
            value_delta: ValueKpo(20),
        }
    }

    fn stat_increased_event() -> PlayerDomainEvent {
        PlayerDomainEvent::PlayerStatIncreased {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            stat: StatKind::St,
            spp_cost: SppCost::try_new(14).unwrap(),
            value_delta: ValueKpo(30),
        }
    }

    #[test]
    fn initial_skill_earned_is_labeled_as_bonus_origin() {
        let rows = evolution_log_vm(&[initial_skill_event()]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].origin, "Compétence initiale bonus");
        assert_eq!(rows[0].label, "Bloc");
    }

    #[test]
    fn player_skill_purchased_is_labeled_as_normal_progression() {
        let rows = evolution_log_vm(&[purchased_skill_event()]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].origin, "Progression normale");
        assert_eq!(rows[0].label, "Esquive");
        assert_eq!(rows[0].value, "+20 kPo");
        assert_eq!(rows[0].cost, "6 SPP");
    }

    #[test]
    fn stat_increase_appears_in_the_journal_with_its_value() {
        let rows = evolution_log_vm(&[stat_increased_event()]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "Caractéristique : Force");
        assert_eq!(rows[0].origin, "Progression normale");
        assert_eq!(rows[0].value, "+30 kPo");
        assert_eq!(rows[0].cost, "14 SPP");
    }

    #[test]
    fn mixed_events_produce_one_row_each_in_order() {
        let rows = evolution_log_vm(&[
            initial_skill_event(),
            purchased_skill_event(),
            stat_increased_event(),
        ]);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].origin, "Compétence initiale bonus");
        assert_eq!(rows[1].origin, "Progression normale");
        assert_eq!(rows[2].label, "Caractéristique : Force");
    }

    // ── Customisations ────────────────────────────────────────────────────────

    fn ids() -> (PlayerId, TeamId, CustomisationId) {
        (
            PlayerId("p1".into()),
            TeamId("t1".into()),
            CustomisationId::try_new("c1".to_string()).unwrap(),
        )
    }

    /// Chaque famille produit sa ligne, et **nomme son commissaire** : c'est
    /// toute la raison d'être de la traçabilité posée en phase 1.
    #[test]
    fn chaque_famille_de_customisation_produit_sa_ligne_avec_son_auteur() {
        let (player_id, team_id, customisation_id) = ids();
        let lignes = evolution_log_vm(&[
            PlayerDomainEvent::PlayerSkillCustomised {
                player_id: player_id.clone(),
                team_id: team_id.clone(),
                customisation_id: customisation_id.clone(),
                skill_id: SkillId::try_new("BLOCK".to_string()).unwrap(),
                skill_name: SkillName::try_new("Bloc".to_string()).unwrap(),
                author: "Bagouze".into(),
            },
            PlayerDomainEvent::PlayerStatCustomised {
                player_id: player_id.clone(),
                team_id: team_id.clone(),
                customisation_id: customisation_id.clone(),
                stat: StatKind::Ag,
                offset: -1,
                author: "Bagouze".into(),
            },
            PlayerDomainEvent::PlayerValueCustomised {
                player_id: player_id.clone(),
                team_id: team_id.clone(),
                customisation_id: customisation_id.clone(),
                delta: crate::app::players::domain::value_objects::KpoDelta::try_new(-15).unwrap(),
                author: "Bagouze".into(),
            },
            PlayerDomainEvent::PlayerSppCustomised {
                player_id,
                team_id,
                customisation_id,
                amount: crate::app::players::domain::value_objects::SppAmount::try_new(5).unwrap(),
                author: "Bagouze".into(),
            },
        ]);

        assert_eq!(lignes.len(), 4);
        assert_eq!(lignes[0].label, "Bloc");
        assert_eq!(lignes[1].label, "Caractéristique : Agilité -1");
        assert_eq!(lignes[2].label, "Prix ajusté");
        assert_eq!(lignes[3].label, "SPP crédités");

        for ligne in &lignes {
            assert_eq!(ligne.origin, "Customisation par Bagouze");
            assert_eq!(ligne.mode_css, "mode-chip-custom");
        }
    }

    /// Une colonne vide, jamais un zéro : une compétence customisée ne coûte
    /// aucun SPP et n'ajoute aucune valeur. Un « 0 » laisserait croire à un
    /// calcul là où il n'y en a pas.
    #[test]
    fn les_colonnes_sans_notion_restent_vides() {
        let (player_id, team_id, customisation_id) = ids();
        let lignes = evolution_log_vm(&[PlayerDomainEvent::PlayerSkillCustomised {
            player_id,
            team_id,
            customisation_id,
            skill_id: SkillId::try_new("BLOCK".to_string()).unwrap(),
            skill_name: SkillName::try_new("Bloc".to_string()).unwrap(),
            author: "Bagouze".into(),
        }]);

        assert_eq!(lignes[0].cost, "");
        assert_eq!(lignes[0].value, "");
    }

    /// `KpoDelta` est un `i32`. Le réduire en `i8` pour formater son signe
    /// ferait d'un −300 un +212, sans que rien ne proteste.
    #[test]
    fn un_gros_ajustement_de_prix_ne_deborde_pas() {
        let (player_id, team_id, customisation_id) = ids();
        let lignes = evolution_log_vm(&[PlayerDomainEvent::PlayerValueCustomised {
            player_id,
            team_id,
            customisation_id,
            delta: crate::app::players::domain::value_objects::KpoDelta::try_new(-300).unwrap(),
            author: "Bagouze".into(),
        }]);

        assert_eq!(lignes[0].value, "-300 kPo");
    }

    #[test]
    fn unrelated_events_are_ignored() {
        let space_id = SpaceId::new();
        let unrelated = PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            space_id,
            position_name: crate::app::players::domain::value_objects::PositionNameVo::try_new(
                "Frappeur".to_string(),
            )
            .unwrap(),
            roster_line_id: crate::app::players::domain::value_objects::RosterLineId::try_new(
                "BLITZER".to_string(),
            )
            .unwrap(),
            jersey: None,
            base_skills: vec![],
            starting_spp: crate::app::players::domain::player::Spp(0),
            starting_value: ValueKpo(100),
        };
        let rows = evolution_log_vm(&[unrelated]);
        assert!(rows.is_empty());
    }
}
