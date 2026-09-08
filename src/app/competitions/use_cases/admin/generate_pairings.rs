use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::competitions::domain::group_repository_port::{GroupWithTeams, IGroupRepository};
use crate::app::competitions::domain::match_day::{MatchDay, Pairing};
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, MatchDayRepositoryError, NewPairingProjection,
};
use crate::app::competitions::domain::tirage::{
    paire, tirer, DrawInput, NombreDeMatchs, ProposedPairing, RencontresJouees,
};
use crate::app::competitions::ports::{ITeamInfoPort, TeamInfoDto};
use crate::app::competitions::use_cases::admin::team_enrollment::{
    build_new_pairing_projection, filter_enrolled_team_ids, load_enrolled_teams, resolve_team_names,
};
use crate::app::shared_kernel::bloodbowl::ids::PairingId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::EventId;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub enum GenerateError {
    MatchDayNotFound,
    IsRestDay,
    PairingsAlreadyExist,
    NoGroups,
    Repository(String),
}

/// `skipped_team_names` : équipes présentes dans un groupe/poule mais non
/// `Enrolled` pour la saison — jamais appariées (BR : pas de pairing pour une
/// équipe non enrôlée, donc pas d'event `PairingCreated` la concernant).
/// `skipped_group_names` : poules avec moins de 2 équipes enrôlées assignées
/// — aucun appariement possible, sans quoi la génération réussirait
/// silencieusement à 0 rencontre pour cette poule (BR : signaler explicitement
/// plutôt que de laisser l'admin croire que la génération a échoué).
/// `unproven_group_names` : poules dont le tirage a épuisé son budget
/// d'exploration. L'appariement rendu reste **maximal** — personne n'est laissé
/// sur le banc sans raison — mais rien ne prouve que le départage des revanches
/// soit le meilleur. Signalé plutôt que tu, pour la même raison que les deux
/// autres champs : l'admin doit savoir ce que la génération a concédé.
#[derive(Debug, Default)]
pub struct GenerateOutcome {
    pub skipped_team_names: Vec<String>,
    pub skipped_group_names: Vec<String>,
    pub unproven_group_names: Vec<String>,
}

#[tracing::instrument(skip_all, fields(match_day_id = ?match_day_id))]
pub async fn execute(
    match_day_id: &str,
    season_id: &str,
    competition_id: &str,
    space_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
    group_repo: &dyn IGroupRepository,
    team_port: &dyn ITeamInfoPort,
    event_bus: &EventBus,
) -> Result<GenerateOutcome, GenerateError> {
    let match_day = match_day_repo
        .find_by_id(match_day_id)
        .await
        .map_err(|e| GenerateError::Repository(e.to_string()))?
        .ok_or(GenerateError::MatchDayNotFound)?;

    if match_day.is_rest() {
        return Err(GenerateError::IsRestDay);
    }
    if !match_day.pairings.is_empty() {
        return Err(GenerateError::PairingsAlreadyExist);
    }

    let team_display = load_enrolled_teams(season_id, team_port)
        .await
        .map_err(GenerateError::Repository)?;
    let groups = load_groups(season_id, group_repo, &team_display).await?;

    let all_days = match_day_repo
        .find_by_season(season_id)
        .await
        .map_err(|e| GenerateError::Repository(e.to_string()))?;
    let mut historique = build_historique(&all_days, match_day_id);
    let matchs_joues = build_matchs_joues(&all_days, match_day_id);

    // `from_os_rng()` comme `random_draw.rs`, et créé **une fois** : un
    // générateur par poule reproduirait le même état pour chacune.
    let mut rng = StdRng::from_os_rng();
    let ctx = Contexte {
        match_day: &match_day,
        competition_id,
        season_id,
        space_id,
        team_display: &team_display,
        matchs_joues: &matchs_joues,
        match_day_repo,
        event_bus,
    };

    let mut outcome = GenerateOutcome::default();
    let mut skipped_team_ids: Vec<String> = Vec::new();
    let mut a_ecrire: Vec<AppariementAEcrire> = Vec::new();
    for group in &groups {
        let (filtered_ids, skipped) = filter_enrolled_team_ids(&group.team_ids, &team_display);
        skipped_team_ids.extend(skipped);
        if filtered_ids.len() < 2 {
            outcome.skipped_group_names.push(group.group_name.clone());
            continue;
        }
        let (paires, prouve) =
            apparier_le_groupe(&ctx, &filtered_ids, &mut historique, &mut rng).await?;
        a_ecrire.extend(paires);
        if !prouve {
            outcome.unproven_group_names.push(group.group_name.clone());
        }
    }

    ecrire_la_journee(&ctx, &a_ecrire).await?;

    outcome.skipped_team_names = resolve_team_names(skipped_team_ids, team_port).await;
    Ok(outcome)
}

/// Écrit toutes les rencontres de la journée, **puis** les annonce.
///
/// L'émission vient après le commit, jamais dedans : un listener qui réagit à
/// un `PairingCreated` dont la transaction est ensuite annulée aurait travaillé
/// sur un fait qui n'a pas eu lieu, et rien ne le lui dirait. L'ordre inverse
/// paraît plus naturel — « tout dans la même unité » — et c'est le piège.
async fn ecrire_la_journee(
    ctx: &Contexte<'_>,
    a_ecrire: &[AppariementAEcrire],
) -> Result<(), GenerateError> {
    if a_ecrire.is_empty() {
        return Ok(());
    }
    ctx.match_day_repo
        .save_pairings(&ctx.match_day.id.to_string(), a_ecrire)
        .await
        .map_err(|e| match e {
            MatchDayRepositoryError::PairingsAlreadyExist => GenerateError::PairingsAlreadyExist,
            autre => GenerateError::Repository(autre.to_string()),
        })?;

    for (pairing, _) in a_ecrire {
        emit_pairing_created(
            ctx,
            &pairing.home_team_id.to_string(),
            &pairing.away_team_id.to_string(),
            pairing,
        );
    }
    Ok(())
}

/// Ce que l'écriture d'une rencontre demande, et qui ne change pas d'une poule
/// à l'autre. Rassemblé pour que les fonctions qui suivent tiennent en une
/// poignée de paramètres plutôt qu'en dix.
struct Contexte<'a> {
    match_day: &'a MatchDay,
    competition_id: &'a str,
    season_id: &'a str,
    space_id: &'a str,
    team_display: &'a HashMap<String, TeamInfoDto>,
    /// R9 — en lecture seule, contrairement à `historique` qui voyage en `&mut` :
    /// les poules étant disjointes, aucune équipe n'apparaît dans deux groupes,
    /// donc rien n'est à réactualiser en cours de journée.
    matchs_joues: &'a HashMap<TeamId, NombreDeMatchs>,
    match_day_repo: &'a dyn IMatchDayRepository,
    event_bus: &'a EventBus,
}

async fn load_groups(
    season_id: &str,
    group_repo: &dyn IGroupRepository,
    team_display: &HashMap<String, TeamInfoDto>,
) -> Result<Vec<GroupWithTeams>, GenerateError> {
    let groups = group_repo
        .find_groups(season_id)
        .await
        .map_err(|e| GenerateError::Repository(e.to_string()))?;

    if !groups.is_empty() {
        return Ok(groups);
    }
    if team_display.is_empty() {
        return Err(GenerateError::NoGroups);
    }
    Ok(vec![GroupWithTeams {
        group_id: "default".to_string(),
        group_name: "Toutes les équipes".to_string(),
        position: 0,
        team_ids: team_display.keys().cloned().collect(),
    }])
}

/// Rend `false` quand le tirage n'a pas pu prouver son optimum.
/// Rend les appariements de la poule — **sans rien écrire** — et si le tirage
/// a pu prouver son optimum.
///
/// L'écriture attend d'avoir toutes les poules : une transaction par poule
/// laisserait encore une journée à moitié appariée si la troisième échouait.
type AppariementAEcrire = (Pairing, NewPairingProjection);

async fn apparier_le_groupe(
    ctx: &Contexte<'_>,
    team_ids: &[String],
    historique: &mut RencontresJouees,
    rng: &mut StdRng,
) -> Result<(Vec<AppariementAEcrire>, bool), GenerateError> {
    let equipes: Vec<TeamId> = team_ids
        .iter()
        .map(|id| TeamId::try_new(id).expect("valid team id"))
        .collect();

    let proposition = tirer(
        &DrawInput {
            interdites: build_interdites(&equipes, ctx.team_display),
            equipes,
            historique: historique.clone(),
            // R9 s'applique bien ici, et c'est nouveau (carte 541). Le critère
            // précédent — exempter parmi celles qui ne l'ont jamais été —
            // demandait une mémoire des exemptions que le Calendrier ne tenait
            // pas ; il recevait donc un ensemble vide et ne départageait rien.
            // Celui-ci se lit sur les appariements déjà écrits.
            matchs_joues: ctx.matchs_joues.clone(),
        },
        rng,
    );

    let ecrites = proposition
        .rencontres
        .iter()
        .map(|r| batir_appariement(ctx, r, historique))
        .collect();
    Ok((ecrites, proposition.optimum_prouve))
}

/// R10 — deux équipes d'un même coach ne se rencontrent jamais.
///
/// La règle est métier, sa **matière** est inter-BC : la relation équipe → coach
/// arrive par `ITeamInfoPort`. Le domaine reçoit des paires interdites, il n'a
/// pas à savoir qu'un coach existe.
fn build_interdites(
    equipes: &[TeamId],
    team_display: &HashMap<String, TeamInfoDto>,
) -> HashSet<(TeamId, TeamId)> {
    let mut interdites = HashSet::new();
    for (rang, a) in equipes.iter().enumerate() {
        for b in equipes.iter().skip(rang + 1) {
            if meme_coach(a, b, team_display) {
                interdites.insert(paire(a, b));
            }
        }
    }
    interdites
}

fn meme_coach(a: &TeamId, b: &TeamId, team_display: &HashMap<String, TeamInfoDto>) -> bool {
    match (
        team_display.get(&a.to_string()),
        team_display.get(&b.to_string()),
    ) {
        (Some(x), Some(y)) => x.coach_id == y.coach_id,
        _ => false,
    }
}

fn batir_appariement(
    ctx: &Contexte<'_>,
    rencontre: &ProposedPairing,
    historique: &mut RencontresJouees,
) -> AppariementAEcrire {
    let (home, away) = (rencontre.home.to_string(), rencontre.away.to_string());
    let pairing = Pairing {
        id: PairingId::new(),
        home_team_id: rencontre.home,
        away_team_id: rencontre.away,
    };
    let projection =
        build_new_pairing_projection(&home, &away, ctx.season_id, ctx.match_day, ctx.team_display);

    // Les poules étant disjointes, cela ne change rien aujourd'hui — mais une
    // rencontre appariée est une rencontre à venir, et l'historique doit le
    // dire à la poule suivante.
    historique.enregistrer(
        &rencontre.home,
        &rencontre.away,
        ctx.match_day.position,
        ctx.match_day.name.clone(),
    );
    (pairing, projection)
}

/// L'historique de la saison, **compté** par paire, la journée en cours exclue.
///
/// Un `HashSet` ne disait que « déjà jouée » : c'est ce choix de type qui
/// rendait la minimisation de R8.2 impossible.
fn build_historique(days: &[MatchDay], exclude_id: &str) -> RencontresJouees {
    let mut historique = RencontresJouees::new();
    for day in days.iter().filter(|d| d.id.to_string() != exclude_id) {
        for p in &day.pairings {
            historique.enregistrer(
                &p.home_team_id,
                &p.away_team_id,
                day.position,
                day.name.clone(),
            );
        }
    }
    historique
}

/// R9 — le nombre d'appariements **programmés** de chaque équipe sur la saison.
///
/// Même boucle que `build_historique`, même exclusion de la journée tirée : les
/// appariements qu'on s'apprête à écrire ne comptent pas contre les équipes
/// qu'ils concernent. Une fonction séparée plutôt qu'un tuple — chacune reste
/// courte et porte son nom.
///
/// Les appariements et non les rapports de match : ils vivent dans les tables du
/// BC, donc le compte se lit sans port.
fn build_matchs_joues(days: &[MatchDay], exclude_id: &str) -> HashMap<TeamId, NombreDeMatchs> {
    let mut comptes: HashMap<TeamId, NombreDeMatchs> = HashMap::new();
    for day in days.iter().filter(|d| d.id.to_string() != exclude_id) {
        for p in &day.pairings {
            for camp in [p.home_team_id, p.away_team_id] {
                comptes.entry(camp).or_default().0 += 1;
            }
        }
    }
    comptes
}

fn emit_pairing_created(ctx: &Contexte<'_>, home: &str, away: &str, pairing: &Pairing) {
    // Invariant garanti par le filtrage fait avant l'appel au tirage :
    // home/away ne peuvent être ici que des ids déjà présents dans team_display.
    let home_info = ctx
        .team_display
        .get(home)
        .expect("home team filtré comme enrôlé avant appariement");
    let away_info = ctx
        .team_display
        .get(away)
        .expect("away team filtré comme enrôlé avant appariement");
    let (competition_id, season_id, space_id) = (ctx.competition_id, ctx.season_id, ctx.space_id);
    let match_day = ctx.match_day;

    emettre(
        ctx.event_bus,
        CompetitionsDomainEvent::PairingCreated {
            event_id: EventId::new(),
            pairing_id: pairing.id.to_string(),
            competition_id: competition_id.to_string(),
            season_id: season_id.to_string(),
            round_id: match_day.id.to_string(),
            home_team_id: home.to_string(),
            away_team_id: away.to_string(),
            space_id: space_id.to_string(),
            home_team_name: home_info.team_name.clone(),
            home_roster_name: home_info.roster_name.clone(),
            home_coach_name: home_info.coach_name.clone(),
            home_logo_url: home_info.logo_url.clone(),
            away_team_name: away_info.team_name.clone(),
            away_roster_name: away_info.roster_name.clone(),
            away_coach_name: away_info.coach_name.clone(),
            away_logo_url: away_info.logo_url.clone(),
            round_name: match_day.name.to_string(),
            round_position: match_day.position.into_inner(),
            round_date_start: match_day.date_start.as_ref().map(|d| d.to_string()),
            round_date_end: match_day.date_end.as_ref().map(|d| d.to_string()),
            round_day_type: match_day.day_type.as_str().to_string(),
        }
        .to_enveloppe(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::group_repository_port::GroupRepositoryError;
    use crate::app::competitions::domain::match_day::{
        MatchDayName, MatchDayPosition, MatchDayType,
    };
    use crate::app::competitions::domain::match_day_repository_port::{
        MatchDayRepositoryError, PairingDisplayDto,
    };
    use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
    use async_trait::async_trait;

    struct FakeMatchDayRepo(MatchDay, std::sync::Mutex<Vec<(String, String)>>, bool);

    impl FakeMatchDayRepo {
        fn new(day: MatchDay) -> Self {
            Self(day, std::sync::Mutex::new(vec![]), false)
        }
        /// Un dépôt qui refuse d'écrire — pour éprouver ce qui se passe *après*
        /// l'échec, et notamment ce qui n'est pas émis.
        fn en_panne(day: MatchDay) -> Self {
            Self(day, std::sync::Mutex::new(vec![]), true)
        }
        /// Les appariements écrits, normalisés et triés — de quoi comparer deux
        /// générations sans dépendre de l'ordre.
        fn ecrits(&self) -> Vec<(String, String)> {
            let mut v: Vec<(String, String)> = self
                .1
                .lock()
                .expect("mutex de test")
                .iter()
                .map(|(a, b)| {
                    if a <= b {
                        (a.clone(), b.clone())
                    } else {
                        (b.clone(), a.clone())
                    }
                })
                .collect();
            v.sort();
            v
        }
    }
    #[async_trait]
    impl IMatchDayRepository for FakeMatchDayRepo {
        async fn find_by_season(&self, _: &str) -> Result<Vec<MatchDay>, MatchDayRepositoryError> {
            Ok(vec![self.0.clone()])
        }
        async fn find_by_id(&self, _: &str) -> Result<Option<MatchDay>, MatchDayRepositoryError> {
            Ok(Some(self.0.clone()))
        }
        async fn save_match_day(&self, _: &MatchDay) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn delete_match_day(&self, _: &str) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn save_pairings(
            &self,
            _: &str,
            pairings: &[(
                Pairing,
                crate::app::competitions::domain::match_day_repository_port::NewPairingProjection,
            )],
        ) -> Result<(), MatchDayRepositoryError> {
            if self.2 {
                return Err(MatchDayRepositoryError::Database("panne simulée".into()));
            }
            let mut ecrits = self.1.lock().expect("mutex de test");
            for (pairing, _) in pairings {
                ecrits.push((
                    pairing.home_team_id.to_string(),
                    pairing.away_team_id.to_string(),
                ));
            }
            Ok(())
        }
        async fn save_pairing(
            &self,
            _: &str,
            pairing: &Pairing,
            _: &crate::app::competitions::domain::match_day_repository_port::NewPairingProjection,
        ) -> Result<(), MatchDayRepositoryError> {
            self.1.lock().expect("mutex de test").push((
                pairing.home_team_id.to_string(),
                pairing.away_team_id.to_string(),
            ));
            Ok(())
        }
        async fn find_pairing_id(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<Option<String>, MatchDayRepositoryError> {
            Ok(None)
        }
        async fn delete_pairing(&self, _: &str) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn ensure_match_days_from_structure(
            &self,
            _: &str,
            _: &[(String, String, String, Option<String>, Option<String>)],
        ) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn list_resultats(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn list_calendrier(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn list_team_matches(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn list_latest_completed_results(
            &self,
            _: &str,
            _: i64,
        ) -> Result<
            Vec<crate::app::competitions::domain::match_day_repository_port::LatestResultDto>,
            MatchDayRepositoryError,
        > {
            Ok(vec![])
        }
    }

    struct FakeGroupRepo;
    #[async_trait]
    impl IGroupRepository for FakeGroupRepo {
        async fn find_groups(&self, _: &str) -> Result<Vec<GroupWithTeams>, GroupRepositoryError> {
            Ok(vec![])
        }
        async fn save_assignments(
            &self,
            _: &[(String, String)],
        ) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn reset_assignments(&self, _: &str) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn assign_team(&self, _: &str, _: &str) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn unassign_team(&self, _: &str) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn ensure_groups_from_structure(
            &self,
            _: &str,
            _: &[(String, String)],
        ) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
    }

    struct FakeTeamInfoPort;
    #[async_trait]
    impl ITeamInfoPort for FakeTeamInfoPort {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
        }
        async fn find_team_names(&self, _: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
        }
        async fn find_team_enrollment(
            &self,
            _: &str,
        ) -> Result<Option<crate::app::competitions::ports::TeamEnrollmentDto>, String> {
            Ok(None)
        }
    }

    struct FakeGroupRepoWithEmptyGroup(&'static str);
    #[async_trait]
    impl IGroupRepository for FakeGroupRepoWithEmptyGroup {
        async fn find_groups(&self, _: &str) -> Result<Vec<GroupWithTeams>, GroupRepositoryError> {
            Ok(vec![GroupWithTeams {
                group_id: "g1".to_string(),
                group_name: self.0.to_string(),
                position: 0,
                team_ids: vec![],
            }])
        }
        async fn save_assignments(
            &self,
            _: &[(String, String)],
        ) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn reset_assignments(&self, _: &str) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn assign_team(&self, _: &str, _: &str) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn unassign_team(&self, _: &str) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn ensure_groups_from_structure(
            &self,
            _: &str,
            _: &[(String, String)],
        ) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
    }

    struct FakeTeamInfoPortWithEnrolled(Vec<TeamInfoDto>);
    #[async_trait]
    impl ITeamInfoPort for FakeTeamInfoPortWithEnrolled {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(self.0.clone())
        }
        async fn find_team_names(&self, _: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
        }
        async fn find_team_enrollment(
            &self,
            _: &str,
        ) -> Result<Option<crate::app::competitions::ports::TeamEnrollmentDto>, String> {
            Ok(None)
        }
    }

    fn match_day_with_pairings(pairings: Vec<Pairing>) -> MatchDay {
        MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("Journée 1".to_string()).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(0).unwrap(),
            pairings,
        }
    }

    // ── R9 — le compte qui alimente le critère d'exemption (carte 541) ───────

    fn appariement(home: &TeamId, away: &TeamId) -> Pairing {
        Pairing {
            id: PairingId::new(),
            home_team_id: *home,
            away_team_id: *away,
        }
    }

    #[test]
    fn build_matchs_joues_compte_les_deux_camps() {
        let (a, b, c) = (TeamId::new(), TeamId::new(), TeamId::new());
        let jours = vec![
            match_day_with_pairings(vec![appariement(&a, &b)]),
            match_day_with_pairings(vec![appariement(&a, &c)]),
        ];

        let comptes = build_matchs_joues(&jours, "aucune");

        assert_eq!(comptes.get(&a), Some(&NombreDeMatchs(2)));
        assert_eq!(comptes.get(&b), Some(&NombreDeMatchs(1)));
        assert_eq!(comptes.get(&c), Some(&NombreDeMatchs(1)));
    }

    /// Les appariements qu'on s'apprête à écrire ne comptent pas contre les
    /// équipes qu'ils concernent — même exclusion que `build_historique`.
    #[test]
    fn build_matchs_joues_ignore_la_journee_tiree() {
        let (a, b) = (TeamId::new(), TeamId::new());
        let jours = vec![
            match_day_with_pairings(vec![appariement(&a, &b)]),
            match_day_with_pairings(vec![appariement(&a, &b)]),
        ];
        let tiree = jours[1].id.to_string();

        let comptes = build_matchs_joues(&jours, &tiree);

        assert_eq!(comptes.get(&a), Some(&NombreDeMatchs(1)));
    }

    /// Le défaut que la 541 corrige : le champ arrivait vide au tirage, donc le
    /// critère ne départageait rien. Ce test échouerait sur le code d'avant.
    #[test]
    fn une_saison_vierge_ne_donne_aucun_compte() {
        let comptes = build_matchs_joues(&[], "aucune");

        assert!(
            comptes.is_empty(),
            "aucune journée, aucun match — et toutes les équipes à égalité"
        );
    }

    #[tokio::test]
    async fn refuses_when_match_day_already_has_pairings() {
        let existing = Pairing {
            id: PairingId::new(),
            home_team_id: TeamId::try_new("01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap(),
            away_team_id: TeamId::try_new("01ARZ3NDEKTSV4RRFFQ69G5FAW").unwrap(),
        };
        let match_day_repo = FakeMatchDayRepo::new(match_day_with_pairings(vec![existing]));
        let group_repo = FakeGroupRepo;
        let team_port = FakeTeamInfoPort;
        let event_bus = crate::common::services::event_bus::event_bus::new_bus();

        let result = execute(
            "d1",
            "s1",
            "c1",
            "sp1",
            &match_day_repo,
            &group_repo,
            &team_port,
            &event_bus,
        )
        .await;

        assert!(matches!(result, Err(GenerateError::PairingsAlreadyExist)));
    }

    #[tokio::test]
    async fn proceeds_when_match_day_has_no_pairings() {
        let match_day_repo = FakeMatchDayRepo::new(match_day_with_pairings(vec![]));
        let group_repo = FakeGroupRepo;
        let team_port = FakeTeamInfoPort;
        let event_bus = crate::common::services::event_bus::event_bus::new_bus();

        let result = execute(
            "d1",
            "s1",
            "c1",
            "sp1",
            &match_day_repo,
            &group_repo,
            &team_port,
            &event_bus,
        )
        .await;

        // pas de groupes ni d'équipes enrôlées -> NoGroups, mais surtout PAS PairingsAlreadyExist
        assert!(matches!(result, Err(GenerateError::NoGroups)));
    }

    #[tokio::test]
    async fn reports_group_with_fewer_than_two_teams_as_skipped_instead_of_silent_success() {
        let match_day_repo = FakeMatchDayRepo::new(match_day_with_pairings(vec![]));
        let group_repo = FakeGroupRepoWithEmptyGroup("Poule 1");
        let team_port = FakeTeamInfoPortWithEnrolled(vec![TeamInfoDto {
            team_id: "t1".into(),
            team_name: "Team 1".into(),
            coach_id: String::new(),
            coach_name: String::new(),
            roster_name: String::new(),
            logo_url: None,
        }]);
        let event_bus = crate::common::services::event_bus::event_bus::new_bus();

        let outcome = execute(
            "d1",
            "s1",
            "c1",
            "sp1",
            &match_day_repo,
            &group_repo,
            &team_port,
            &event_bus,
        )
        .await
        .expect("ne doit pas échouer, juste signaler la poule ignorée");

        assert_eq!(outcome.skipped_group_names, vec!["Poule 1".to_string()]);
    }

    /// Non-régression de la carte 507 : « Générer les rencontres » deux fois de
    /// suite doit donner deux appariements différents.
    ///
    /// Le test vit **ici** et pas seulement dans le domaine : celui du domaine
    /// prouve que `tirer` sait varier à graines différentes ; celui-ci prouve
    /// que le use case lui donne bien un générateur neuf. Une graine fixe
    /// oubliée dans `execute` passerait le premier et échouerait ici.
    #[tokio::test]
    async fn regenerer_une_journee_ne_redonne_pas_le_meme_appariement() {
        let equipes: Vec<TeamInfoDto> = (0..6)
            .map(|i| TeamInfoDto {
                team_id: TeamId::new().to_string(),
                team_name: format!("Équipe {i}"),
                coach_id: TeamId::new().to_string(),
                coach_name: format!("Coach {i}"),
                roster_name: "Humains".to_string(),
                logo_url: None,
            })
            .collect();

        let mut vus: HashSet<Vec<(String, String)>> = HashSet::new();
        for _ in 0..10 {
            let match_day_repo = FakeMatchDayRepo::new(match_day_with_pairings(vec![]));
            let outcome = execute(
                "d1",
                "s1",
                "c1",
                "sp1",
                &match_day_repo,
                &FakeGroupRepo,
                &FakeTeamInfoPortWithEnrolled(equipes.clone()),
                &crate::common::services::event_bus::event_bus::new_bus(),
            )
            .await
            .expect("génération");
            assert!(outcome.unproven_group_names.is_empty());
            assert_eq!(
                match_day_repo.ecrits().len(),
                3,
                "six équipes, trois matchs"
            );
            vus.insert(match_day_repo.ecrits());
        }

        assert!(
            vus.len() > 1,
            "dix générations ont donné exactement le même appariement"
        );
    }

    /// Un dépôt en échec ne laisse **rien** passer : ni appariement, ni
    /// événement.
    ///
    /// C'est tout l'objet de la carte 509. Avant, les appariements partaient un
    /// par un et leurs événements avec : une panne au troisième laissait une
    /// journée à moitié appariée, et des listeners avaient déjà réagi à des
    /// rencontres qui n'existeraient jamais.
    #[tokio::test]
    async fn un_depot_en_echec_n_ecrit_ni_n_emet_rien() {
        let equipes: Vec<TeamInfoDto> = (0..4)
            .map(|i| TeamInfoDto {
                team_id: TeamId::new().to_string(),
                team_name: format!("Équipe {i}"),
                coach_id: TeamId::new().to_string(),
                coach_name: format!("Coach {i}"),
                roster_name: "Humains".to_string(),
                logo_url: None,
            })
            .collect();

        let depot = FakeMatchDayRepo::en_panne(match_day_with_pairings(vec![]));
        let bus = crate::common::services::event_bus::event_bus::new_bus();
        let mut abonne = bus.subscribe();

        let resultat = execute(
            "d1",
            "s1",
            "c1",
            "sp1",
            &depot,
            &FakeGroupRepo,
            &FakeTeamInfoPortWithEnrolled(equipes),
            &bus,
        )
        .await;

        assert!(
            matches!(resultat, Err(GenerateError::Repository(_))),
            "l'échec d'écriture doit remonter"
        );
        assert!(depot.ecrits().is_empty(), "rien ne doit être écrit");
        assert!(
            abonne.try_recv().is_err(),
            "aucun PairingCreated ne doit être émis quand l'écriture échoue"
        );
    }
}
