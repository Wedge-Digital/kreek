//! Reconstruire tout le classement d'une saison avec le barème courant.
//!
//! # Pourquoi aucun port vers `match_report`
//!
//! Une ligne de classement est **cumulative** : elle porte les totaux de
//! l'équipe après le match, pas les statistiques du match. On croirait donc
//! devoir les redemander à `match_report`. Il n'en est rien — elles se
//! retrouvent par **différence de deux lignes consécutives** de la même équipe
//! (`RankingLine::stats_between`).
//!
//! Le BC est donc autosuffisant, et le rejeu ne peut pas diverger d'un rapport
//! modifié entre-temps puisqu'il ne relit rien.

use crate::app::ranking::domain::error::DomainError;
use crate::app::ranking::domain::ranking_line::{
    CumulativeTotals, MatchContext, RankingLine, RankingRules,
};
use crate::app::ranking::ports::{
    IRankingCompetitionPort, IRankingRepository, RankingLineFullRow, RankingRepositoryError,
};
use crate::app::ranking::use_cases::record_match_ranking_use_case::to_domain_rules;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, RoundId, SeasonId};
use std::collections::HashMap;

/// Ce que le rejeu a fait.
///
/// Un compte-rendu et non `()` : « recalculé » sans chiffre ne se distingue pas
/// de « rien à recalculer », et c'est ce que l'écran devra dire.
#[derive(Debug, PartialEq, Eq)]
pub struct RecomputeReport {
    pub matches_replayed: u32,
    pub teams: u32,
}

#[derive(Debug)]
pub enum RecomputeSeasonRankingError {
    /// La saison n'a pas de barème : il n'y a rien avec quoi rejouer.
    RulesNotConfigured,
    /// Les lignes lues ne s'enchaînent pas — cumul décroissant, écart hors
    /// bornes. Le rejeu s'arrête **sans rien écrire** : mieux vaut un classement
    /// périmé qu'un classement faux.
    Inconsistent(DomainError),
    Repository(String),
}

impl From<RankingRepositoryError> for RecomputeSeasonRankingError {
    fn from(e: RankingRepositoryError) -> Self {
        Self::Repository(e.to_string())
    }
}

#[tracing::instrument(skip_all, fields(season_id = ?season_id))]
pub async fn execute(
    season_id: &SeasonId,
    repo: &dyn IRankingRepository,
    competition_port: &dyn IRankingCompetitionPort,
) -> Result<RecomputeReport, RecomputeSeasonRankingError> {
    let season = season_id.to_string();

    let rules = competition_port
        .find_ranking_rules(&season)
        .await
        .map(to_domain_rules)
        .ok_or(RecomputeSeasonRankingError::RulesNotConfigured)?;

    let anciennes = repo.find_all_lines_for_season(&season).await?;
    if anciennes.is_empty() {
        return Ok(RecomputeReport {
            matches_replayed: 0,
            teams: 0,
        });
    }

    let nouvelles =
        rejouer(&anciennes, &rules).map_err(RecomputeSeasonRankingError::Inconsistent)?;
    let equipes = compter(anciennes.iter().map(|r| r.team_id.to_string()));
    // **Des matchs, pas des lignes.** Chaque match en produit deux — une par
    // équipe — et rendre `nouvelles.len()` annoncerait le double : « recalculé
    // sur 24 matchs » pour douze. Compter les rapports distincts est exact, sans
    // supposer qu'il y en a toujours exactement deux par match.
    let matchs = compter(anciennes.iter().map(|r| r.match_report_id.to_string()));

    repo.replace_lines_for_season(&season, &nouvelles).await?;

    Ok(RecomputeReport {
        matches_replayed: matchs,
        teams: equipes,
    })
}

/// Rejoue toutes les lignes **dans l'ordre reçu**, en tenant un cumul par équipe.
///
/// L'ordre global est conservé et non regroupé par équipe : c'est lui qui sera
/// réécrit, et le réordonner ferait diverger la `sequence` du calendrier réel.
/// Le cumul, lui, est bien propre à chaque équipe — d'où la table.
fn rejouer(
    anciennes: &[RankingLineFullRow],
    rules: &RankingRules,
) -> Result<Vec<RankingLine>, DomainError> {
    let mut cumuls: HashMap<String, CumulativeTotals> = HashMap::new();
    let mut nouvelles = Vec::with_capacity(anciennes.len());

    for row in anciennes {
        let equipe = row.team_id.to_string();
        let precedent = cumuls.get(&equipe).cloned();
        let stats = RankingLine::stats_between(precedent.as_ref(), &to_domain_line(row))?;

        let ligne = RankingLine::record_match(precedent, contexte(row), stats, rules);
        cumuls.insert(equipe, totaux(&ligne));
        nouvelles.push(ligne);
    }
    Ok(nouvelles)
}

/// Le nombre de valeurs distinctes d'une suite d'identifiants.
fn compter(valeurs: impl Iterator<Item = String>) -> u32 {
    let mut vues: Vec<String> = valeurs.collect();
    vues.sort();
    vues.dedup();
    vues.len() as u32
}

/// La ligne persistée, ramenée au domaine.
///
/// **Infaillible** : les cinq identifiants ont été décodés au dépôt, une ligne
/// illisible n'arrive jamais ici. Une première version les décodait de nouveau
/// avec un repli sur un identifiant neuf — la ligne aurait été réécrite en
/// silence sous un rapport qui n'existe pas.
///
/// **Les cumuls sont repris tels quels**, y compris `wins`/`draws`/`losses` : ils
/// ne servent qu'à `stats_between`, qui n'en lit aucun. Le résultat est redérivé
/// des deux scores par `record_match` — ces colonnes sont un produit du rejeu,
/// jamais une entrée.
fn to_domain_line(row: &RankingLineFullRow) -> RankingLine {
    use crate::app::ranking::domain::ranking_line::*;
    RankingLine {
        team_id: row.team_id,
        competition_id: row.competition_id,
        season_id: row.season_id,
        round_id: row.round_id,
        match_report_id: row.match_report_id,
        recorded_at: row.recorded_at,
        matches_played: MatchesPlayed(row.matches_played),
        wins: WinCount(row.wins),
        draws: DrawCount(row.draws),
        losses: LossCount(row.losses),
        ranking_points: RankingPoints(row.ranking_points),
        bonus_points: RankingPoints(row.bonus_points),
        td_for: TdFor(row.td_for),
        td_against: TdAgainst(row.td_against),
        casualties: CasualtiesTotal(row.casualties),
        fouls: FoulsCommitted(row.fouls),
        completions: CompletionsMade(row.completions),
    }
}

/// Le contexte est **repris de la ligne d'origine** : le rejeu ne change ni la
/// journée, ni le rapport, ni l'horodatage. Seuls les cumuls sont recalculés.
fn contexte(row: &RankingLineFullRow) -> MatchContext {
    let ligne = to_domain_line(row);
    MatchContext {
        team_id: ligne.team_id,
        competition_id: ligne.competition_id,
        season_id: ligne.season_id,
        round_id: ligne.round_id,
        match_report_id: ligne.match_report_id,
        recorded_at: ligne.recorded_at,
    }
}

fn totaux(ligne: &RankingLine) -> CumulativeTotals {
    CumulativeTotals {
        matches_played: ligne.matches_played,
        wins: ligne.wins,
        draws: ligne.draws,
        losses: ligne.losses,
        ranking_points: ligne.ranking_points,
        bonus_points: ligne.bonus_points,
        td_for: ligne.td_for,
        td_against: ligne.td_against,
        casualties: ligne.casualties,
        fouls: ligne.fouls,
        completions: ligne.completions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ranking::domain::ranking_line::{
        CasualtiesInflicted, CompletionsMade, DrawCount, FoulsCommitted, MatchScore, MatchStats,
        RankingPoints, TdFor, WinCount,
    };
    use crate::app::ranking::ports::ManualPointRow;
    use crate::app::ranking::ports::{
        BonusRuleInfo, EnrolledTeamInfo, RankingGroupInfo, RankingLineRow, RankingRulesInfo,
        TiebreakSettingInfo,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use async_trait::async_trait;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // ── Doublures ────────────────────────────────────────────────────────────

    /// Elle tient les lignes **dans l'ordre d'insertion**, comme le fait
    /// `sequence` en base. C'est cet ordre que le rejeu doit respecter, et une
    /// doublure qui le perdrait ferait passer un test qui ne prouve rien.
    #[derive(Default)]
    struct FakeRepo {
        lines: Mutex<Vec<RankingLine>>,
        /// Nombre d'appels à `replace_lines_for_season` — pour prouver qu'un
        /// rejeu refusé n'écrit rien.
        remplacements: Mutex<u32>,
    }

    #[async_trait]
    impl IRankingRepository for FakeRepo {
        async fn find_latest_line(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Option<RankingLineRow>, RankingRepositoryError> {
            unimplemented!("hors du périmètre du rejeu")
        }
        async fn find_latest_lines_for_season(
            &self,
            _: &str,
        ) -> Result<Vec<RankingLineRow>, RankingRepositoryError> {
            unimplemented!("hors du périmètre du rejeu")
        }
        async fn insert_lines(&self, l: &[RankingLine]) -> Result<(), RankingRepositoryError> {
            self.lines.lock().unwrap().extend_from_slice(l);
            Ok(())
        }
        async fn delete_lines_for_match(&self, _: &str) -> Result<(), RankingRepositoryError> {
            unimplemented!("hors du périmètre du rejeu")
        }
        async fn find_all_lines_for_season(
            &self,
            _: &str,
        ) -> Result<Vec<RankingLineFullRow>, RankingRepositoryError> {
            Ok(self.lines.lock().unwrap().iter().map(to_row).collect())
        }
        async fn replace_lines_for_season(
            &self,
            _: &str,
            lines: &[RankingLine],
        ) -> Result<(), RankingRepositoryError> {
            *self.remplacements.lock().unwrap() += 1;
            *self.lines.lock().unwrap() = lines.to_vec();
            Ok(())
        }
        async fn find_manual_totals_for_season(
            &self,
            _: &str,
        ) -> Result<HashMap<String, i32>, RankingRepositoryError> {
            Ok(HashMap::new())
        }
        async fn list_manual_points(
            &self,
            _: &str,
        ) -> Result<Vec<ManualPointRow>, RankingRepositoryError> {
            Ok(Vec::new())
        }
        async fn insert_manual_points(
            &self,
            _: &str,
            _: &str,
            _: i32,
            _: Option<&str>,
            _: &str,
        ) -> Result<(), RankingRepositoryError> {
            Ok(())
        }
        async fn delete_manual_points(
            &self,
            _: i64,
            _: &str,
        ) -> Result<u64, RankingRepositoryError> {
            Ok(0)
        }
    }

    struct FakePort {
        rules: RankingRulesInfo,
    }

    #[async_trait]
    impl IRankingCompetitionPort for FakePort {
        async fn find_ranking_rules(&self, _: &str) -> Option<RankingRulesInfo> {
            Some(self.rules.clone())
        }
        async fn find_enrolled_teams(&self, _: &str) -> Vec<EnrolledTeamInfo> {
            vec![]
        }
        async fn find_groups(&self, _: &str) -> Vec<RankingGroupInfo> {
            vec![]
        }
    }

    // ── Fixtures ─────────────────────────────────────────────────────────────

    fn to_row(l: &RankingLine) -> RankingLineFullRow {
        RankingLineFullRow {
            team_id: l.team_id,
            competition_id: l.competition_id,
            season_id: l.season_id,
            round_id: l.round_id,
            match_report_id: l.match_report_id,
            recorded_at: l.recorded_at,
            matches_played: l.matches_played.0,
            wins: l.wins.0,
            draws: l.draws.0,
            losses: l.losses.0,
            ranking_points: l.ranking_points.0,
            bonus_points: l.bonus_points.0,
            td_for: l.td_for.0,
            td_against: l.td_against.0,
            casualties: l.casualties.0,
            fouls: l.fouls.0,
            completions: l.completions.0,
        }
    }

    fn bareme(victoire: u32) -> RankingRulesInfo {
        let eteint = BonusRuleInfo {
            activated: false,
            threshold: 0,
            points: 0,
        };
        RankingRulesInfo {
            win_points: victoire,
            draw_points: 1,
            lose_points: 0,
            offensive: eteint.clone(),
            defensive: eteint.clone(),
            aggressive: eteint,
            tiebreakers: vec![TiebreakSettingInfo {
                code: "nb_td".into(),
                activated: true,
            }],
        }
    }

    fn stats(own: u8, opp: u8) -> MatchStats {
        MatchStats {
            own_td: MatchScore(own),
            opponent_td: MatchScore(opp),
            casualties_inflicted: CasualtiesInflicted(1),
            fouls: FoulsCommitted(2),
            completions: CompletionsMade(3),
        }
    }

    /// **Les deux équipes d'un match partagent son rapport.** La première
    /// version de cette doublure en engendrait un par ligne : quatre lignes
    /// paraissaient alors quatre matchs, et le décompte rendu par le use case
    /// semblait juste. C'est l'écran de la carte 422 qui l'a démenti, en
    /// annonçant « 2 lignes » là où il n'y avait qu'un match.
    fn ctx(team: TeamId, season: SeasonId, rapport: MatchReportId) -> MatchContext {
        MatchContext {
            team_id: team,
            competition_id: CompetitionId::new(),
            season_id: season,
            round_id: RoundId::new(),
            match_report_id: rapport,
            recorded_at: Utc::now(),
        }
    }

    /// Deux équipes, deux journées : quatre lignes, dans l'ordre où un vrai
    /// calendrier les aurait produites — A et B pour la journée 1, puis A et B
    /// pour la journée 2. Un rejeu qui regrouperait par équipe s'en tirerait
    /// aussi bien ; c'est le test d'ordre qui l'attrape, pas celui-ci.
    fn saison(rules_info: &RankingRulesInfo) -> (FakeRepo, SeasonId, TeamId, TeamId) {
        let rules = to_domain_rules(rules_info.clone());
        let season = SeasonId::new();
        let (a, b) = (TeamId::new(), TeamId::new());

        // Deux journées, deux rapports : quatre lignes pour **deux** matchs.
        let (j1, j2) = (MatchReportId::new(), MatchReportId::new());
        let a1 = RankingLine::record_match(None, ctx(a, season, j1), stats(3, 1), &rules);
        let b1 = RankingLine::record_match(None, ctx(b, season, j1), stats(1, 3), &rules);
        let a2 =
            RankingLine::record_match(Some(totaux(&a1)), ctx(a, season, j2), stats(0, 0), &rules);
        let b2 =
            RankingLine::record_match(Some(totaux(&b1)), ctx(b, season, j2), stats(2, 0), &rules);

        let repo = FakeRepo::default();
        *repo.lines.lock().unwrap() = vec![a1, b1, a2, b2];
        (repo, season, a, b)
    }

    fn ligne_finale(repo: &FakeRepo, team: TeamId) -> RankingLine {
        repo.lines
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|l| l.team_id == team)
            .cloned()
            .expect("l'équipe doit avoir une ligne")
    }

    // ── Les scénarios ────────────────────────────────────────────────────────

    /// **Le filet du recalcul entier.** Rejouer sans changer le barème doit
    /// rendre exactement les mêmes lignes : toute divergence est un défaut du
    /// rejeu, puisque rien d'autre n'a bougé.
    #[tokio::test]
    async fn rejeu_idempotent_a_bareme_inchange() {
        let info = bareme(3);
        let (repo, season, a, b) = saison(&info);
        let avant: Vec<RankingLine> = repo.lines.lock().unwrap().clone();
        let port = FakePort { rules: info };

        let rapport = execute(&season, &repo, &port).await.expect("rejeu");

        assert_eq!(
            rapport,
            RecomputeReport {
                matches_replayed: 2,
                teams: 2
            }
        );
        let apres = repo.lines.lock().unwrap().clone();
        assert_eq!(apres.len(), avant.len());
        for (av, ap) in avant.iter().zip(apres.iter()) {
            assert_eq!(av.team_id, ap.team_id, "l'ordre des lignes a changé");
            assert_eq!(av.matches_played, ap.matches_played);
            assert_eq!(av.wins, ap.wins);
            assert_eq!(av.losses, ap.losses);
            assert_eq!(av.ranking_points, ap.ranking_points);
            assert_eq!(av.td_for, ap.td_for);
            assert_eq!(av.casualties, ap.casualties);
            assert_eq!(av.fouls, ap.fouls);
            assert_eq!(av.completions, ap.completions);
        }
        let _ = (a, b);
    }

    /// La raison d'être de la carte : le barème change, les totaux suivent.
    ///
    /// L'équipe A a gagné puis fait nul — 3+1 = 4 points à trois points la
    /// victoire, 5+1 = 6 à cinq. Les deux nombres sont **distincts**, sans quoi
    /// le test ne discriminerait pas.
    #[tokio::test]
    async fn rejeu_applique_le_nouveau_bareme() {
        let (repo, season, a, _) = saison(&bareme(3));
        assert_eq!(ligne_finale(&repo, a).ranking_points, RankingPoints(4));

        let port = FakePort { rules: bareme(5) };
        let rapport = execute(&season, &repo, &port).await.expect("rejeu");

        assert_eq!(
            rapport.matches_replayed, 2,
            "quatre lignes, mais deux matchs"
        );
        assert_eq!(
            ligne_finale(&repo, a).ranking_points,
            RankingPoints(6),
            "5 pour la victoire + 1 pour le nul"
        );
        // Les compteurs de résultat, eux, ne bougent pas : ils sont redérivés
        // des scores, que le barème ne touche pas.
        assert_eq!(ligne_finale(&repo, a).wins, WinCount(1));
        assert_eq!(ligne_finale(&repo, a).draws, DrawCount(1));
    }

    /// Une saison sans ligne n'est pas une erreur : le rapport le dit par ses
    /// chiffres, et rien n'est réécrit.
    #[tokio::test]
    async fn une_saison_vide_ne_reecrit_rien() {
        let repo = FakeRepo::default();
        let port = FakePort { rules: bareme(3) };

        let rapport = execute(&SeasonId::new(), &repo, &port).await.unwrap();

        assert_eq!(
            rapport,
            RecomputeReport {
                matches_replayed: 0,
                teams: 0
            }
        );
        assert_eq!(*repo.remplacements.lock().unwrap(), 0);
    }

    /// **Rien n'est écrit quand le rejeu échoue.** Mieux vaut un classement
    /// périmé qu'un classement faux : le use case s'arrête avant d'appeler
    /// `replace_lines_for_season`.
    #[tokio::test]
    async fn un_rejeu_incoherent_n_ecrit_rien() {
        let (repo, season, _, _) = saison(&bareme(3));
        // On casse la chaîne : la deuxième ligne de A porte un cumul inférieur
        // à la première.
        repo.lines.lock().unwrap()[2].td_for = TdFor(0);
        let port = FakePort { rules: bareme(3) };

        let issue = execute(&season, &repo, &port).await;

        assert!(
            matches!(issue, Err(RecomputeSeasonRankingError::Inconsistent(_))),
            "attendu un refus d'incohérence : {issue:?}"
        );
        assert_eq!(
            *repo.remplacements.lock().unwrap(),
            0,
            "un rejeu refusé ne doit rien réécrire"
        );
    }

    /// Sans barème, il n'y a rien avec quoi rejouer — et surtout, rien à écrire.
    #[tokio::test]
    async fn sans_bareme_le_rejeu_est_refuse() {
        struct SansRegles;
        #[async_trait]
        impl IRankingCompetitionPort for SansRegles {
            async fn find_ranking_rules(&self, _: &str) -> Option<RankingRulesInfo> {
                None
            }
            async fn find_enrolled_teams(&self, _: &str) -> Vec<EnrolledTeamInfo> {
                vec![]
            }
            async fn find_groups(&self, _: &str) -> Vec<RankingGroupInfo> {
                vec![]
            }
        }
        let (repo, season, _, _) = saison(&bareme(3));

        let issue = execute(&season, &repo, &SansRegles).await;

        assert!(matches!(
            issue,
            Err(RecomputeSeasonRankingError::RulesNotConfigured)
        ));
        assert_eq!(*repo.remplacements.lock().unwrap(), 0);
    }

    /// Le cumul est tenu **par équipe**, et l'ordre global des lignes est
    /// conservé. Une implémentation qui regrouperait par équipe avant de rejouer
    /// rendrait les mêmes totaux mais dans un ordre différent — et la `sequence`
    /// réécrite ne suivrait plus le calendrier.
    #[tokio::test]
    async fn le_rejeu_conserve_l_ordre_des_lignes() {
        let info = bareme(3);
        let (repo, season, _, _) = saison(&info);
        let ordre_avant: Vec<TeamId> = repo
            .lines
            .lock()
            .unwrap()
            .iter()
            .map(|l| l.team_id)
            .collect();
        let port = FakePort { rules: info };

        execute(&season, &repo, &port).await.unwrap();

        let ordre_apres: Vec<TeamId> = repo
            .lines
            .lock()
            .unwrap()
            .iter()
            .map(|l| l.team_id)
            .collect();
        assert_eq!(ordre_avant, ordre_apres, "A, B, A, B — pas A, A, B, B");
    }
}
