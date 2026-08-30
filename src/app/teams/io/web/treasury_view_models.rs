//! Les view models du relevé de trésorerie (carte 436).
//!
//! Ils sont **inertes** : aucune méthode, aucun calcul. Tout ce qui les
//! construit — et tout ce qui les formate — vit dans `builders.rs`, parce que
//! le relevé descend d'un service qui interroge un port, jamais d'un agrégat.
//! C'est ce qui rendra la traduction possible : un seul fichier à toucher pour
//! cet écran.

/// Le relevé complet, prêt à rendre.
pub struct TreasuryVm {
    pub summary: SummaryVm,
    pub groups: Vec<GroupVm>,
    /// Aucun mouvement au-delà de la dotation : le gabarit rend le bloc
    /// « Aucun mouvement pour l'instant » au lieu du tableau.
    pub is_opening_only: bool,
    /// Le nombre de lignes du relevé, **dotation comprise** — c'est ce que
    /// compte l'indication « 11 mouvements » sous le titre.
    pub movement_count: u32,
}

/// Le bandeau : l'équation qui explique le solde, lue de gauche à droite.
///
/// `dotation + encaissé − dépensé = solde`, et les quatre termes viennent du
/// même relevé — c'est cette égalité qui rend le bandeau vérifiable à l'œil.
pub struct SummaryVm {
    pub opening_kpo: u32,
    /// Encaissé **dotation exclue** : elle a sa propre colonne dans l'équation,
    /// et l'y compter deux fois la ferait fausse.
    pub credited_kpo: u32,
    pub debited_kpo: u32,
    pub balance_kpo: u32,
}

/// Une période du relevé : ce qui s'est passé depuis un match, ou l'ouverture.
///
/// **Le titre ouvre une période, il n'étiquette pas des lignes.** Seuls trois
/// motifs portent un identifiant de rapport ; la recette de match, la ligne la
/// plus fréquente, n'en porte pas. Rattacher chaque ligne à son match est donc
/// impossible — et inutile : un relevé de compte se lit par tranches de temps.
pub struct GroupVm {
    /// `None` pour l'ouverture — la dotation n'a pas de journée, et le gabarit
    /// n'affiche alors aucun séparateur.
    pub heading: Option<String>,
    pub rows: Vec<MovementRowVm>,
}

pub struct MovementRowVm {
    /// « 12 août »
    pub date_label: String,
    pub icon: &'static str,
    /// « Recrutement »
    pub label: String,
    /// « Gwenn, Passeuse — n° 7 ». `None` quand le détail n'apprendrait rien de
    /// plus que le libellé : une chaîne vide laisserait un `<div>` qui prend sa
    /// marge.
    pub detail: Option<String>,
    /// « −90 kPo », **signe compris**.
    pub amount_label: String,
    /// « 380 kPo », **sans signe** : un solde est un état.
    pub balance_label: String,
    pub kind: RowKind,
    /// La couleur du montant suit le sens du mouvement, pas la nature de la
    /// ligne : une correction rend de l'argent ou en reprend, et les deux se
    /// lisent dans le relevé.
    pub is_credit: bool,
}

/// Ce que la ligne **est**, pas comment elle s'affiche. Le gabarit en tire ses
/// classes ; c'est lui qui décide de la couleur, pas le view model — sans quoi
/// changer une teinte demanderait de recompiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// La dotation : le point de départ du relevé, pas un mouvement.
    Opening,
    Credit,
    Debit,
    /// `MatchIncomeReverted` et `InducementRefunded` — elles défont la ligne
    /// précédente au lieu d'en ajouter une.
    Correction,
}
