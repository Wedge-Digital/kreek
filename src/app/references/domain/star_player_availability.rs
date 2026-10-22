//! Quels star players une équipe peut engager (R13, carte 603).
//!
//! La règle vivait en privé dans `build_star_player_items`, le sélecteur de
//! coups de pouce. L'impression des star players disponibles en a besoin :
//! elle rejoint le domaine, à côté de `est_disponible_pour`, **déplacée par
//! copier-coller**. Le sélecteur l'appelle ; ce qu'il montre et ce qui
//! s'imprime sortent du même endroit.
//!
//! Le serveur, lui, ne l'applique toujours pas à l'engagement
//! (`build_star_player_spec`) : hors périmètre de l'épic E16.

use super::models::StarPlayer;

/// Ce star player est-il disponible pour ce roster ?
///
/// Sans liste de rosters autorisés, il joue pour tous. Sans roster connu — le
/// sélecteur ouvert hors d'une équipe —, rien n'est filtré.
pub fn star_player_available_for(star: &StarPlayer, roster_id: &str) -> bool {
    roster_id.is_empty()
        || star.available_for_rosters.is_empty()
        || star.available_for_rosters.iter().any(|r| r == roster_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn star(rosters: &[&str]) -> StarPlayer {
        StarPlayer {
            uid: "DEMO_STAR".into(),
            name: "Étoile".into(),
            cost: 100,
            ma: 6,
            st: 3,
            ag: "3+".into(),
            pa: "4+".into(),
            av: "9+".into(),
            player_type: "Coureur".into(),
            skills: vec![],
            special_ability_name: String::new(),
            special_ability_description: String::new(),
            plays_for: vec![],
            available_for_rosters: rosters.iter().map(|r| r.to_string()).collect(),
        }
    }

    #[test]
    fn sans_liste_il_joue_pour_tous() {
        assert!(star_player_available_for(&star(&[]), "DEMO_GRANIT"));
    }

    #[test]
    fn un_roster_de_la_liste_peut_l_engager() {
        assert!(star_player_available_for(
            &star(&["DEMO_GRANIT", "DEMO_ZEPHYR"]),
            "DEMO_ZEPHYR"
        ));
    }

    #[test]
    fn un_roster_absent_de_la_liste_ne_le_peut_pas() {
        assert!(!star_player_available_for(
            &star(&["DEMO_GRANIT"]),
            "DEMO_LANTERNE"
        ));
    }

    #[test]
    fn sans_roster_rien_n_est_filtre() {
        assert!(star_player_available_for(&star(&["DEMO_GRANIT"]), ""));
    }
}
