//! Quels mots-clefs proposer au coach, et dans quel ordre.
//!
//! Le service est **obligatoire** : sans lui le handler manipulerait les DTOs
//! du port pour en faire des VMs, ce que `CLAUDE.md` interdit nommément.
//!
//! Il part de `list_hateable()`, **jamais du catalogue entier** — les mots-clefs
//! de poste n'ont rien à faire dans un sélecteur de Haine, et c'est le port qui
//! garantit qu'ils n'en sortent pas (carte 401).

use crate::app::match_report::ports::{IKeywordCatalogPort, ITeamDataPort, KeywordDto};
use std::collections::HashSet;

/// Les mots-clefs haïssables, partagés selon qu'on les croise en face ou non.
pub struct HateKeywordChoices {
    pub in_opponent_roster: Vec<KeywordDto>,
    pub others: Vec<KeywordDto>,
}

// arch:no-instrument — service de lecture : assemble une vue, sans intention métier
pub async fn choix_de_haine(
    opponent_team_id: &str,
    keywords: &dyn IKeywordCatalogPort,
    team_data: &dyn ITeamDataPort,
) -> HateKeywordChoices {
    let en_face = mots_clefs_du_roster(opponent_team_id, team_data).await;
    partager(keywords.list_hateable(), &en_face)
}

/// L'union des mots-clefs du **roster** adverse, pas des joueurs alignés.
///
/// Couvrant, et sans dépendance à la feuille de match. C'est aussi ce qui
/// commande le titre du groupe — « Dans le roster adverse », et non
/// « rencontrés », qu'un poste non aligné démentirait.
async fn mots_clefs_du_roster(team_id: &str, team_data: &dyn ITeamDataPort) -> HashSet<String> {
    team_data
        .find_roster_positions(team_id)
        .await
        .into_iter()
        .flat_map(|p| p.keywords)
        .collect()
}

/// Trie par **libellé** : un coach cherche « Nain », pas `DWARF`.
///
/// Un roster dont tous les postes sont des rôles rend un premier groupe vide,
/// et tous ses mots-clefs passent dans le repli. C'est prévu : le groupe
/// disparaît plutôt que d'afficher un titre au-dessus du rien.
fn partager(mut haissables: Vec<KeywordDto>, en_face: &HashSet<String>) -> HateKeywordChoices {
    haissables.sort_by(|a, b| a.label.cmp(&b.label));
    let (in_opponent_roster, others) = haissables
        .into_iter()
        .partition(|k| en_face.contains(&k.uid));
    HateKeywordChoices {
        in_opponent_roster,
        others,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mot(uid: &str, label: &str) -> KeywordDto {
        KeywordDto {
            uid: uid.to_string(),
            label: label.to_string(),
            hate_skill_uid: format!("HAINE_{uid}"),
        }
    }

    fn catalogue() -> Vec<KeywordDto> {
        vec![
            mot("SKAVEN", "Skaven"),
            mot("DWARF", "Nain"),
            mot("ELF", "Elfe"),
            mot("DARK_ELF", "Elfe Noir"),
        ]
    }

    fn en_face(uids: &[&str]) -> HashSet<String> {
        uids.iter().map(|u| u.to_string()).collect()
    }

    #[test]
    fn les_mots_clefs_du_roster_adverse_sont_a_part() {
        let c = partager(catalogue(), &en_face(&["DWARF", "ELF"]));
        assert_eq!(
            c.in_opponent_roster
                .iter()
                .map(|k| k.uid.as_str())
                .collect::<Vec<_>>(),
            vec!["ELF", "DWARF"]
        );
        assert_eq!(
            c.others.iter().map(|k| k.uid.as_str()).collect::<Vec<_>>(),
            vec!["DARK_ELF", "SKAVEN"]
        );
    }

    /// « Elfe » avant « Elfe Noir » avant « Nain » avant « Skaven » : l'ordre du
    /// libellé, pas celui de l'uid, qui donnerait DARK_ELF avant DWARF.
    #[test]
    fn les_deux_listes_sont_triees_par_libelle() {
        let c = partager(catalogue(), &en_face(&[]));
        assert_eq!(
            c.others
                .iter()
                .map(|k| k.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Elfe", "Elfe Noir", "Nain", "Skaven"]
        );
        assert!(c.in_opponent_roster.is_empty());
    }

    /// Un roster dont tous les postes sont des rôles : le premier groupe est
    /// vide, et rien n'est perdu — tout passe dans le repli.
    #[test]
    fn un_adversaire_sans_mot_clef_connu_donne_un_premier_groupe_vide() {
        let c = partager(catalogue(), &en_face(&["BLITZER", "LINEMAN"]));
        assert!(c.in_opponent_roster.is_empty());
        assert_eq!(c.others.len(), 4, "aucun mot-clef ne doit disparaître");
    }

    /// Le partage ne fabrique rien : ce que le port ne rend pas n'apparaît nulle
    /// part, même si le roster adverse le porte. C'est ainsi qu'un `BLITZER` ne
    /// se retrouve dans aucune des deux listes.
    #[test]
    fn aucun_mot_clef_non_haissable_n_entre_dans_les_listes() {
        let c = partager(catalogue(), &en_face(&["BLITZER", "DWARF"]));
        let tous: Vec<&str> = c
            .in_opponent_roster
            .iter()
            .chain(c.others.iter())
            .map(|k| k.uid.as_str())
            .collect();
        assert_eq!(tous.len(), 4);
        assert!(!tous.contains(&"BLITZER"));
    }
}
