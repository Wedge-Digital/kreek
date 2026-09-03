use crate::app::players::domain::player::TeamId;
use crate::app::players::ports::{IPlayerProjectionRepository, PlayerProjection};
use crate::app::teams::domain::basket::SquadPresence;
use crate::app::teams::ports::{ISquadPort, SquadMemberDto};
use async_trait::async_trait;
use std::sync::Arc;

/// L'effectif vu par `teams`, obtenu du dépôt de projection de `players`.
///
/// Il tenait un `PgPool` et interrogeait `players_proj` lui-même. C'était le
/// seul adapter du BC, sur vingt-neuf, à écrire du SQL sur la table d'un autre
/// — et sa requête était un doublon strict de `find_by_team_id`, filtre
/// d'appartenance compris. Deux lectures pour un même contrat, donc deux
/// endroits à corriger à chaque évolution : la colonne `membership` avait déjà
/// coûté sept chemins mis à jour à la main.
pub struct SquadAdapter {
    players: Arc<dyn IPlayerProjectionRepository>,
}

impl SquadAdapter {
    pub fn new(players: Arc<dyn IPlayerProjectionRepository>) -> Self {
        Self { players }
    }
}

/// La seule traduction du vocabulaire de `players` vers celui de `teams`.
///
/// Trois indisponibilités, deux traitements opposés : un blessé et un retraité
/// gardent leur place — l'un revient au match suivant (BR12), l'autre compte
/// dans les quotas toute la saison (carte 39) — quand un mort ne garde rien.
/// C'est pourquoi `teams` reçoit une présence et non un booléen : filtrer sur
/// « alignable au prochain match » aurait libéré la place des trois.
///
/// **Le défaut est `Empeche`, pas `Alignable`.** Un statut inconnu — colonne
/// non migrée, valeur écrite par une version future — doit priver l'équipe du
/// joueur, jamais lui rendre une place à tort : un plafond trop généreux se
/// solde par un effectif illégal que rien ne rattrape.
///
/// La carte 259 attendait ici une conjonction avec « membre actif ». La carte
/// 260 l'a résolue autrement, et mieux : l'appartenance filtre **la requête**,
/// pas ce prédicat. Un renvoyé n'est pas un joueur indisponible de plus — il
/// n'est plus de l'effectif, donc sa ligne n'existe pas. La conjonction
/// l'aurait laissé occuper sa place dans les quotas de poste et le plafond de
/// seize, et l'aurait affiché renvoyable une seconde fois. Cette requête est
/// désormais celle de `players` : c'est lui qui possède la colonne, et lui qui
/// la filtre.
fn presence(participation_status: &str) -> SquadPresence {
    match participation_status {
        "Available" => SquadPresence::Alignable,
        "Dead" => SquadPresence::Perdu,
        _ => SquadPresence::Empeche,
    }
}

fn to_squad_member(p: PlayerProjection) -> SquadMemberDto {
    SquadMemberDto {
        player_id: p.player_id,
        roster_line_id: p.roster_line_id,
        // Un numéro hors bornes n'est pas un numéro : mieux vaut
        // n'en afficher aucun que d'en inventer un.
        jersey: p.jersey.filter(|j| (1..=99).contains(j)).map(|j| j as u8),
        personal_name: p.personal_name,
        position_name: p.position_name,
        spp: p.spp.max(0) as u32,
        value_kpo: p.value_kpo.max(0) as u32,
        presence: presence(&p.participation_status),
    }
}

#[async_trait]
impl ISquadPort for SquadAdapter {
    async fn find_squad(&self, team_id: &str) -> Vec<SquadMemberDto> {
        let joueurs = match self
            .players
            .find_by_team_id(&TeamId(team_id.to_string()))
            .await
        {
            Ok(joueurs) => joueurs,
            // Un effectif vide et un effectif illisible se ressemblent trop :
            // le port ne rend pas de `Result`, donc sans cette ligne l'échec
            // passerait pour une équipe sans joueurs — quotas grands ouverts.
            Err(e) => {
                tracing::error!("squad_adapter: find_by_team_id {team_id}: {e}");
                return Vec::new();
            }
        };
        joueurs.into_iter().map(to_squad_member).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::io::repository::projection_repository::PgPlayerProjectionRepository;
    use sqlx::PgPool;

    #[test]
    fn seul_available_rend_un_joueur_alignable() {
        assert!(presence("Available").alignable());
        for indisponible in ["MissingNextGame", "Retired", "Dead"] {
            assert!(!presence(indisponible).alignable(), "{indisponible}");
        }
    }

    /// Le partage qui compte : trois indisponibilités, deux traitements. Un
    /// blessé et un retraité gardent leur place, un mort la rend.
    #[test]
    fn seul_le_mort_ne_garde_pas_sa_place() {
        for occupant in ["Available", "MissingNextGame", "Retired"] {
            assert!(presence(occupant).occupe_une_place(), "{occupant}");
        }
        assert!(!presence("Dead").occupe_une_place());
    }

    /// Un statut inconnu prive l'équipe du joueur plutôt que de lui rendre une
    /// place. Le sens de la prudence n'est pas neutre : un plafond trop
    /// généreux se solde par un effectif illégal que rien ne rattrape.
    #[test]
    fn un_statut_inconnu_occupe_sa_place_sans_etre_alignable() {
        let inconnu = presence("StatutQueNulNeConnait");
        assert!(inconnu.occupe_une_place());
        assert!(!inconnu.alignable());
    }

    async fn test_pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .ok()
    }

    /// L'adapter monté sur le vrai dépôt de `players`, comme en production.
    /// Le doubler ici ne prouverait plus rien : ce qu'on éprouve désormais,
    /// c'est justement que la lecture de l'autre BC rend ce qu'on attend.
    fn adapter(pool: PgPool) -> SquadAdapter {
        SquadAdapter::new(Arc::new(PgPlayerProjectionRepository::new(pool)))
    }

    async fn seed(pool: &PgPool, team_id: &str, player_id: &str, statut: &str) {
        sqlx::query(
            "INSERT INTO players_proj
                 (player_id, team_id, space_id, position_name, roster_line_id,
                  personal_name, jersey, base_skills, acquired_skills, spp,
                  value_kpo, version, participation_status)
             VALUES ($1, $2, 'space-1', 'Piétaille des Carrières',
                     'DEMO_GRANIT__PIETAILLE', 'Grumpf', 3, '[]'::jsonb,
                     '[]'::jsonb, 7, 50, 1, $3)",
        )
        .bind(player_id)
        .bind(team_id)
        .bind(statut)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn les_sept_champs_remontent() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let team_id = ulid::Ulid::new().to_string();
        let player_id = ulid::Ulid::new().to_string();
        seed(&pool, &team_id, &player_id, "Available").await;

        let effectif = adapter(pool).find_squad(&team_id).await;

        assert_eq!(effectif.len(), 1);
        let m = &effectif[0];
        assert_eq!(m.player_id, player_id);
        assert_eq!(m.roster_line_id, "DEMO_GRANIT__PIETAILLE");
        assert_eq!(m.personal_name, "Grumpf");
        assert_eq!(m.position_name, "Piétaille des Carrières");
        assert_eq!(m.spp, 7);
        assert_eq!(m.value_kpo, 50);
        assert_eq!(m.jersey, Some(3));
        assert!(m.presence.alignable());
    }

    /// Un renvoyé, lui, n'y figure plus du tout — et c'est ce qui le distingue
    /// d'un blessé. Sans cette exclusion il continuerait d'occuper sa place
    /// dans les quotas de poste, dans le plafond de seize, et dans la valeur
    /// d'équipe.
    ///
    /// Le filtre vit maintenant dans la requête de `players`. Ce test est donc
    /// ce qui vérifie que la délégation n'a rien perdu en route.
    #[tokio::test]
    async fn un_renvoye_ne_fait_plus_partie_de_l_effectif() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let team_id = ulid::Ulid::new().to_string();
        let renvoye = ulid::Ulid::new().to_string();
        seed(&pool, &team_id, &renvoye, "Available").await;
        seed(&pool, &team_id, &ulid::Ulid::new().to_string(), "Available").await;

        sqlx::query("UPDATE players_proj SET membership = 'Dismissed' WHERE player_id = $1")
            .bind(&renvoye)
            .execute(&pool)
            .await
            .unwrap();

        let effectif = adapter(pool).find_squad(&team_id).await;

        assert_eq!(effectif.len(), 1, "un seul appartient encore à l'effectif");
        assert_ne!(effectif[0].player_id, renvoye);
    }

    /// L'effectif est rendu **entier** : un blessé y figure, drapeau à faux.
    /// C'est ce qui permet au panier de recrutement de compter ses quotas
    /// sur tout l'effectif, quand la valeur d'équipe ne somme que les
    /// disponibles.
    #[tokio::test]
    async fn les_indisponibles_restent_dans_l_effectif_avec_le_drapeau_a_faux() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let team_id = ulid::Ulid::new().to_string();
        seed(&pool, &team_id, &ulid::Ulid::new().to_string(), "Available").await;
        seed(
            &pool,
            &team_id,
            &ulid::Ulid::new().to_string(),
            "MissingNextGame",
        )
        .await;

        let effectif = adapter(pool).find_squad(&team_id).await;

        assert_eq!(effectif.len(), 2, "les deux sont dans l'effectif");
        assert_eq!(
            effectif.iter().filter(|m| m.presence.alignable()).count(),
            1,
            "un seul est alignable"
        );
    }
}
