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
    PlayerRecruitment,
    StaffPurchase,
}

impl MovementReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InitialEndowment => "InitialEndowment",
            Self::MatchIncome => "MatchIncome",
            Self::MatchIncomeReverted => "MatchIncomeReverted",
            Self::CostlyMistake => "CostlyMistake",
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
}
