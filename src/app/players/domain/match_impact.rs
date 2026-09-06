use crate::app::players::domain::player::TeamId;
use nutype::nutype;
use serde::{Deserialize, Serialize};

// ── Contexte de match (embarqué dans chaque event, zéro appel inter-BC en lecture) ──

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchReportId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchContext {
    pub match_report_id: MatchReportId,
    pub round_id: RoundId,
    pub round_label: String, // arch:ok texte libre dénormalisé (zéro appel inter-BC en lecture)
    pub opponent_team_id: TeamId,
    pub opponent_team_name: String, // arch:ok texte libre dénormalisé (zéro appel inter-BC en lecture)
}

// ── SPP gagné par une action (résolu en amont via references, jamais calculé ici) ──

#[nutype(
    validate(greater_or_equal = 1),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct SppEarned(u32);

// ── Statut de participation ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerParticipationStatus {
    Available,
    MissingNextGame,
    Retired,
    Dead,
}

impl PlayerParticipationStatus {
    /// **Le défaut est prudent, et c'est délibéré.**
    ///
    /// `RosterMembership::from_str` retombe sur `Active`, et c'est juste chez
    /// lui : le doute y porte sur une appartenance, que rien n'exagère.
    ///
    /// Ici le doute porte sur un **compte affiché**. Un `_ => Available`
    /// ferait entrer dans le sous-total des joueurs disponibles tout statut
    /// que le code ne reconnaît pas — donc **gonflerait** un chiffre que le
    /// coach regarde précisément pour en vérifier un autre. Se tromper vers
    /// « indisponible » minore et se voit ; se tromper vers « disponible »
    /// majore et passe pour juste.
    ///
    /// C'est déjà le parti de `infrastructure/teams/squad_adapter.rs`, qui ne
    /// reconnaît que `"Available"` et range tout le reste en empêché.
    pub fn from_str(valeur: &str) -> Self {
        match valeur {
            "Available" => Self::Available,
            "Retired" => Self::Retired,
            "Dead" => Self::Dead,
            _ => Self::MissingNextGame,
        }
    }

    /// Jouera-t-il le prochain match ?
    ///
    /// **La même question que celle de la valeur d'équipe**, qui ne retient que
    /// les alignables (`players_value`, via `SquadPresence::alignable()`). Le
    /// sous-total de l'effectif existe pour rendre ce chiffre vérifiable : s'il
    /// comptait un joueur de plus ou de moins, il ne le vérifierait pas, il le
    /// contredirait.
    pub fn disponible(&self) -> bool {
        matches!(self, Self::Available)
    }
}

#[cfg(test)]
mod tests_participation {
    use super::PlayerParticipationStatus as Statut;

    #[test]
    fn seul_available_est_disponible() {
        assert!(Statut::Available.disponible());
        assert!(!Statut::MissingNextGame.disponible());
        assert!(!Statut::Retired.disponible());
        assert!(!Statut::Dead.disponible());
    }

    #[test]
    fn les_quatre_statuts_font_l_aller_retour() {
        for (texte, attendu) in [
            ("Available", Statut::Available),
            ("MissingNextGame", Statut::MissingNextGame),
            ("Retired", Statut::Retired),
            ("Dead", Statut::Dead),
        ] {
            assert_eq!(Statut::from_str(texte), attendu, "{texte}");
        }
    }

    /// Le sens du défaut : un statut que le code ne connaît pas ne doit pas
    /// grossir le compte des disponibles.
    #[test]
    fn un_statut_inconnu_ne_compte_pas_comme_disponible() {
        assert!(!Statut::from_str("Suspendu").disponible());
        assert!(!Statut::from_str("").disponible());
    }

    /// La valeur d'équipe traduit `"Available"` en `Alignable` et **tout le
    /// reste** en empêché ou perdu (`squad_adapter.rs`). Ce test tient les deux
    /// définitions ensemble : si l'une s'ouvrait à un statut de plus sans
    /// l'autre, le sous-total cesserait de vérifier la valeur d'équipe.
    #[test]
    fn le_predicat_dit_la_meme_chose_que_la_valeur_d_equipe() {
        for texte in ["Available", "MissingNextGame", "Retired", "Dead", "Inconnu"] {
            let alignable_pour_la_ve = texte == "Available";
            assert_eq!(
                Statut::from_str(texte).disponible(),
                alignable_pour_la_ve,
                "{texte}"
            );
        }
    }
}

// ── Blessures ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatKind {
    Ma,
    St,
    Ag,
    Pa,
    Av,
}

impl StatKind {
    /// Ce qu'un cran d'**amélioration** fait à la valeur brute.
    ///
    /// MV, FO et AR montent quand le joueur progresse. AG et PA sont des
    /// nombres cibles à atteindre au dé : ils **descendent**.
    ///
    /// AR s'affiche avec un « + » comme AG et PA mais se comporte à l'inverse —
    /// en règles 2020 l'adversaire doit atteindre la cible pour blesser, donc
    /// une armure haute protège mieux. Le suffixe ne dit rien de la direction ;
    /// cette table est la seule à faire foi.
    ///
    /// Elle vit **ici**, dans le domaine, et non dans `player_stats_service` où
    /// elle se trouvait : le panier de customisation en a besoin pour juger si
    /// une amélioration franchit une borne, et deux tables auraient fini par
    /// diverger.
    pub fn improvement_step(self) -> i8 {
        match self {
            Self::Ma | Self::St | Self::Av => 1,
            Self::Ag | Self::Pa => -1,
        }
    }

    /// L'offset **brut** que valent `crans` d'amélioration. C'est ce que le
    /// joueur voit écrit (« Amélioration d'Agilité −1 ») : l'affichage annonce
    /// l'effet sur la valeur, pas l'intention.
    ///
    /// Ici et pas dans la vue, sans quoi la table des directions se
    /// retrouverait multipliée à la main dans chaque libellé.
    pub fn raw_offset(self, crans: i8) -> i8 {
        crans * self.improvement_step()
    }

    /// Bornes **inclusives** de la valeur brute résolue. Une modification qui
    /// en sortirait est refusée.
    pub fn bounds(self) -> (u8, u8) {
        match self {
            Self::Ma => (0, 9),
            Self::St => (0, 9),
            Self::Ag => (1, 6),
            Self::Pa => (1, 6),
            Self::Av => (2, 12),
        }
    }

    /// La valeur brute obtenue en appliquant `crans` d'amélioration (négatif
    /// pour une dégradation), bornes comprises. `None` si le résultat sort des
    /// bornes — c'est le refus, pas un écrêtage silencieux.
    pub fn apply_crans(self, current: u8, crans: i8) -> Option<u8> {
        let (min, max) = self.bounds();
        let cible = current as i16 + crans as i16 * self.improvement_step() as i16;
        match cible >= min as i16 && cible <= max as i16 {
            true => Some(cible as u8),
            false => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InjuryType {
    Commotion,
    Amoche,
    BlessureSerieuse,
    Sequel { stat: StatKind },
    Mort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInjuryRecord {
    pub injury_type: InjuryType,
    pub context: MatchContext,
}

#[nutype(
    validate(greater_or_equal = 1),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct StatMalus(u8);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StatAdjustment {
    pub stat: StatKind,
    pub malus: StatMalus,
}

// ── Compteurs de carrière (même style que Spp/ValueKpo dans player.rs) ─────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TouchdownCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PassCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InterceptionCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CasualtyCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MvpCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FoulCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PersistentInjuryCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MatchesPlayedCount(pub u16);

#[cfg(test)]
mod stat_kind_tests {
    use super::StatKind;

    /// Le test qui protège toute la fonctionnalité : améliorer l'agilité
    /// **descend** le seuil de dé, améliorer l'armure le monte. Le suffixe « + »
    /// que les deux partagent à l'affichage ne dit rien de la direction.
    #[test]
    fn ameliorer_descend_les_seuils_de_de_et_monte_le_reste() {
        assert_eq!(StatKind::Ag.improvement_step(), -1);
        assert_eq!(StatKind::Pa.improvement_step(), -1);
        assert_eq!(StatKind::Ma.improvement_step(), 1);
        assert_eq!(StatKind::St.improvement_step(), 1);
        assert_eq!(StatKind::Av.improvement_step(), 1);
    }

    #[test]
    fn les_bornes_couvrent_les_cinq_caracteristiques() {
        assert_eq!(StatKind::Ma.bounds(), (0, 9));
        assert_eq!(StatKind::St.bounds(), (0, 9));
        assert_eq!(StatKind::Ag.bounds(), (1, 6));
        assert_eq!(StatKind::Pa.bounds(), (1, 6));
        assert_eq!(StatKind::Av.bounds(), (2, 12));
    }

    #[test]
    fn apply_crans_ameliore_dans_le_bon_sens() {
        // AG 3+ améliorée d'un cran devient 2+.
        assert_eq!(StatKind::Ag.apply_crans(3, 1), Some(2));
        // AR 8+ améliorée d'un cran devient 9+ — plus haut protège mieux.
        assert_eq!(StatKind::Av.apply_crans(8, 1), Some(9));
        // MV 7 dégradé d'un cran devient 6.
        assert_eq!(StatKind::Ma.apply_crans(7, -1), Some(6));
    }

    /// Hors bornes = refus, jamais écrêtage : rendre `Some(1)` là où l'on
    /// demandait `0+` ferait croire à une application réussie.
    #[test]
    fn apply_crans_refuse_hors_bornes_sans_ecreter() {
        assert_eq!(StatKind::Ag.apply_crans(1, 1), None); // 1+ ne peut pas mieux
        assert_eq!(StatKind::Ag.apply_crans(6, -1), None); // 6+ ne peut pas pire
        assert_eq!(StatKind::Ma.apply_crans(9, 1), None);
        assert_eq!(StatKind::Ma.apply_crans(0, -1), None);
        assert_eq!(StatKind::Av.apply_crans(12, 1), None);
        assert_eq!(StatKind::Av.apply_crans(2, -1), None);
    }

    #[test]
    fn apply_crans_accepte_une_amplitude_superieure_a_un() {
        assert_eq!(StatKind::Ag.apply_crans(5, 3), Some(2));
        assert_eq!(StatKind::Ag.apply_crans(5, 5), None);
    }
}
