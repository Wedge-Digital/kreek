//! Une saison est-elle ouverte à la création d'équipe ?
//!
//! La question a une réponse à quatre valeurs, et **chacune doit rester
//! distinguable**. C'est la leçon de la carte 407 : la porte était gardée par
//! un `Option` où « saison inconnue », « saison sans règles » et « saison pas
//! encore prête » se confondaient en un seul `None`. Le troisième cas, celui
//! qui comptait, n'était même pas exprimable.
//!
//! La décision vit ici plutôt que dans l'adapter parce qu'ici elle se teste :
//! aucune base, aucun HTTP, aucun `async`.

use crate::app::team_creation::domain::creation_rules::CreationRules;
use crate::app::team_creation::ports::{ICompetitionCreationRulesPort, SeasonCreationData};

/// Ce que voit le coach quand la saison n'est pas encore ouverte.
///
/// Un seul libellé pour les quatre points d'entrée : le statut brut reste au
/// journal, il ne dit rien au coach et exposerait le vocabulaire interne du BC
/// `competitions`.
pub const SAISON_PAS_OUVERTE: &str = "Cette compétition n'est pas encore ouverte aux inscriptions.";

/// Ce que le coach a le droit de faire de cette saison.
pub enum AccesCreation {
    /// Saison prête, règles utilisables.
    Ouverte(CreationRules),
    /// Saison réelle, mais encore en cours de configuration. `statut` ne sert
    /// qu'au journal — il ne s'affiche pas au coach.
    PasEncorePrete { statut: String },
    /// Saison prête, mais dont les règles ne permettent de bâtir aucune équipe.
    SansRegles,
    /// Saison inconnue, ou identifiant illisible.
    Introuvable,
}

// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub fn acces_creation(data: Option<SeasonCreationData>) -> AccesCreation {
    let Some(data) = data else {
        return AccesCreation::Introuvable;
    };
    if !data.prete {
        return AccesCreation::PasEncorePrete {
            statut: data.statut,
        };
    }
    match data.rules {
        Some(rules) if !rules.tiers.is_empty() => AccesCreation::Ouverte(rules),
        _ => AccesCreation::SansRegles,
    }
}

/// Garde des points de **soumission** — ceux qui n'ont pas besoin des règles,
/// seulement du droit d'entrer.
///
/// Elle existe parce qu'une équipe peut être commencée avant la finalisation et
/// soumise après : la garde d'ouverture seule laisserait passer ces équipes-là.
/// Elle journalise elle-même le motif, pour que les trois appelants ne puissent
/// pas oublier de le faire.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn verifier_saison_ouverte(
    port: &dyn ICompetitionCreationRulesPort,
    season_id: &str,
) -> Result<(), &'static str> {
    match acces_creation(port.find_season_creation_data(season_id).await) {
        AccesCreation::Ouverte(_) => Ok(()),
        AccesCreation::PasEncorePrete { statut } => {
            tracing::info!(
                season_id = %season_id,
                statut = %statut,
                "soumission refusée : la saison n'est pas ouverte aux inscriptions"
            );
            Err(SAISON_PAS_OUVERTE)
        }
        AccesCreation::SansRegles => {
            tracing::warn!(
                season_id = %season_id,
                "soumission refusée : saison prête mais sans règles de création"
            );
            Err("La compétition sélectionnée n'a pas de règles de création.")
        }
        AccesCreation::Introuvable => {
            tracing::warn!(season_id = %season_id, "soumission refusée : saison introuvable");
            Err("Saison introuvable — rechargez la page.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::tier::{CreationBudget, StartingXp, TierName};
    use crate::app::team_creation::domain::creation_rules::CreationTier;

    fn tier() -> CreationTier {
        CreationTier {
            name: TierName::try_new("Tier 1").unwrap(),
            budget: CreationBudget(1_000_000),
            start_xp: StartingXp::try_new(0).unwrap(),
            rosters: vec![],
        }
    }

    fn donnee(prete: bool, statut: &str, tiers: Vec<CreationTier>) -> Option<SeasonCreationData> {
        Some(SeasonCreationData {
            prete,
            statut: statut.to_string(),
            rules: Some(CreationRules { tiers }),
        })
    }

    #[test]
    fn une_saison_prete_avec_des_regles_est_ouverte() {
        assert!(matches!(
            acces_creation(donnee(true, "ready", vec![tier()])),
            AccesCreation::Ouverte(_)
        ));
    }

    /// Le cas de la carte 407 : la saison existe, ses règles sont déjà posées —
    /// et c'est justement ce qui la rendait joignable trois étapes trop tôt.
    #[test]
    fn une_saison_non_prete_est_refusee_meme_avec_des_regles() {
        let acces = acces_creation(donnee(false, "structure_selected", vec![tier()]));
        match acces {
            AccesCreation::PasEncorePrete { statut } => assert_eq!(statut, "structure_selected"),
            _ => panic!("une saison non prête doit être refusée"),
        }
    }

    #[test]
    fn le_statut_brut_voyage_jusqu_au_journal() {
        let acces = acces_creation(donnee(false, "rules_selected", vec![]));
        match acces {
            AccesCreation::PasEncorePrete { statut } => assert_eq!(statut, "rules_selected"),
            _ => panic!("le motif du refus doit rester lisible"),
        }
    }

    /// « Pas encore prête » prime sur « sans règles » : sans cet ordre, une
    /// saison en tout début de configuration se signalerait comme mal réglée
    /// alors qu'elle est simplement inachevée.
    #[test]
    fn pas_encore_prete_prime_sur_sans_regles() {
        assert!(matches!(
            acces_creation(donnee(false, "rules_selected", vec![])),
            AccesCreation::PasEncorePrete { .. }
        ));
    }

    #[test]
    fn une_saison_prete_sans_tier_n_a_pas_de_regles() {
        assert!(matches!(
            acces_creation(donnee(true, "ready", vec![])),
            AccesCreation::SansRegles
        ));
    }

    #[test]
    fn une_saison_prete_dont_les_regles_manquent_n_a_pas_de_regles() {
        let data = Some(SeasonCreationData {
            prete: true,
            statut: "ready".to_string(),
            rules: None,
        });
        assert!(matches!(acces_creation(data), AccesCreation::SansRegles));
    }

    #[test]
    fn une_saison_absente_est_introuvable() {
        assert!(matches!(acces_creation(None), AccesCreation::Introuvable));
    }
}
