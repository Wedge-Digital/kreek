use crate::app::teams::ports::{ISquadPort, SquadMemberDto};
use async_trait::async_trait;
use sqlx::PgPool;

pub struct SquadAdapter {
    pool: PgPool,
}

impl SquadAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Seul `Available` rend un joueur alignable ; `MissingNextGame`, `Retired` et
/// `Dead` ne le sont pas. C'est ici, et nulle part ailleurs, que le vocabulaire
/// de `players` est traduit — `teams` ne connaît que le booléen.
///
/// **Carte 260** : un joueur devra aussi être *membre actif* de l'effectif. La
/// conjonction se pose ici, dès que `players_proj.membership` existe.
fn is_available(participation_status: &str) -> bool {
    participation_status == "Available"
}

type LigneEffectif = (
    String,
    String,
    Option<i16>,
    String,
    String,
    i32,
    i32,
    String,
);

#[async_trait]
impl ISquadPort for SquadAdapter {
    async fn find_squad(&self, team_id: &str) -> Vec<SquadMemberDto> {
        let rows: Vec<LigneEffectif> = sqlx::query_as(
            "SELECT player_id, roster_line_id, jersey, personal_name, position_name,
                    spp, value_kpo, participation_status
             FROM players_proj WHERE team_id = $1
             ORDER BY jersey NULLS LAST, player_id",
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows.into_iter()
            .map(
                |(
                    player_id,
                    roster_line_id,
                    jersey,
                    personal_name,
                    position_name,
                    spp,
                    value,
                    statut,
                )| {
                    SquadMemberDto {
                        player_id,
                        roster_line_id,
                        // Un numéro hors bornes n'est pas un numéro : mieux vaut
                        // n'en afficher aucun que d'en inventer un.
                        jersey: jersey.filter(|j| (1..=99).contains(j)).map(|j| j as u8),
                        personal_name,
                        position_name,
                        spp: spp.max(0) as u32,
                        value_kpo: value.max(0) as u32,
                        available_for_next_match: is_available(&statut),
                    }
                },
            )
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seul_available_rend_un_joueur_disponible() {
        assert!(is_available("Available"));
        for indisponible in ["MissingNextGame", "Retired", "Dead"] {
            assert!(!is_available(indisponible), "{indisponible}");
        }
    }

    async fn test_pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .ok()
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

        let effectif = SquadAdapter::new(pool).find_squad(&team_id).await;

        assert_eq!(effectif.len(), 1);
        let m = &effectif[0];
        assert_eq!(m.player_id, player_id);
        assert_eq!(m.roster_line_id, "DEMO_GRANIT__PIETAILLE");
        assert_eq!(m.personal_name, "Grumpf");
        assert_eq!(m.position_name, "Piétaille des Carrières");
        assert_eq!(m.spp, 7);
        assert_eq!(m.value_kpo, 50);
        assert_eq!(m.jersey, Some(3));
        assert!(m.available_for_next_match);
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

        let effectif = SquadAdapter::new(pool).find_squad(&team_id).await;

        assert_eq!(effectif.len(), 2, "les deux sont dans l'effectif");
        assert_eq!(
            effectif
                .iter()
                .filter(|m| m.available_for_next_match)
                .count(),
            1,
            "un seul est alignable"
        );
    }
}
