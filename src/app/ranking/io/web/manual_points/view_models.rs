//! Les vues de la page des points manuels.
//!
//! Regroupées par équipe, en accordéon : c'est la question que se pose un
//! commissaire — « qu'est-ce que cette équipe a reçu ? » —, pas « qu'ai-je
//! attribué mardi ? ». Un relevé chronologique répondrait à la seconde.

/// Une ligne, telle que la page la montre.
pub struct ManualPointVm {
    pub id: i64,
    /// L'URL de retrait, **construite par le builder**.
    ///
    /// Askama passe `line.id` en `&i64`, et les gabarits du projet ne
    /// déréférencent pas. Plus fondamentalement : un VM porte ce que le gabarit
    /// rend — `team_link` du classement suit déjà ce parti pris.
    pub delete_url: String,
    /// Déjà signé — « +3 », « −1 ».
    pub points: String,
    /// `plus` ou `minus` : la maquette colore le signe. La classe est calculée
    /// ici plutôt que par un `{% if %}` dans le gabarit, qui se recopierait à
    /// chaque endroit affichant un signe.
    pub points_class: &'static str,
    /// Le texte libre du commissaire, ou `None` : le motif est facultatif.
    pub reason: Option<String>,
    pub awarded_by: String,
    pub awarded_at: String,
}

/// Un bloc d'accordéon : une équipe, son total, ses lignes.
pub struct ManualPointsTeamVm {
    pub team_id: String,
    pub team_name: String,
    /// Le total du bloc, signé — ce qu'on lit replié.
    pub total: String,
    pub total_class: &'static str,
    pub line_count: usize,
    /// « 1 ligne » / « 2 lignes ». Le pluriel se calcule ici plutôt que dans le
    /// gabarit : Askama n'a pas de règle de pluriel, et un `{% if %}` par
    /// occurrence se recopie mal.
    pub line_label: String,
    pub lines: Vec<ManualPointVm>,
}

pub struct ManualPointsListVm {
    pub teams: Vec<ManualPointsTeamVm>,
    /// « 4 lignes · 3 équipes concernées » — le compte que la tête du panneau
    /// affiche, calculé ici pour la même raison que `line_label`.
    pub teams_label: String,
    /// **Le gabarit rend ou non la colonne de suppression, il ne décide pas.**
    /// L'autorisation est une décision applicative ; la laisser au gabarit la
    /// disperserait dans le HTML, où personne ne va la chercher.
    pub can_manage: bool,
}

/// Ce que le formulaire propose, et ce qu'il conserve après un enregistrement.
pub struct ManualPointsFormVm {
    /// L'équipe reste choisie d'un envoi à l'autre : le geste réel est d'en
    /// traiter plusieurs d'affilée pour la même équipe — les forfaits d'une
    /// journée. Tout réinitialiser ferait re-choisir à chaque fois.
    pub selected_team_id: Option<String>,
    /// Les points et le motif, eux, **sont vidés** : les garder ferait attribuer
    /// deux fois le même nombre par inadvertance.
    pub error: Option<String>,
    pub can_manage: bool,
}
