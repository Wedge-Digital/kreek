//! Le relevé de trésorerie d'une équipe (carte 435).
//!
//! Il assemble trois sources — le grand livre, l'effectif, le contexte des
//! matchs — en une vue lisible. **Aucune ligne de HTML** : la carte 436
//! n'affichera que ce que celui-ci produit, et tout le risque de la
//! fonctionnalité est ici.
//!
//! # Deux refus qui arrêtent le relevé
//!
//! Un motif illisible ou une dotation absente **interrompent** l'assemblage.
//! Sauter la ligne produirait des soldes qui ne s'enchaînent plus — un défaut
//! qui se lit comme une erreur de calcul et se cherche du mauvais côté. Un
//! relevé de compte faux est pire qu'un relevé absent.

use crate::app::teams::domain::treasury::{MovementDirection, MovementReason};
use crate::app::teams::ports::{
    IMatchContextPort, ISquadPort, ITeamRepository, MatchContextDto, TreasuryMovementRow,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

// ── La vue ────────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Eq)]
pub struct TreasuryStatement {
    /// La dotation de départ. **Pas un `Option`** : mesuré le 2026-08-30, les
    /// 8 532 équipes de la base ont un grand livre et **toutes** portent une
    /// ligne `InitialEndowment`. En faire un `Option` obligerait chaque
    /// consommateur à traiter un cas qui n'existe pas.
    pub opening: i32,
    /// **Le solde vient de la dernière ligne, jamais d'une somme.**
    /// `balance_after_kpo` est écrit dans la transaction de l'événement ; le
    /// recalculer créerait une seconde vérité, qui pourrait diverger de celle
    /// que le relevé est précisément censé rendre visible.
    pub balance: i32,
    /// L'encaissé et le dépensé, eux, **sont** des sommes : ils n'existent
    /// nulle part ailleurs.
    pub earned: i32,
    pub spent: i32,
    pub lines: Vec<StatementLine>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct StatementLine {
    pub direction: MovementDirection,
    pub amount: i32,
    pub reason: MovementReason,
    pub balance_after: i32,
    pub occurred_at: time::OffsetDateTime,
    /// Ce que la ligne raconte — « Victoire », « Apothicaire × 1 ».
    pub detail: String,
    /// Le match, quand la ligne en vient **et** qu'il est retrouvé.
    pub match_context: Option<LineMatchContext>,
}

/// Le contexte d'un match, **du point de vue de l'équipe du relevé** : le port
/// ne sait pas de quelle équipe on parle, le service le sait, et le reste de la
/// chaîne n'a plus à comparer des identifiants.
#[derive(Debug, PartialEq, Eq)]
pub struct LineMatchContext {
    pub round_name: String,
    pub opponent_name: String,
    /// Le score réordonné en **(nous, eux)**. `None` quand le match est en
    /// cours — une absence qui ne dit pas la même chose que celle du contexte
    /// entier.
    pub score: Option<(u8, u8)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TreasuryStatementError {
    MissingOpeningEntry,
    UnknownReason(String),
    Repository(String),
}

// ── L'assemblage ──────────────────────────────────────────────────────────────

// arch:no-instrument — service de lecture : assemble une vue, sans intention métier
pub async fn build_statement(
    team_id: &str,
    repo: &dyn ITeamRepository,
    squad: &dyn ISquadPort,
    match_ctx: &dyn IMatchContextPort,
) -> Result<TreasuryStatement, TreasuryStatementError> {
    let rows = repo
        .list_treasury_movements(team_id)
        .await
        .map_err(|e| TreasuryStatementError::Repository(format!("{e:?}")))?;

    let premiere = rows
        .first()
        .ok_or(TreasuryStatementError::MissingOpeningEntry)?;
    if MovementReason::parse(&premiere.reason) != Some(MovementReason::InitialEndowment) {
        return Err(TreasuryStatementError::MissingOpeningEntry);
    }

    // **Trois familles de requêtes, jamais une par ligne.** Les identifiants de
    // rapport sont collectés et dédupliqués avant d'être résolus : deux lignes
    // du même match — l'achat de coups de pouce et son remboursement — ne
    // doivent pas produire deux lectures. C'est un N+1 invisible sur un relevé
    // de six lignes, et coûteux sur une saison.
    let contextes = resoudre_les_matchs(&rows, match_ctx).await;
    let effectif: HashMap<String, String> = squad
        .find_squad(team_id)
        .await
        .into_iter()
        .map(|m| {
            (
                m.player_id,
                libelle_joueur(&m.personal_name, &m.position_name, m.jersey),
            )
        })
        .collect();

    let mut lines = Vec::with_capacity(rows.len());
    let (mut earned, mut spent) = (0, 0);
    for row in &rows {
        let reason = MovementReason::parse(&row.reason)
            .ok_or_else(|| TreasuryStatementError::UnknownReason(row.reason.clone()))?;
        let direction = MovementDirection::parse(&row.direction)
            .ok_or_else(|| TreasuryStatementError::UnknownReason(row.direction.clone()))?;

        match direction {
            MovementDirection::Credit => earned += row.amount_kpo,
            MovementDirection::Debit => spent += row.amount_kpo,
        }

        lines.push(StatementLine {
            direction,
            amount: row.amount_kpo,
            reason,
            balance_after: row.balance_after_kpo,
            occurred_at: row.occurred_at,
            detail: detail_de(reason, row.payload.as_ref(), &effectif),
            match_context: match_report_id_de(row.payload.as_ref())
                .and_then(|id| contextes.get(&id))
                .map(|ctx| vue_du_match(ctx, team_id)),
        });
    }

    Ok(TreasuryStatement {
        opening: premiere.balance_after_kpo,
        // La dernière ligne, jamais une somme.
        balance: rows.last().map(|r| r.balance_after_kpo).unwrap_or(0),
        earned,
        spent,
        lines,
    })
}

async fn resoudre_les_matchs(
    rows: &[TreasuryMovementRow],
    match_ctx: &dyn IMatchContextPort,
) -> HashMap<String, MatchContextDto> {
    let ids: HashSet<String> = rows
        .iter()
        .filter_map(|r| match_report_id_de(r.payload.as_ref()))
        .collect();

    let mut resolus = HashMap::with_capacity(ids.len());
    for id in ids {
        if let Some(ctx) = match_ctx.find_match_context(&id).await {
            resolus.insert(id, ctx);
        }
    }
    resolus
}

/// **Seuls trois événements portent un identifiant de rapport** :
/// `PostMatchSequenceReverted`, `InducementsPaid` et `InducementsRefunded`.
///
/// `PostMatchSequenceStarted` — la recette de match, la ligne la plus fréquente
/// — n'en porte **pas**. Sa ligne n'aura donc jamais de contexte de match, et
/// son détail vient de son `result`, comme la carte le prescrit. Ce n'est pas un
/// oubli d'implémentation : l'information n'existe pas dans l'événement.
fn match_report_id_de(payload: Option<&Value>) -> Option<String> {
    payload?
        .get("match_report_id")?
        .as_str()
        .map(str::to_string)
}

fn vue_du_match(ctx: &MatchContextDto, team_id: &str) -> LineMatchContext {
    let nous_recevons = ctx.home_team_id == team_id;
    LineMatchContext {
        round_name: ctx.round_name.clone(),
        opponent_name: match nous_recevons {
            true => ctx.away_team_name.clone(),
            false => ctx.home_team_name.clone(),
        },
        // Réordonné en (nous, eux). Les deux scores sont présents ou absents
        // ensemble : un match en cours n'en a aucun.
        score: match (ctx.home_score, ctx.away_score, nous_recevons) {
            (Some(h), Some(a), true) => Some((h, a)),
            (Some(h), Some(a), false) => Some((a, h)),
            _ => None,
        },
    }
}

// ── Le détail, source par source ──────────────────────────────────────────────

fn detail_de(
    reason: MovementReason,
    payload: Option<&Value>,
    effectif: &HashMap<String, String>,
) -> String {
    match reason {
        MovementReason::InitialEndowment => "Création de l'équipe".to_string(),
        MovementReason::MatchIncome => resultat_du_match(payload),
        MovementReason::MatchIncomeReverted => "Rapport de match corrigé".to_string(),
        MovementReason::InducementRefunded => "Rendus avec l'annulation du rapport".to_string(),
        MovementReason::InducementPurchase => "Coups de pouce".to_string(),
        MovementReason::PlayerRecruitment => joueur_recrute(payload, effectif),
        MovementReason::StaffPurchase => staff_achete(payload),
        MovementReason::CostlyMistake => bourde(payload),
    }
}

fn resultat_du_match(payload: Option<&Value>) -> String {
    match payload
        .and_then(|p| p.get("result"))
        .and_then(Value::as_str)
    {
        Some("Win") => "Victoire",
        Some("Draw") => "Match nul",
        Some("Loss") => "Défaite",
        _ => "Recette de match",
    }
    .to_string()
}

/// **Un joueur renvoyé perd son nom.** `ISquadPort` rend l'effectif *courant*,
/// pas l'historique : le repli sur le poste, qui vient de l'événement, vaut
/// mieux qu'une ligne muette.
fn joueur_recrute(payload: Option<&Value>, effectif: &HashMap<String, String>) -> String {
    let id = payload
        .and_then(|p| p.get("player_id"))
        .and_then(Value::as_str);
    if let Some(libelle) = id.and_then(|i| effectif.get(i)) {
        return libelle.clone();
    }
    payload
        .and_then(|p| p.get("roster_line"))
        .and_then(Value::as_str)
        .map(|l| format!("Recrue — {l}"))
        .unwrap_or_else(|| "Recrue".to_string())
}

fn libelle_joueur(nom: &str, poste: &str, maillot: Option<u8>) -> String {
    match maillot {
        Some(n) => format!("{nom}, {poste} — n° {n}"),
        None => format!("{nom}, {poste}"),
    }
}

fn staff_achete(payload: Option<&Value>) -> String {
    let poste = match payload
        .and_then(|p| p.get("staff_type"))
        .and_then(Value::as_str)
    {
        Some("Reroll") => "Relance",
        Some("Apothecary") => "Apothicaire",
        Some("Assistant") => "Assistant",
        Some("Cheerleader") => "Pom-pom girl",
        Some("FansFactor") => "Fans dévoués",
        _ => "Staff",
    };
    let quantite = payload
        .and_then(|p| p.get("quantity"))
        .and_then(Value::as_u64)
        .unwrap_or(1);
    format!("{poste} × {quantite}")
}

fn bourde(payload: Option<&Value>) -> String {
    let incident = match payload
        .and_then(|p| p.get("incident"))
        .and_then(Value::as_str)
    {
        Some("Minor") => "Incident mineur",
        Some("Major") => "Incident majeur",
        Some("Catastrophe") => "Catastrophe",
        _ => "Erreur coûteuse",
    };
    match payload.and_then(|p| p.get("roll")).and_then(Value::as_u64) {
        Some(jet) => format!("{incident} — jet de {jet}"),
        None => incident.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::teams::domain::team::{Team, TeamDomainEvent};
    use crate::app::teams::ports::{
        MyTeamRow, RepositoryError, SquadMemberDto, TeamCardRow, TeamEnrollmentRow,
    };
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Mutex;

    const EQUIPE: &str = "T1";
    const ADVERSE: &str = "T2";

    // ── Doublures ────────────────────────────────────────────────────────────

    struct FakeRepo {
        lignes: Vec<TreasuryMovementRow>,
    }

    fn ligne(
        version: i64,
        direction: &str,
        montant: i32,
        motif: &str,
        solde: i32,
        payload: Option<serde_json::Value>,
    ) -> TreasuryMovementRow {
        TreasuryMovementRow {
            event_version: version,
            direction: direction.into(),
            amount_kpo: montant,
            reason: motif.into(),
            balance_after_kpo: solde,
            occurred_at: time::macros::datetime!(2026-08-30 12:00 UTC),
            payload,
        }
    }

    fn dotation() -> TreasuryMovementRow {
        ligne(1, "Credit", 1000, "InitialEndowment", 1000, None)
    }

    #[async_trait]
    impl ITeamRepository for FakeRepo {
        async fn append(
            &self,
            _: &str,
            _: &TeamDomainEvent,
            _: u64,
        ) -> Result<u64, RepositoryError> {
            Ok(0)
        }
        async fn append_batch(
            &self,
            _: &str,
            _: &[TeamDomainEvent],
            _: u64,
        ) -> Result<u64, RepositoryError> {
            Ok(0)
        }
        async fn find_space_id(&self, _: &str) -> Result<Option<String>, RepositoryError> {
            Ok(None)
        }
        async fn find_by_id(&self, _: &str) -> Result<Option<Team>, RepositoryError> {
            Ok(None)
        }
        async fn find_by_season_and_status(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Vec<TeamEnrollmentRow>, RepositoryError> {
            Ok(Vec::new())
        }
        async fn find_enrolled_for_season(
            &self,
            _: &str,
        ) -> Result<Vec<TeamCardRow>, RepositoryError> {
            Ok(Vec::new())
        }
        async fn find_by_coach_and_space(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Vec<MyTeamRow>, RepositoryError> {
            Ok(Vec::new())
        }
        async fn list_treasury_movements(
            &self,
            _: &str,
        ) -> Result<Vec<TreasuryMovementRow>, RepositoryError> {
            Ok(self.lignes.iter().map(copie).collect())
        }
    }

    fn copie(r: &TreasuryMovementRow) -> TreasuryMovementRow {
        TreasuryMovementRow {
            event_version: r.event_version,
            direction: r.direction.clone(),
            amount_kpo: r.amount_kpo,
            reason: r.reason.clone(),
            balance_after_kpo: r.balance_after_kpo,
            occurred_at: r.occurred_at,
            payload: r.payload.clone(),
        }
    }

    struct FakeSquad {
        membres: Vec<SquadMemberDto>,
    }

    #[async_trait]
    impl ISquadPort for FakeSquad {
        async fn find_squad(&self, _: &str) -> Vec<SquadMemberDto> {
            self.membres.iter().map(membre_copie).collect()
        }
    }

    fn membre_copie(m: &SquadMemberDto) -> SquadMemberDto {
        SquadMemberDto {
            player_id: m.player_id.clone(),
            roster_line_id: m.roster_line_id.clone(),
            jersey: m.jersey,
            personal_name: m.personal_name.clone(),
            position_name: m.position_name.clone(),
            spp: m.spp,
            value_kpo: m.value_kpo,
            available_for_next_match: m.available_for_next_match,
        }
    }

    /// **Le compteur d'appels est ce qui rend le N+1 observable.** Sans lui, on
    /// pourrait affirmer que le contexte est résolu — jamais qu'il ne l'est
    /// qu'une fois.
    struct FakeMatchCtx {
        appels: Mutex<Vec<String>>,
    }

    impl FakeMatchCtx {
        fn neuf() -> Self {
            Self {
                appels: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl IMatchContextPort for FakeMatchCtx {
        async fn find_match_context(&self, match_report_id: &str) -> Option<MatchContextDto> {
            self.appels
                .lock()
                .unwrap()
                .push(match_report_id.to_string());
            Some(MatchContextDto {
                round_name: "Journée 3".into(),
                home_team_id: ADVERSE.into(),
                home_team_name: "Les Zéphyriens".into(),
                away_team_id: EQUIPE.into(),
                away_team_name: "Les Granitiers".into(),
                home_score: Some(1),
                away_score: Some(3),
            })
        }
    }

    async fn releve(
        lignes: Vec<TreasuryMovementRow>,
    ) -> Result<TreasuryStatement, TreasuryStatementError> {
        build_statement(
            EQUIPE,
            &FakeRepo { lignes },
            &FakeSquad { membres: vec![] },
            &FakeMatchCtx::neuf(),
        )
        .await
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    /// **Le solde vient de la dernière ligne, jamais d'une somme.**
    ///
    /// Les montants sont volontairement incohérents avec les soldes : une somme
    /// donnerait 1300, la dernière ligne donne 900. Un test aux chiffres
    /// cohérents passerait dans les deux cas et ne prouverait rien.
    #[tokio::test]
    async fn le_solde_est_celui_de_la_derniere_ligne() {
        let r = releve(vec![
            dotation(),
            ligne(2, "Credit", 300, "MatchIncome", 900, None),
        ])
        .await
        .unwrap();

        assert_eq!(
            r.balance, 900,
            "le solde doit venir de la ligne, pas d'une addition"
        );
        assert_eq!(r.opening, 1000);
        assert_eq!(r.earned, 1300, "l'encaissé, lui, est bien une somme");
        assert_eq!(r.spent, 0);
    }

    #[tokio::test]
    async fn un_motif_inconnu_arrete_le_releve() {
        let r = releve(vec![
            dotation(),
            ligne(2, "Credit", 10, "Pillage", 1010, None),
        ])
        .await;

        assert_eq!(
            r,
            Err(TreasuryStatementError::UnknownReason("Pillage".into()))
        );
    }

    /// Un relevé sans dotation aurait des soldes qui ne s'enchaînent pas depuis
    /// zéro : mieux vaut refuser que rendre une suite qu'on lira comme fausse.
    #[tokio::test]
    async fn une_dotation_absente_arrete_le_releve() {
        assert_eq!(
            releve(vec![]).await,
            Err(TreasuryStatementError::MissingOpeningEntry)
        );
        assert_eq!(
            releve(vec![ligne(1, "Credit", 300, "MatchIncome", 300, None)]).await,
            Err(TreasuryStatementError::MissingOpeningEntry),
            "une première ligne qui n'est pas la dotation ne vaut pas dotation"
        );
    }

    /// **Le N+1 que la carte nomme.** L'achat de coups de pouce et son
    /// remboursement portent le même rapport : deux lignes, une seule lecture.
    #[tokio::test]
    async fn deux_lignes_du_meme_match_ne_font_qu_une_lecture() {
        let ctx = FakeMatchCtx::neuf();
        let payload = Some(json!({"match_report_id": "MR1", "amount_kpo": 50}));
        let lignes = vec![
            dotation(),
            ligne(2, "Debit", 50, "InducementPurchase", 950, payload.clone()),
            ligne(3, "Credit", 50, "InducementRefunded", 1000, payload),
        ];

        build_statement(
            EQUIPE,
            &FakeRepo { lignes },
            &FakeSquad { membres: vec![] },
            &ctx,
        )
        .await
        .unwrap();

        assert_eq!(
            *ctx.appels.lock().unwrap(),
            vec!["MR1".to_string()],
            "le même rapport ne doit être lu qu'une fois"
        );
    }

    /// Le score est réordonné en **(nous, eux)** : ici l'équipe joue à
    /// l'extérieur et a marqué 3 contre 1.
    #[tokio::test]
    async fn le_score_est_reordonne_du_point_de_vue_de_l_equipe() {
        let lignes = vec![
            dotation(),
            ligne(
                2,
                "Credit",
                50,
                "InducementRefunded",
                1050,
                Some(json!({"match_report_id": "MR1"})),
            ),
        ];
        let r = releve(lignes).await.unwrap();

        let ctx = r.lines[1]
            .match_context
            .as_ref()
            .expect("le contexte doit être résolu");
        assert_eq!(ctx.score, Some((3, 1)), "nous d'abord, eux ensuite");
        assert_eq!(ctx.opponent_name, "Les Zéphyriens");
        assert_eq!(ctx.round_name, "Journée 3");
    }

    /// **Un joueur renvoyé perd son nom.** `ISquadPort` rend l'effectif
    /// *courant* : le repli sur le poste, qui vient de l'événement, vaut mieux
    /// qu'une ligne muette.
    #[tokio::test]
    async fn un_joueur_renvoye_se_replie_sur_son_poste() {
        let lignes = vec![
            dotation(),
            ligne(
                2,
                "Debit",
                80,
                "PlayerRecruitment",
                920,
                Some(json!({"player_id": "P_PARTI", "roster_line": "DEMO_GRANIT__PIETAILLE"})),
            ),
        ];

        let r = releve(lignes).await.unwrap();

        assert_eq!(r.lines[1].detail, "Recrue — DEMO_GRANIT__PIETAILLE");
    }

    /// Chaque motif dit ce qu'il raconte, et le détail vient de sa source.
    #[tokio::test]
    async fn chaque_motif_rend_son_detail() {
        let lignes = vec![
            dotation(),
            ligne(
                2,
                "Credit",
                300,
                "MatchIncome",
                1300,
                Some(json!({"result": "Win"})),
            ),
            ligne(
                3,
                "Debit",
                50,
                "StaffPurchase",
                1250,
                Some(json!({"staff_type": "Apothecary", "quantity": 1})),
            ),
            ligne(
                4,
                "Debit",
                30,
                "CostlyMistake",
                1220,
                Some(json!({"roll": 4, "incident": "Minor"})),
            ),
            ligne(
                5,
                "Debit",
                300,
                "MatchIncomeReverted",
                920,
                Some(json!({"match_report_id": "MR9"})),
            ),
        ];

        let r = releve(lignes).await.unwrap();

        let details: Vec<&str> = r.lines.iter().map(|l| l.detail.as_str()).collect();
        assert_eq!(
            details,
            vec![
                "Création de l'équipe",
                "Victoire",
                "Apothicaire × 1",
                "Incident mineur — jet de 4",
                "Rapport de match corrigé",
            ]
        );
    }

    /// **La recette de match n'a pas de contexte, et ce n'est pas un oubli.**
    ///
    /// `PostMatchSequenceStarted` ne porte pas de `match_report_id` : l'info
    /// n'existe pas dans l'événement. Son détail vient de son `result`, comme la
    /// carte le prescrit. Ce test fixe cette limite plutôt que de la laisser
    /// découvrir par quelqu'un qui la prendrait pour un défaut.
    #[tokio::test]
    async fn la_recette_de_match_n_a_pas_de_contexte_faute_d_identifiant() {
        let ctx = FakeMatchCtx::neuf();
        let lignes = vec![
            dotation(),
            ligne(
                2,
                "Credit",
                300,
                "MatchIncome",
                1300,
                Some(json!({"result": "Draw"})),
            ),
        ];

        let r = build_statement(
            EQUIPE,
            &FakeRepo { lignes },
            &FakeSquad { membres: vec![] },
            &ctx,
        )
        .await
        .unwrap();

        assert_eq!(r.lines[1].detail, "Match nul");
        assert!(r.lines[1].match_context.is_none());
        assert!(
            ctx.appels.lock().unwrap().is_empty(),
            "aucune lecture inutile"
        );
    }
}
