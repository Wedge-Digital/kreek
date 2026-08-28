use async_trait::async_trait;

#[derive(Clone)]
pub struct TeamInfoDto {
    pub team_id: String,
    pub team_name: String,
    pub coach_id: String,
    pub coach_name: String,
    pub roster_name: String,
    pub logo_url: Option<String>,
}

#[async_trait]
pub trait ITeamInfoPort: Send + Sync {
    async fn find_enrolled_teams(&self, season_id: &str) -> Result<Vec<TeamInfoDto>, String>;

    /// Résout des noms d'affichage pour des équipes données, indépendamment de leur
    /// statut d'enrôlement — utilisé pour nommer des équipes exclues d'un appariement
    /// dans un message d'avertissement admin. Les ids introuvables sont omis.
    async fn find_team_names(&self, team_ids: &[String]) -> Result<Vec<TeamInfoDto>, String>;
}

// ── ACL vers le BC `match_report` (garde-fou de suppression d'un pairing) ─────

/// Consultation, pas propagation : la question « ce rapport est-il publié
/// **maintenant** ? » conditionne une action bloquante, sa fraîcheur est
/// critique. Une projection locale alimentée par event laisserait passer une
/// suppression rendue invalide entre-temps.
#[async_trait]
pub trait IMatchReportStatusPort: Send + Sync {
    /// Parmi ces pairings, ceux dont le rapport de match est publié — donc
    /// ceux dont la rencontre ne peut plus être supprimée.
    async fn find_published_pairings(&self, pairing_ids: &[String]) -> Result<Vec<String>, String>;
}

// ── ACL vers le BC `references` (résolution de noms pour l'onglet résumé) ──────

pub trait ICompetitionReferencePort: Send + Sync {
    fn find_inducement_name(&self, uid: &str) -> Option<String>;
    fn find_star_player_name(&self, uid: &str) -> Option<String>;
}

// ── ACL vers le BC `spaces` (profil membre, pour l'autorisation admin) ─────────

/// Un membre de l'espace, tel que la résolution des destinataires a besoin de le
/// connaître. DTO de lecture : primitives assumées, aucun invariant à protéger.
///
/// **C'est le seul chemin vers une adresse e-mail.** Ni `invited_coaches` ni
/// `find_enrolled_teams` n'en portent : l'intersection avec cet ensemble n'est
/// donc pas un contrôle ajouté, c'est la seule façon d'obtenir de quoi écrire à
/// quelqu'un. R7 en découle sans qu'aucune ligne ne la vérifie.
#[derive(Clone, Debug)]
pub struct SpaceMemberDto {
    pub coach_id: String,
    pub coach_name: String,
    pub email: String,
}

#[async_trait]
pub trait ICompetitionSpaceMemberPort: Send + Sync {
    /// Les membres de l'espace, avec leur adresse.
    ///
    /// Ajoutée à ce port plutôt qu'à un second vers le même BC : c'est bien
    /// d'appartenance qu'il s'agit, et le port en porte déjà le nom.
    async fn list_space_members(
        &self,
        space_id: &crate::app::shared_kernel::identity::ids::SpaceId,
    ) -> Vec<SpaceMemberDto>;

    async fn find_member_profile(
        &self,
        coach_id: &crate::app::shared_kernel::identity::ids::CoachId,
        space_id: &crate::app::shared_kernel::identity::ids::SpaceId,
    ) -> Option<crate::app::shared_kernel::identity::authorization::SpaceProfile>;

    /// Tous les espaces, pour le sélecteur de la page de test des widgets.
    ///
    /// Le nom du port parle d'appartenance et cette méthode n'en relève pas :
    /// dette de nommage assumée plutôt qu'un second port pour un appelant
    /// unique. Retourne des `SpaceDefinition` — un type d'identité partagé,
    /// déjà connu des deux côtés — pour éviter un DTO de plus.
    async fn find_all_spaces(
        &self,
    ) -> Vec<crate::app::shared_kernel::identity::space_definition::SpaceDefinition>;
}

// ── ACL vers le BC `ranking` (catalogue des critères de départage) ─────────────

/// Un critère de départage tel que le formulaire de règles a besoin de le
/// connaître. DTO de lecture : primitives assumées, aucun invariant à protéger.
pub struct TiebreakCriterionDto {
    pub code: String,
    pub label: String,
}

/// Ce qu'un rejeu de classement a fait.
pub struct RecomputeReportDto {
    pub matches_replayed: u32,
    pub teams: u32,
}

/// Demander à `ranking` de rejouer tout le classement d'une saison.
///
/// # Le premier port de ce BC qui **ordonne** au lieu de demander
///
/// Les sept autres sont des lectures. Le `CLAUDE.md` range la propagation d'un
/// effet du côté des **app events** — « on réagit à un fait qui vient de se
/// produire ailleurs ». On s'en écarte ici pour une raison précise :
///
/// **l'écran doit confirmer.** Le commissaire enregistre un barème et veut
/// savoir, dans la même réponse, combien de matchs ont été rejoués. Un événement
/// asynchrone ne le permet pas — il faudrait sonder, ou promettre sans preuve.
///
/// Et un second `POST` enchaîné par le front ne le permet pas non plus : si
/// l'onglet se ferme entre les deux, le barème reste enregistré **sans son
/// recalcul**, et le classement publié mélange alors deux règles sans que
/// personne ne l'apprenne.
///
/// Si un second cas de commande synchrone apparaît, la règle du `CLAUDE.md`
/// mérite d'être complétée plutôt que contournée une fois de plus.
#[async_trait]
pub trait IRankingRecomputePort: Send + Sync {
    async fn recompute_season(&self, season_id: &str) -> Result<RecomputeReportDto, String>;
}

pub trait ITiebreakCatalogPort: Send + Sync {
    /// Catalogue complet, dans l'ordre canonique. Synchrone : le catalogue est
    /// statique, sa consultation ne fait aucun IO.
    fn all(&self) -> Vec<TiebreakCriterionDto>;
}
