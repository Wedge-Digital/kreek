//! Le panneau de customisation — chemin de **lecture** seul.
//!
//! Il occupe `#pd-right-panel`, troisième occupant du slot après le journal des
//! évolutions et le panneau de dépense de SPP. Le fragment porte l'`id` du
//! conteneur : en `outerHTML`, le fragment *est* le conteneur — l'interdiction
//! du CLAUDE.md vise les swaps `innerHTML`.
//!
//! **Tout est rendu par le serveur** : valeurs effectives, aperçus, lignes en
//! attente, grisage des boutons. Le panier vivant côté base, la vue n'a rien à
//! simuler. Ce qui reste en Alpine se limite à la bascule d'onglets et au
//! filtre de recherche des compétences.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::players::domain::customisation_basket::{
    is_expired, ActionState, CustomisationBasket, CustomisationLine,
};
use crate::app::players::domain::match_impact::StatKind;
use crate::app::players::domain::player::{Player, PlayerId};
use crate::app::players::io::web::player_detail_controller::can_customise;
use crate::app::players::io::web::widgets::evolution_journal_widget::{
    evolution_journal_widget, EvolutionJournalParams,
};
use crate::app::players::io::web::widgets::stat_display;
use crate::app::players::ports::{
    CustomisationBasketState, ICustomisationBasketRepository, ISkillCatalogPort, RepositoryError,
};
use crate::app::players::use_cases::customisation_basket_hydration_service::hydrate;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

// ── View models ───────────────────────────────────────────────────────────────

pub struct AddableSkillVm {
    pub skill_id: String,
    pub name: String,
    pub description: String,
    pub category_css: String,
    pub category_label: String,
}

pub struct StatCardVm {
    pub key: &'static str,
    pub label: &'static str,
    pub name: &'static str,
    /// Valeur **effective**, panier compris, déjà formatée (« 3+ », « 7 »).
    pub current: String,
    /// Offset brut déjà au panier, `None` s'il n'y en a pas.
    pub pending_offset: Option<i16>,
    /// Ce que vaudrait la caractéristique après un cran de plus, dans chaque
    /// sens. `None` quand la borne est atteinte — le bouton est alors grisé.
    pub preview_up: Option<String>,
    pub preview_down: Option<String>,
}

pub struct PendingLineVm {
    pub line_id: String,
    pub label: String,
    pub family: &'static str,
}

pub struct CustomisationVm {
    pub player_name: String,
    pub spp_reserve: u32,
    pub skills: Vec<AddableSkillVm>,
    pub stats: Vec<StatCardVm>,
    pub price_kpo: u32,
    pub spp_earned: u32,
    pub pending: Vec<PendingLineVm>,
    pub add_skill_url: String,
    pub add_stat_url: String,
    pub adjust_price_url: String,
    pub add_spp_url: String,
    pub remove_line_url: String,
    pub validate_url: String,
    pub cancel_url: String,
    /// Version du panier, embarquée dans chaque geste : c'est la garde
    /// d'écriture concurrente, et le panneau est re-rendu à chaque action —
    /// elle repart donc toujours à jour.
    pub version: u32,
}

// ── Construction ──────────────────────────────────────────────────────────────

/// Les identifiants sont ceux du référentiel, au pluriel près : c'est
/// `MUTATIONS` et non `MUTATION`, et `DEVIOUS` et `TRAITS` existent aussi.
///
/// La table copiée depuis `skill_picker.rs` disait `MUTATION` au singulier et
/// ignorait les deux dernières : trois catégories sur sept retombaient
/// silencieusement sur la pastille « général ». Le défaut vit toujours dans
/// `references::skill_picker`, `player_table_widget` et `team_created_listener`
/// — il déborde de cette carte, mais il est réel.
/// `None` pour une catégorie inconnue de la table — distinct de « connue et
/// générale ». Sans cette distinction, un test de couverture ne peut pas voir
/// qu'une catégorie est tombée dans le repli : `GENERAL` et l'inconnu rendent
/// la même classe.
fn try_category_css(category_id: &str) -> Option<&'static str> {
    match category_id {
        "GENERAL" => Some("type-general"),
        "STRENGTH" => Some("type-strength"),
        "AGILITY" => Some("type-agility"),
        "PASSING" => Some("type-passing"),
        "MUTATIONS" => Some("type-mutation"),
        "DEVIOUS" => Some("type-devious"),
        "TRAITS" => Some("type-traits"),
        _ => None,
    }
}

/// Une catégorie inconnue reste lisible plutôt que sans pastille : un
/// référentiel tiers peut porter des catégories que cette table ignore.
fn category_css(category_id: &str) -> &'static str {
    try_category_css(category_id).unwrap_or("type-general")
}

/// Seules les compétences **ajoutables** sont listées : ni celles que le joueur
/// possède, ni celles déjà au panier. La customisation ignore en revanche
/// l'accès du poste — c'est ce qui la distingue du `skill-picker`.
fn build_skills(
    basket: &CustomisationBasket,
    catalog: &dyn ISkillCatalogPort,
) -> Vec<AddableSkillVm> {
    let mut skills: Vec<AddableSkillVm> = catalog
        .list_all_skills()
        .into_iter()
        .filter(|s| est_ajoutable(basket, &s.skill_id))
        .map(|s| AddableSkillVm {
            category_css: category_css(&s.category).to_string(),
            category_label: s.category_label,
            skill_id: s.skill_id,
            name: s.name,
            description: s.description,
        })
        .collect();
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

fn est_ajoutable(basket: &CustomisationBasket, skill_id: &str) -> bool {
    match crate::app::players::domain::value_objects::SkillId::try_new(skill_id.to_string()) {
        Ok(id) => matches!(basket.action_for_skill(&id), ActionState::Allowed),
        Err(_) => false,
    }
}

fn build_stats(basket: &CustomisationBasket) -> Vec<StatCardVm> {
    stat_display::ALL
        .iter()
        .map(|d| {
            let courant = basket.effective_stat(d.stat);
            let offset = basket.pending_offset(d.stat);
            StatCardVm {
                key: d.key,
                label: d.label,
                name: d.name,
                current: stat_display::format(courant, d.is_target),
                pending_offset: (offset != 0).then_some(offset),
                preview_up: apercu(basket, d.stat, 1, d.is_target),
                preview_down: apercu(basket, d.stat, -1, d.is_target),
            }
        })
        .collect()
}

/// L'aperçu et le grisage ne font qu'un : une borne atteinte se voit à l'absence
/// de valeur à afficher. Le refus serveur reste la vérité — un autre onglet peut
/// avoir consommé la marge entre l'affichage et le clic.
fn apercu(
    basket: &CustomisationBasket,
    stat: StatKind,
    sens: i8,
    is_target: bool,
) -> Option<String> {
    match basket.action_for_stat(stat, sens) {
        ActionState::Allowed => stat
            .apply_crans(basket.effective_stat(stat), sens)
            .map(|v| stat_display::format(v, is_target)),
        _ => None,
    }
}

fn build_pending(
    basket: &CustomisationBasket,
    catalog: &dyn ISkillCatalogPort,
) -> Vec<PendingLineVm> {
    basket
        .lines()
        .iter()
        .map(|l| PendingLineVm {
            line_id: l.id().as_ref().to_string(),
            label: pending_label(l, catalog),
            family: family_label(l),
        })
        .collect()
}

fn family_label(line: &CustomisationLine) -> &'static str {
    match line {
        CustomisationLine::Skill { .. } => "Compétence",
        CustomisationLine::Stat { .. } => "Caractéristique",
        CustomisationLine::Price { .. } => "Prix",
        CustomisationLine::Spp { .. } => "SPP",
    }
}

/// Le libellé annonce **l'offset qui sera appliqué**, jamais un total
/// intermédiaire : deux améliorations chaînées se lisent alors chacune pour ce
/// qu'elle vaut. L'amplitude peut être négative sans cesser d'être une
/// amélioration — « Amélioration d'Agilité −1 » est correct, l'agilité étant un
/// nombre cible à atteindre au dé.
fn pending_label(line: &CustomisationLine, catalog: &dyn ISkillCatalogPort) -> String {
    match line {
        CustomisationLine::Skill { skill_id, .. } => catalog
            .find_skill(skill_id.as_ref())
            .map(|s| s.name)
            .unwrap_or_else(|| skill_id.as_ref().to_string()),
        CustomisationLine::Stat { stat, crans, .. } => stat_label(*stat, crans.into_inner()),
        CustomisationLine::Price { delta, .. } => {
            format!("Prix {} kPo", signe(delta.into_inner() as i16))
        }
        CustomisationLine::Spp { amount, .. } => {
            format!("SPP +{}", amount.into_inner())
        }
    }
}

fn stat_label(stat: StatKind, crans: i8) -> String {
    let sens = match crans > 0 {
        true => "Amélioration",
        false => "Dégradation",
    };
    let nom = stat_display::ALL
        .iter()
        .find(|d| d.stat == stat)
        .map(|d| d.name)
        .unwrap_or("Caractéristique");
    let article = match nom.starts_with(['A', 'E', 'I', 'O', 'U']) {
        true => "d'",
        false => "de ",
    };
    format!(
        "{sens} {article}{nom} {}",
        signe(stat.raw_offset(crans) as i16)
    )
}

/// Un signe explicite dans les deux sens : « +10 » se lit comme un ajout, « 10 »
/// se lirait comme une valeur.
fn signe(v: i16) -> String {
    match v >= 0 {
        true => format!("+{v}"),
        false => v.to_string(),
    }
}

fn build_vm(
    player: &Player,
    basket: &CustomisationBasket,
    catalog: &dyn ISkillCatalogPort,
    routes: &AppRoutes,
    space_id: &str,
) -> CustomisationVm {
    let player_id = player.id.0.as_str();
    let c = &routes.players;
    CustomisationVm {
        player_name: player.position_name.to_string(),
        spp_reserve: reserve_effective(player, basket),
        skills: build_skills(basket, catalog),
        stats: build_stats(basket),
        price_kpo: basket.effective_value().0,
        spp_earned: basket.effective_spp().0,
        pending: build_pending(basket, catalog),
        add_skill_url: c.customisation_add_skill(space_id, player_id),
        add_stat_url: c.customisation_add_stat(space_id, player_id),
        adjust_price_url: c.customisation_adjust_price(space_id, player_id),
        add_spp_url: c.customisation_add_spp(space_id, player_id),
        remove_line_url: c.customisation_remove_line(space_id, player_id),
        validate_url: c.customisation_validate(space_id, player_id),
        cancel_url: c.customisation_cancel(space_id, player_id),
        version: basket.version().0,
    }
}

/// La réserve suit le panier : ajouter des SPP au panier doit se voir dans
/// l'en-tête, sans quoi le commissaire ne saurait pas ce qu'il vient de faire.
/// Les SPP déjà dépensés, eux, ne bougent pas.
fn reserve_effective(player: &Player, basket: &CustomisationBasket) -> u32 {
    let depenses = player.spp.0.saturating_sub(player.spp_remaining());
    basket.effective_spp().0.saturating_sub(depenses)
}

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "player-customisation-widget.html")]
pub struct PlayerCustomisationTemplate {
    pub vm: CustomisationVm,
}

impl IntoResponse for PlayerCustomisationTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("player_customisation_widget render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn player_customisation_widget(
    Path((space_id, player_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
) -> Response {
    let Some(user) = auth_session.user.clone() else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    match autoriser(&state, &user, &space_id, &player_id).await {
        Err(refus) => refus,
        // Défense en profondeur : l'URL atteinte directement par qui n'a pas le
        // droit retombe sur le journal, jamais sur le mode.
        Ok(false) => journal(space_id, player_id, state).await,
        Ok(true) => match ouvrir_panier(&state, &space_id, &player_id).await {
            Ok(()) => rendre_panneau(&state, &space_id, &player_id).await,
            Err(e) => {
                tracing::error!("player_customisation_widget ouverture panier {player_id}: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
    }
}

/// `Err` porte la réponse d'échec (joueur ou équipe introuvable, espace mal
/// formé), `Ok(false)` un refus de droit — les deux ne se traitent pas de la
/// même façon : le premier est une erreur, le second une retombée sur le
/// journal.
async fn autoriser(
    state: &AppState,
    user: &crate::app::auth::domain::user::User,
    space_id: &str,
    player_id: &str,
) -> Result<bool, Response> {
    let Ok(space_id_vo) = SpaceId::try_new(space_id) else {
        return Err(StatusCode::BAD_REQUEST.into_response());
    };
    let team = charger_equipe(state, player_id).await?;
    let coach_name = user.coach_name.clone().into_inner();
    Ok(can_customise(state, &user.id, &coach_name, &space_id_vo, &team).await)
}

/// L'équipe du joueur — c'est elle qui porte la compétition, donc l'admin
/// susceptible d'autoriser la customisation.
async fn charger_equipe(
    state: &AppState,
    player_id: &str,
) -> Result<crate::app::players::ports::TeamRosterInfoDto, Response> {
    let player = match state
        .players
        .repository
        .find_by_id(&PlayerId(player_id.to_string()))
        .await
    {
        Ok(Some(p)) => p,
        Ok(None) => return Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("player_customisation_widget find_by_id {player_id}: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    };
    state
        .players
        .roster_port
        .find_team_info(&player.team_id.0)
        .await
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())
}

async fn rendre_panneau(state: &AppState, space_id: &str, player_id: &str) -> Response {
    let catalog = state.players.skill_catalog.as_ref();
    let hydrate = hydrate(
        &PlayerId(player_id.to_string()),
        state.players.repository.as_ref(),
        state.players.customisation_basket_repository.as_ref(),
        catalog,
    )
    .await;

    match hydrate {
        Ok((basket, player)) => PlayerCustomisationTemplate {
            vm: build_vm(&player, &basket, catalog, &AppRoutes::default(), space_id),
        }
        .into_response(),
        Err(e) => {
            tracing::error!("player_customisation_widget hydratation {player_id}: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Ouvrir le panier — même vide — **est** l'entrée dans le mode : le contrôleur
/// de page choisit l'occupant du slot en regardant son existence, et un
/// rechargement complet retombe donc sur le panneau sans rien de plus.
///
/// Un panier périmé est supprimé au passage plutôt que rouvert : le
/// commissaire qui revient repart d'une page blanche, pas d'une saisie de la
/// veille dont il ne se souvient plus.
async fn ouvrir_panier(
    state: &AppState,
    space_id: &str,
    player_id: &str,
) -> Result<(), RepositoryError> {
    let repo = state.players.customisation_basket_repository.as_ref();
    match repo.load(player_id).await? {
        Some(etat) if is_expired(etat.updated_at, time::OffsetDateTime::now_utc()) => {
            repo.delete(player_id).await?;
            creer_panier_vide(repo, space_id, player_id).await
        }
        Some(_) => Ok(()),
        None => creer_panier_vide(repo, space_id, player_id).await,
    }
}

async fn creer_panier_vide(
    repo: &dyn ICustomisationBasketRepository,
    space_id: &str,
    player_id: &str,
) -> Result<(), RepositoryError> {
    repo.save(
        &CustomisationBasketState {
            player_id: player_id.to_string(),
            space_id: space_id.to_string(),
            state: serde_json::json!([]),
            version: 0,
            updated_at: time::OffsetDateTime::UNIX_EPOCH,
        },
        0,
    )
    .await
    .map(|_| ())
}

async fn journal(space_id: String, player_id: String, state: AppState) -> Response {
    evolution_journal_widget(
        Path((space_id, player_id)),
        axum::extract::Query(EvolutionJournalParams::default()),
        State(state),
    )
    .await
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::customisation_basket::{BasketVersion, ResolvedStats};
    use crate::app::players::domain::events::PlayerDomainEvent;
    use crate::app::players::domain::player::{Spp, TeamId, ValueKpo};
    use crate::app::players::domain::value_objects::{
        KpoDelta, PositionNameVo, RosterLineId, SkillId, SppAmount, StatCrans,
    };
    use crate::app::references::io::repository::in_memory_reference_repository::InMemoryReferenceRepository;
    use crate::infrastructure::players::skill_catalog_adapter::SkillCatalogAdapter;

    fn catalogue() -> SkillCatalogAdapter {
        SkillCatalogAdapter::new(std::sync::Arc::new(
            InMemoryReferenceRepository::load_for_tests(),
        ))
    }

    fn joueur() -> Player {
        let created = PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Piétaille des Carrières".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("DEMO_GRANIT__PIETAILLE".to_string()).unwrap(),
            jersey: None,
            base_skills: vec![],
            starting_spp: Spp(30),
            starting_value: ValueKpo(50),
        };
        Player::from_events(&[created]).unwrap()
    }

    /// Base du poste : MV 7, FO 3, AG 3+, PA 5+, AR 8+.
    fn panier(catalog: &dyn ISkillCatalogPort, possedees: Vec<&str>) -> CustomisationBasket {
        CustomisationBasket::hydrate(
            PlayerId("p1".into()),
            BasketVersion(3),
            vec![],
            ResolvedStats {
                ma: 7,
                st: 3,
                ag: 3,
                pa: 5,
                av: 8,
            },
            possedees
                .iter()
                .map(|s| SkillId::try_new(s.to_string()).unwrap())
                .collect(),
            catalog
                .list_all_skills()
                .into_iter()
                .filter_map(|s| SkillId::try_new(s.skill_id).ok())
                .collect(),
            ValueKpo(50),
            Spp(30),
        )
    }

    fn crans(v: i8) -> StatCrans {
        StatCrans::try_new(v).unwrap()
    }

    // ── Libellés ──────────────────────────────────────────────────────────────

    /// Le point que la maquette a mis trois tours à trouver : le libellé annonce
    /// l'offset **appliqué**, et une amélioration d'agilité vaut −1.
    #[test]
    fn le_libelle_annonce_l_offset_applique_pas_l_intention() {
        assert_eq!(stat_label(StatKind::Ag, 1), "Amélioration d'Agilité -1");
        assert_eq!(stat_label(StatKind::Ma, -1), "Dégradation de Mouvement -1");
        assert_eq!(stat_label(StatKind::St, 1), "Amélioration de Force +1");
        assert_eq!(stat_label(StatKind::Av, 1), "Amélioration d'Armure +1");
        assert_eq!(stat_label(StatKind::Pa, -1), "Dégradation de Passe +1");
    }

    #[test]
    fn les_lignes_de_prix_et_de_spp_portent_leur_signe() {
        let catalog = catalogue();
        let mut p = panier(&catalog, vec![]);
        p.adjust_price(KpoDelta::try_new(-15).unwrap()).unwrap();
        p.add_spp(SppAmount::try_new(5).unwrap()).unwrap();

        let lignes = build_pending(&p, &catalog);
        assert_eq!(lignes[0].label, "Prix -15 kPo");
        assert_eq!(lignes[0].family, "Prix");
        assert_eq!(lignes[1].label, "SPP +5");
        assert_eq!(lignes[1].family, "SPP");
    }

    // ── Caractéristiques ──────────────────────────────────────────────────────

    #[test]
    fn la_carte_montre_la_valeur_effective_et_l_offset_en_attente() {
        let catalog = catalogue();
        let mut p = panier(&catalog, vec![]);
        p.add_stat(StatKind::Ag, crans(1)).unwrap();

        let cartes = build_stats(&p);
        let ag = cartes.iter().find(|c| c.key == "ag").unwrap();

        assert_eq!(ag.current, "2+");
        assert_eq!(ag.pending_offset, Some(-1));
        // Une seconde amélioration reste possible (2+ → 1+), une dégradation aussi.
        assert_eq!(ag.preview_up.as_deref(), Some("1+"));
        assert_eq!(ag.preview_down.as_deref(), Some("3+"));
    }

    /// L'aperçu manquant **est** le grisage : la borne atteinte n'a rien à
    /// montrer.
    #[test]
    fn une_borne_atteinte_supprime_l_apercu_de_ce_cote() {
        let catalog = catalogue();
        let mut p = panier(&catalog, vec![]);
        p.add_stat(StatKind::Ag, crans(2)).unwrap();

        let cartes = build_stats(&p);
        let ag = cartes.iter().find(|c| c.key == "ag").unwrap();

        assert_eq!(ag.current, "1+");
        assert!(ag.preview_up.is_none(), "1+ est la borne haute de qualité");
        assert!(ag.preview_down.is_some());
    }

    #[test]
    fn les_cinq_caracteristiques_sont_presentes_dans_l_ordre() {
        let catalog = catalogue();
        let p = panier(&catalog, vec![]);
        let cartes = build_stats(&p);
        let cles: Vec<_> = cartes.iter().map(|c| c.key).collect();
        assert_eq!(cles, vec!["ma", "st", "ag", "pa", "av"]);
    }

    // ── Compétences ───────────────────────────────────────────────────────────

    #[test]
    fn une_competence_possedee_ou_au_panier_disparait_de_la_liste() {
        let catalog = catalogue();
        let toutes = catalog.list_all_skills();
        let possedee = toutes[0].skill_id.clone();
        let mise_au_panier = toutes[1].skill_id.clone();

        let mut p = panier(&catalog, vec![possedee.as_str()]);
        p.add_skill(SkillId::try_new(mise_au_panier.clone()).unwrap())
            .unwrap();

        let listees = build_skills(&p, &catalog);
        assert_eq!(listees.len(), toutes.len() - 2);
        assert!(!listees.iter().any(|s| s.skill_id == possedee));
        assert!(!listees.iter().any(|s| s.skill_id == mise_au_panier));
    }

    /// Le défaut que le rendu a révélé : la table disait `MUTATION` quand le
    /// référentiel dit `MUTATIONS`, et ignorait `DEVIOUS` et `TRAITS`. Trois
    /// catégories sur sept prenaient donc la pastille « général » sans que rien
    /// ne le signale. Ce test lie la table au référentiel réel — le jour où une
    /// catégorie s'ajoute, il tombe.
    #[test]
    fn chaque_categorie_du_referentiel_a_sa_propre_pastille() {
        let catalog = catalogue();
        let mut categories: Vec<String> = catalog
            .list_all_skills()
            .into_iter()
            .map(|s| s.category)
            .collect();
        categories.sort();
        categories.dedup();

        for categorie in categories {
            assert!(
                try_category_css(&categorie).is_some(),
                "la catégorie {categorie} retombe sur la pastille par défaut"
            );
        }
        assert!(try_category_css("CATEGORIE_INCONNUE").is_none());
    }

    #[test]
    fn les_competences_sont_triees_par_nom_et_portent_leur_categorie() {
        let catalog = catalogue();
        let p = panier(&catalog, vec![]);
        let listees = build_skills(&p, &catalog);

        let noms: Vec<_> = listees.iter().map(|s| s.name.clone()).collect();
        let mut tries = noms.clone();
        tries.sort();
        assert_eq!(noms, tries);
        assert!(listees.iter().all(|s| s.category_css.starts_with("type-")));
        assert!(listees.iter().all(|s| !s.category_label.is_empty()));
    }

    // ── En-tête ───────────────────────────────────────────────────────────────

    /// La réserve affichée suit le panier — sinon le commissaire ne verrait pas
    /// l'effet de son propre geste.
    #[test]
    fn la_reserve_suit_les_spp_mis_au_panier() {
        let catalog = catalogue();
        let joueur = joueur();
        let mut p = panier(&catalog, vec![]);
        assert_eq!(reserve_effective(&joueur, &p), 30);

        p.add_spp(SppAmount::try_new(7).unwrap()).unwrap();
        assert_eq!(reserve_effective(&joueur, &p), 37);
    }

    #[test]
    fn le_vm_reporte_la_version_du_panier_pour_la_garde_de_concurrence() {
        let catalog = catalogue();
        let p = panier(&catalog, vec![]);
        let vm = build_vm(&joueur(), &p, &catalog, &AppRoutes::default(), "space1");
        assert_eq!(vm.version, 3);
        assert_eq!(vm.price_kpo, 50);
        assert!(vm.pending.is_empty());
    }

    // ── Rendu ─────────────────────────────────────────────────────────────────

    /// Un panier représentatif : une ligne de chaque famille, et l'agilité
    /// poussée à sa borne pour qu'un bouton grisé apparaisse.
    fn panier_garni(catalog: &dyn ISkillCatalogPort) -> CustomisationBasket {
        let mut p = panier(catalog, vec![]);
        let premiere = catalog.list_all_skills()[0].skill_id.clone();
        p.add_skill(SkillId::try_new(premiere).unwrap()).unwrap();
        p.add_stat(StatKind::Ag, crans(2)).unwrap();
        p.adjust_price(KpoDelta::try_new(-15).unwrap()).unwrap();
        // Volontairement 7 et non 5 : à 5, le prix effectif et les SPP
        // effectifs valaient tous deux 35, et le test ne pouvait plus
        // distinguer les deux champs.
        p.add_spp(SppAmount::try_new(7).unwrap()).unwrap();
        p
    }

    /// Le template compile à la construction ; ce test vérifie qu'il **rend**,
    /// et que ce qui distingue ce panneau des deux autres est bien là. Sans
    /// lui, une erreur de rendu ne se verrait qu'en navigateur.
    ///
    /// `KREEK_DUMP_CUSTOMISATION_PREVIEW` imprime le fragment sur la sortie
    /// standard — de quoi fabriquer un aperçu autonome sans serveur ni session.
    #[test]
    fn le_panneau_rend_ses_quatre_onglets_et_son_panier() {
        let catalog = catalogue();
        let p = panier_garni(&catalog);
        let vm = build_vm(&joueur(), &p, &catalog, &AppRoutes::default(), "space1");
        let html = PlayerCustomisationTemplate { vm }.render().unwrap();

        if std::env::var("KREEK_DUMP_CUSTOMISATION_PREVIEW").is_ok() {
            println!("{html}");
        }

        // Le fragment est le conteneur : le swap est `outerHTML`.
        assert!(html.contains("id=\"pd-right-panel\""));
        assert!(html.contains("hx-disinherit=\"*\""));

        for onglet in ["Compétences", "Caractéristiques", "Prix", "SPP"] {
            assert!(html.contains(onglet), "onglet manquant : {onglet}");
        }

        // Agilité à 1+, donc « Améliorer » grisé et « Dégrader » actif.
        assert!(html.contains("disabled title=\"Borne atteinte\""));
        // Les quatre lignes du panier, dont le libellé en offset brut.
        // L'apostrophe ressort échappée : c'est Askama qui fait son travail.
        assert!(html.contains("Amélioration d&#x27;Agilité -2"));
        assert!(html.contains("Prix -15 kPo"));
        assert!(html.contains("SPP +7"));
        // Prix effectif 50 − 15, SPP effectifs 30 + 7 : deux nombres distincts,
        // faute de quoi une inversion des deux champs passerait inaperçue.
        assert!(html.contains("35 kPo"));
        assert!(html.contains(">37</div>"));
        // La version voyage avec chaque geste.
        assert!(html.contains("\"expected_version\": \"3\""));
    }
}
