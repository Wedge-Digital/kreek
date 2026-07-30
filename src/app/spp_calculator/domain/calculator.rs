/// Entrée opaque du calcul SPP — ne dépend d'aucun type du domaine `match_report`.
/// `actor_key` est fourni par l'appelant (BC match_report, via son adapter) et sert
/// uniquement de clé de regroupement, sans signification pour `spp_calculator`.
pub struct SppActionInput {
    pub actor_key: String, // arch:ok entrée opaque, sans signification pour spp_calculator (cf. doc du module)
    /// `None` pour une action qui ne rapporte rien — l'agression, la blessure
    /// subie. Elles arrivent quand même jusqu'ici : c'est le calcul qui doit
    /// répondre « zéro », pas l'appelant qui doit savoir les taire.
    pub action: Option<SppAction>,
}

/// Les cinq lignes du barème. Miroir de la catégorie du BC `match_report` —
/// jamais son type, que `spp_calculator` n'a pas le droit d'importer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SppAction {
    Touchdown,
    Pass,
    Interception,
    Casualty,
    Mvp,
}

/// Le barème d'un roster, tel que le corpus le déclare. Miroir du modèle de
/// `references`, pour la même raison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SppScale {
    pub touchdown: u8,
    pub pass: u8,
    pub interception: u8,
    pub casualty: u8,
    pub mvp: u8,
}

impl SppScale {
    fn value_of(&self, action: SppAction) -> u8 {
        match action {
            SppAction::Touchdown => self.touchdown,
            SppAction::Pass => self.pass,
            SppAction::Interception => self.interception,
            SppAction::Casualty => self.casualty,
            SppAction::Mvp => self.mvp,
        }
    }
}

pub struct SppCalculationResult {
    pub home: Vec<(String, u8)>,
    pub away: Vec<(String, u8)>,
}

/// Ce que chaque acteur a gagné sur ce match, camp par camp.
///
/// Chaque camp est compté avec **son** barème : c'est tout l'objet de la
/// distinction, deux équipes d'un même match ne valorisent pas forcément les
/// mêmes gestes de la même façon.
///
/// Les acteurs à zéro sont écartés — celui qui n'a fait qu'agresser, ou qui n'a
/// fait qu'encaisser, n'a pas sa place dans une carte « Performances ».
pub fn calculate(
    home_actions: &[SppActionInput],
    away_actions: &[SppActionInput],
    home_scale: SppScale,
    away_scale: SppScale,
) -> SppCalculationResult {
    SppCalculationResult {
        home: spp_by_actor(home_actions, home_scale),
        away: spp_by_actor(away_actions, away_scale),
    }
}

/// L'ordre de première apparition est conservé — un tri par montant aurait sa
/// place dans la vue, pas dans un calcul.
fn spp_by_actor(actions: &[SppActionInput], scale: SppScale) -> Vec<(String, u8)> {
    let mut totals: Vec<(String, u8)> = vec![];
    for action in actions {
        let gain = action.action.map_or(0, |a| scale.value_of(a));
        match totals.iter_mut().find(|(key, _)| key == &action.actor_key) {
            Some((_, total)) => *total = total.saturating_add(gain),
            None => totals.push((action.actor_key.clone(), gain)),
        }
    }
    totals.retain(|(_, total)| *total > 0);
    totals
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le barème du corpus de démonstration pour un roster ordinaire.
    const NORMAL: SppScale = SppScale {
        touchdown: 4,
        pass: 1,
        interception: 3,
        casualty: 2,
        mvp: 5,
    };

    /// Celui des Brutes Bagarreuses : la sortie prime sur l'essai.
    const BRUTES: SppScale = SppScale {
        touchdown: 2,
        pass: 1,
        interception: 3,
        casualty: 4,
        mvp: 5,
    };

    fn geste(acteur: &str, action: Option<SppAction>) -> SppActionInput {
        SppActionInput {
            actor_key: acteur.to_string(),
            action,
        }
    }

    #[test]
    fn une_blessure_subie_ne_credite_rien_a_sa_victime() {
        let actions = vec![geste("p1", None)];
        let result = calculate(&actions, &[], NORMAL, NORMAL);
        assert!(result.home.is_empty(), "p1 n'a fait qu'encaisser");
    }

    #[test]
    fn les_gains_d_un_meme_acteur_s_additionnent() {
        let actions = vec![
            geste("p1", Some(SppAction::Touchdown)),
            geste("p1", Some(SppAction::Casualty)),
            geste("p2", Some(SppAction::Mvp)),
        ];
        let result = calculate(&actions, &[], NORMAL, NORMAL);
        assert_eq!(result.home, vec![("p1".into(), 6), ("p2".into(), 5)]);
    }

    /// Le stub créditait un forfait par acteur, quoi qu'il fasse. Ce test le
    /// dit en toutes lettres : deux actions de types différents ne valent pas
    /// la même chose.
    #[test]
    fn deux_types_d_action_ne_valent_pas_la_meme_chose() {
        let essai = calculate(
            &[geste("p1", Some(SppAction::Touchdown))],
            &[],
            NORMAL,
            NORMAL,
        );
        let sortie = calculate(
            &[geste("p1", Some(SppAction::Casualty))],
            &[],
            NORMAL,
            NORMAL,
        );
        assert_eq!(essai.home[0].1, NORMAL.touchdown);
        assert_eq!(sortie.home[0].1, NORMAL.casualty);
        assert_ne!(essai.home[0].1, sortie.home[0].1);
    }

    /// Le test que la carte 276 réclamait, et celui qu'un barème unique recodé
    /// en dur ne peut pas passer : le **même** match, valorisé des deux côtés
    /// par le barème du camp concerné, ne rend pas les mêmes totaux.
    ///
    /// Les gestes sont **asymétriques à dessein** — un essai et *deux* sorties.
    /// Avec un essai et une sortie, les deux barèmes étant une permutation l'un
    /// de l'autre, les totaux se rejoignaient à 6 et le test ne pouvait pas
    /// échouer. C'est exactement le piège relevé sur la carte 275, et
    /// l'assertion finale est là pour qu'il ne repasse jamais inaperçu.
    #[test]
    fn un_meme_match_rend_des_totaux_differents_selon_le_bareme_du_camp() {
        let gestes = || {
            vec![
                geste("p1", Some(SppAction::Touchdown)),
                geste("p1", Some(SppAction::Casualty)),
                geste("p1", Some(SppAction::Casualty)),
            ]
        };
        let result = calculate(&gestes(), &gestes(), BRUTES, NORMAL);

        assert_eq!(result.home[0].1, BRUTES.touchdown + 2 * BRUTES.casualty); // 2 + 8
        assert_eq!(result.away[0].1, NORMAL.touchdown + 2 * NORMAL.casualty); // 4 + 4
        assert_ne!(
            result.home[0].1, result.away[0].1,
            "les mêmes gestes, deux barèmes, deux totaux"
        );
    }

    #[test]
    fn une_agression_seule_n_ouvre_aucune_ligne() {
        let actions = vec![geste("p1", None), geste("p2", Some(SppAction::Touchdown))];
        let result = calculate(&actions, &[], NORMAL, NORMAL);
        assert_eq!(result.home, vec![("p2".into(), NORMAL.touchdown)]);
    }
}
