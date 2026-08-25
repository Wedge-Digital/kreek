//! Carte 388 — recalculer toutes les valeurs d'équipe.
//!
//! # Rien à corriger équipe par équipe
//!
//! La valeur d'équipe est **une somme recalculée, jamais une accumulation de
//! deltas**. Il suffit donc de la recalculer, par le mécanisme nominal.
//!
//! Toutes les équipes, et pas seulement celles des rosters à vil prix : placée
//! après la migration du bonus Élite, elle rattrape du même coup les valeurs
//! joueurs que celle-ci vient de corriger.
//!
//! # Sa garde, et pourquoi elle est indispensable
//!
//! Cette migration est **one-shot** : marquée en base une fois faite, plus rien
//! ne repasse derrière. Si le corpus de production ne portait pas encore
//! `LOW_COST_LINEMEN` au premier démarrage du nouveau binaire, elle
//! recalculerait toutes les valeurs **sans** la règle, se marquerait comme
//! appliquée, et laisserait des valeurs fausses définitivement — sans une ligne
//! de journal pour le dire.
//!
//! Elle vérifie donc qu'au moins un roster la porte, et refuse le démarrage
//! sinon. C'est la même exigence que les champs obligatoires du barème Élite :
//! une règle inactive doit se voir.
//!
//! # Elle n'est pas atomique avec sa marque, et c'est assumé
//!
//! Elle passe par `recompute_team_value_use_case`, qui ouvre sa propre
//! transaction par équipe — c'est ce qui en fait le mécanisme *nominal* plutôt
//! qu'une écriture parallèle à réinventer. La règle d'atomicité de la carte 386
//! y perd donc son effet.
//!
//! Sans conséquence ici : l'événement appendu porte une valeur **absolue**, pas
//! un delta. Une interruption au milieu laisse des équipes recalculées et
//! d'autres non ; le rejeu au démarrage suivant repasse sur toutes, et
//! recalculer une valeur déjà juste la laisse juste. C'est précisément ce qu'un
//! delta ne permettrait pas.

use crate::app::teams::use_cases::recompute_team_value_use_case;
use crate::infrastructure::data_migrations::DataMigration;
use crate::state::AppState;
use async_trait::async_trait;
use sqlx::{Postgres, Row, Transaction};

/// Cf. `roster_catalog_adapter` — l'uid vit là-bas pour le domaine, ici pour la
/// garde. Les deux visent le même fait du corpus.
const LOW_COST_LINEMEN: &str = "LOW_COST_LINEMEN";

/// Refuse le démarrage si aucun roster ne porte la règle.
///
/// Extraite de la migration pour être exerçable : c'est le cas d'**échec** qui
/// compte, et il ne se constate pas sur le corpus d'exemple, qui porte la
/// règle. Une garde dont on ne teste que la branche passante ne garde rien.
fn garde<'a>(rosters: impl Iterator<Item = &'a [String]>) -> Result<(), String> {
    let porteurs = rosters
        .filter(|regles| regles.iter().any(|r| r == LOW_COST_LINEMEN))
        .count();
    if porteurs == 0 {
        return Err(format!(
            "aucun roster du corpus ne porte « {LOW_COST_LINEMEN} » — \
             recalculer les valeurs d'équipe sans la règle les figerait fausses, \
             définitivement. Poser la règle sur les rosters concernés avant de \
             déployer."
        ));
    }
    Ok(())
}

pub struct RecalculValeursEquipe;

#[async_trait]
impl DataMigration for RecalculValeursEquipe {
    fn nom(&self) -> &'static str {
        "388-recalcul-valeurs-equipe"
    }

    async fn executer(
        &self,
        state: &AppState,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<usize, String> {
        garde(
            state
                .references
                .repository
                .list_teams()
                .iter()
                .map(|t| t.special_rules.as_slice()),
        )?;

        let equipes: Vec<String> = sqlx::query("SELECT team_id FROM team_proj ORDER BY team_id")
            .fetch_all(&mut **tx)
            .await
            .map_err(|e| format!("lecture des équipes : {e}"))?
            .into_iter()
            .map(|r| r.get("team_id"))
            .collect();

        for team_id in &equipes {
            recompute_team_value_use_case::execute(
                team_id,
                state.teams.team_repository.as_ref(),
                state.teams.squad_port.as_ref(),
                state.teams.roster_catalog_port.as_ref(),
                state.teams.journeyman_type_port.as_ref(),
            )
            .await
            .map_err(|e| format!("recalcul de {team_id} : {e:?}"))?;
        }
        Ok(equipes.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::data_migrations::appliquer;

    async fn etat(pool: sqlx::PgPool) -> AppState {
        crate::compose(crate::config::AppConfig::for_tests(), pool).await
    }

    /// Le cas qui compte : sans la règle au corpus, la migration refuse de
    /// commencer plutôt que de figer des valeurs fausses.
    #[test]
    fn sans_la_regle_au_corpus_la_garde_refuse() {
        let sans: Vec<Vec<String>> = vec![vec!["BRAWLIN_BRUTES".to_string()], vec![]];
        let e =
            garde(sans.iter().map(|v| v.as_slice())).expect_err("aucun roster ne porte la règle");
        assert!(
            e.contains(LOW_COST_LINEMEN),
            "le message doit nommer la règle"
        );
        assert!(
            e.contains("avant de déployer"),
            "le message doit dire quoi faire : {e}"
        );
    }

    #[test]
    fn un_seul_roster_porteur_suffit() {
        let avec: Vec<Vec<String>> = vec![
            vec!["BRAWLIN_BRUTES".to_string()],
            vec![LOW_COST_LINEMEN.to_string()],
        ];
        assert!(garde(avec.iter().map(|v| v.as_slice())).is_ok());
    }

    /// Le corpus d'exemple porte la règle sur les Lanterniers : la garde passe,
    /// et une base sans équipe se migre sans rien faire.
    #[sqlx::test]
    async fn la_garde_passe_sur_le_corpus_d_exemple(pool: sqlx::PgPool) {
        let state = etat(pool.clone()).await;

        appliquer(&state, &pool, vec![Box::new(RecalculValeursEquipe)])
            .await
            .expect("le corpus d'exemple porte la règle");

        let marques: Vec<String> = sqlx::query_scalar("SELECT name FROM applied_data_migrations")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(marques, vec!["388-recalcul-valeurs-equipe"]);
    }
}
