use crate::app::teams::domain::value_objects::Kpo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementDirection {
    Credit,
    Debit,
}

impl MovementDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Credit => "Credit",
            Self::Debit => "Debit",
        }
    }

    /// L'inverse d'`as_str`. **Sans table** : deux variantes se lisent d'un
    /// coup d'œil, et le `match` reste exhaustif dans les deux sens.
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "Credit" => Some(Self::Credit),
            "Debit" => Some(Self::Debit),
            _ => None,
        }
    }
}

/// Pourquoi la trésorerie a bougé. Le motif est ce qui rend le grand livre
/// lisible — sans lui, on aurait une suite de montants sans histoire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementReason {
    /// Dotation de départ, posée à la création de l'équipe.
    InitialEndowment,
    /// Recette d'après-match.
    MatchIncome,
    /// Recette d'après-match défaite par la correction d'un rapport publié.
    MatchIncomeReverted,
    /// Bourde coûteuse de la séquence d'après-match.
    CostlyMistake,
    /// Coups de pouce achetés pour un match. Ce qui sort de la caisse, pas ce
    /// qui a été acheté : la petite monnaie de l'underdog n'en fait pas partie.
    InducementPurchase,
    /// Rendu à la dépublication d'un rapport, pour que la correction ne laisse
    /// pas l'équipe payer un match qui n'a plus eu lieu.
    InducementRefunded,
    PlayerRecruitment,
    StaffPurchase,
}

impl MovementReason {
    /// La table qui fait foi **pour la lecture**.
    ///
    /// # Pourquoi `as_str` n'en dérive pas
    ///
    /// Il reste un `match` : le compilateur force alors à traiter toute
    /// nouvelle variante, ce qu'une recherche dans `ALL` ne ferait pas — et
    /// cela évite un `unwrap()` sur une recherche qui ne peut pas échouer.
    ///
    /// # Le trou, dit franchement
    ///
    /// Ajouter une variante **sans l'ajouter ici** n'est pas attrapé par le
    /// compilateur : `as_str` compilera, `parse` rendra `None`, et le relevé
    /// s'arrêtera sur un `UnknownReason` en production. Trois endroits à toucher
    /// pour un nouveau motif, et `tous_les_motifs_font_l_aller_retour` est **le
    /// seul** mécanisme qui les relie.
    const ALL: [(MovementReason, &'static str); 8] = [
        (Self::InitialEndowment, "InitialEndowment"),
        (Self::MatchIncome, "MatchIncome"),
        (Self::MatchIncomeReverted, "MatchIncomeReverted"),
        (Self::CostlyMistake, "CostlyMistake"),
        (Self::InducementPurchase, "InducementPurchase"),
        (Self::InducementRefunded, "InducementRefunded"),
        (Self::PlayerRecruitment, "PlayerRecruitment"),
        (Self::StaffPurchase, "StaffPurchase"),
    ];

    /// L'inverse d'`as_str`, dérivé d'`ALL`.
    ///
    /// **Sensible à la casse, délibérément** : ces chaînes sont écrites par
    /// `as_str` et jamais saisies. Tolérer une variante de casse masquerait une
    /// écriture fautive au lieu de la révéler.
    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(_, libelle)| *libelle == raw)
            .map(|(motif, _)| *motif)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InitialEndowment => "InitialEndowment",
            Self::MatchIncome => "MatchIncome",
            Self::MatchIncomeReverted => "MatchIncomeReverted",
            Self::CostlyMistake => "CostlyMistake",
            Self::InducementPurchase => "InducementPurchase",
            Self::InducementRefunded => "InducementRefunded",
            Self::PlayerRecruitment => "PlayerRecruitment",
            Self::StaffPurchase => "StaffPurchase",
        }
    }
}

/// Un mouvement de trésorerie **effectif**, tel qu'il s'est réellement produit.
///
/// `amount` n'est pas forcément le montant porté par l'événement : une bourde
/// coûteuse de 50 kPo sur une caisse de 30 retire 30, pas 50. Le solde est
/// plancher à zéro **à chaque étape**, sans report négatif — 30, moins 50, plus
/// 100 donne 100 et non 80. Les 20 manquants n'ont jamais existé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreasuryMovement {
    pub direction: MovementDirection,
    pub amount: Kpo,
    pub reason: MovementReason,
    pub balance_after: Kpo,
}

impl TreasuryMovement {
    pub fn credit(balance_before: Kpo, amount: Kpo, reason: MovementReason) -> Self {
        Self {
            direction: MovementDirection::Credit,
            amount,
            reason,
            balance_after: Kpo(balance_before.0 + amount.0),
        }
    }

    /// Le montant demandé est écrêté au solde disponible : on ne peut pas
    /// retirer ce qu'on n'a pas.
    pub fn debit(balance_before: Kpo, requested: Kpo, reason: MovementReason) -> Self {
        let amount = Kpo(requested.0.min(balance_before.0));
        Self {
            direction: MovementDirection::Debit,
            amount,
            reason,
            balance_after: Kpo(balance_before.0 - amount.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_credit_ajoute_au_solde() {
        let m = TreasuryMovement::credit(Kpo(30), Kpo(100), MovementReason::MatchIncome);
        assert_eq!(m.amount, Kpo(100));
        assert_eq!(m.balance_after, Kpo(130));
    }

    #[test]
    fn un_debit_couvert_retire_le_montant_demande() {
        let m = TreasuryMovement::debit(Kpo(100), Kpo(40), MovementReason::StaffPurchase);
        assert_eq!(m.amount, Kpo(40));
        assert_eq!(m.balance_after, Kpo(60));
    }

    /// Le cœur de la règle : perdre 50 avec 30 en caisse retire 30, et le solde
    /// tombe à zéro — il ne descend pas à −20.
    #[test]
    fn un_debit_superieur_au_solde_est_ecrete() {
        let m = TreasuryMovement::debit(Kpo(30), Kpo(50), MovementReason::CostlyMistake);
        assert_eq!(m.amount, Kpo(30), "le montant effectif, pas celui décidé");
        assert_eq!(m.balance_after, Kpo(0));
    }

    /// Aucun report négatif d'une étape à l'autre : 30 − 50 + 100 = 100.
    #[test]
    fn le_manque_ne_se_reporte_pas_sur_le_mouvement_suivant() {
        let bourde = TreasuryMovement::debit(Kpo(30), Kpo(50), MovementReason::CostlyMistake);
        let recette =
            TreasuryMovement::credit(bourde.balance_after, Kpo(100), MovementReason::MatchIncome);
        assert_eq!(recette.balance_after, Kpo(100), "et surtout pas 80");
    }

    // ── L'aller-retour des motifs (carte 435) ────────────────────────────────

    /// **La garde d'exhaustivité, et la seule qui tienne.**
    ///
    /// Elle n'a aucun usage à l'exécution : son `match` **casse la compilation**
    /// dès qu'une variante est ajoutée à `MovementReason`, et c'est ce qui force
    /// à revenir ici — puis à `tous` ci-dessous, puis à `ALL`.
    ///
    /// Sans elle, la boucle était ouverte du mauvais côté. Retirer une entrée
    /// d'`ALL` est déjà attrapé par le compilateur, le tableau étant déclaré de
    /// taille `; 8]`. Mais **ajouter** une variante sans l'y ajouter compile
    /// sans un mot : `ALL` reste à huit, et une énumération écrite à la main
    /// dans le test reste à huit elle aussi. Le test passait alors en ignorant
    /// la variante neuve, qui n'aurait échoué qu'en production, sur un
    /// `UnknownReason` arrêtant le relevé.
    #[allow(dead_code)]
    fn garde_d_exhaustivite(motif: MovementReason) {
        match motif {
            MovementReason::InitialEndowment => (),
            MovementReason::MatchIncome => (),
            MovementReason::MatchIncomeReverted => (),
            MovementReason::CostlyMistake => (),
            MovementReason::InducementPurchase => (),
            MovementReason::InducementRefunded => (),
            MovementReason::PlayerRecruitment => (),
            MovementReason::StaffPurchase => (),
        }
    }

    /// **Le test qui garde la table `ALL`.**
    ///
    /// Le compilateur force `as_str` à traiter toute nouvelle variante, mais
    /// **rien ne force à l'ajouter à `ALL`** : une variante oubliée compilerait,
    /// `parse` rendrait `None`, et le relevé s'arrêterait sur un `UnknownReason`
    /// en production. Ce test est le seul mécanisme qui relie les deux.
    ///
    /// Il énumère les huit variantes **à la main**, sans les dériver d'`ALL` :
    /// les tirer de la table qu'il vérifie le rendrait tautologique.
    #[test]
    fn tous_les_motifs_font_l_aller_retour() {
        let tous = [
            MovementReason::InitialEndowment,
            MovementReason::MatchIncome,
            MovementReason::MatchIncomeReverted,
            MovementReason::CostlyMistake,
            MovementReason::InducementPurchase,
            MovementReason::InducementRefunded,
            MovementReason::PlayerRecruitment,
            MovementReason::StaffPurchase,
        ];

        for motif in tous {
            assert_eq!(
                MovementReason::parse(motif.as_str()),
                Some(motif),
                "« {} » ne fait pas l'aller-retour : absent de ALL ?",
                motif.as_str()
            );
        }
        // `garde_d_exhaustivite` ci-dessus est ce qui rend ce compte fiable :
        // sans elle, il resterait vrai en ignorant une variante neuve.
        assert_eq!(tous.len(), 8, "une variante a été ajoutée sans venir ici");
    }

    #[test]
    fn parse_refuse_un_motif_inconnu() {
        assert_eq!(MovementReason::parse("Pillage"), None);
        assert_eq!(MovementReason::parse(""), None);
    }

    /// **Sensible à la casse, délibérément.** Ces chaînes sont écrites par
    /// `as_str` et jamais saisies : tolérer une variante de casse masquerait une
    /// écriture fautive au lieu de la révéler.
    #[test]
    fn parse_est_sensible_a_la_casse() {
        assert_eq!(MovementReason::parse("matchincome"), None);
        assert_eq!(MovementReason::parse("MATCHINCOME"), None);
        assert_eq!(
            MovementReason::parse("MatchIncome"),
            Some(MovementReason::MatchIncome)
        );
    }

    #[test]
    fn les_deux_directions_font_l_aller_retour() {
        for sens in [MovementDirection::Credit, MovementDirection::Debit] {
            assert_eq!(MovementDirection::parse(sens.as_str()), Some(sens));
        }
        assert_eq!(MovementDirection::parse("Virement"), None);
        assert_eq!(MovementDirection::parse("credit"), None);
    }
}
