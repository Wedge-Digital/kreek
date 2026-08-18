use crate::app::players::domain::error::DomainError;
use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::player::{Player, PlayerId, RosterMembership, TeamId};
use crate::app::players::ports::{IPlayerRepository, RepositoryError};
use crate::app::players::use_cases::commands::{RosterRowCommand, UpdateRosterCommand};
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug)]
pub enum UpdateRosterError {
    UnknownOrInactivePlayer,
    DuplicateJersey,
    DuplicateDisplayOrder,
    Domain(DomainError),
    Repository(RepositoryError),
}

type Entry = (PlayerId, TeamId, PlayerDomainEvent, i32);

/// Met à jour l'effectif en un lot : rien n'est persisté si une seule ligne est
/// refusée. Retourne l'effectif actif à jour.
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: UpdateRosterCommand,
    player_repo: &dyn IPlayerRepository,
    event_bus: &EventBus,
) -> Result<Vec<Player>, UpdateRosterError> {
    let roster = load_active_roster(player_repo, &cmd.team_id).await?;
    ensure_all_active(&roster, &cmd.rows)?;
    ensure_no_duplicates(&roster, &cmd.rows)?;

    let entries = build_entries(&roster, &cmd.rows)?;
    let to_publish: Vec<(String, PlayerDomainEvent)> = entries
        .iter()
        .map(|(player_id, _, event, _)| (player_id.0.clone(), event.clone()))
        .collect();

    player_repo
        .append_batch(entries)
        .await
        .map_err(UpdateRosterError::Repository)?;

    for (player_id, event) in to_publish {
        emettre(event_bus, event.to_enveloppe(&player_id));
    }

    load_active_roster(player_repo, &cmd.team_id).await
}

/// `find_by_team_id` du repository rejoue **tous** les joueurs de l'équipe,
/// renvoyés compris — contrairement à son homonyme de la projection, qui filtre
/// en SQL. Le filtre est donc à faire ici.
async fn load_active_roster(
    player_repo: &dyn IPlayerRepository,
    team_id: &TeamId,
) -> Result<Vec<Player>, UpdateRosterError> {
    let players = player_repo
        .find_by_team_id(team_id)
        .await
        .map_err(UpdateRosterError::Repository)?;
    Ok(players
        .into_iter()
        .filter(|p| p.membership == RosterMembership::Active)
        .collect())
}

/// Une ligne visant un joueur inconnu ou renvoyé condamne tout le lot : mieux
/// vaut refuser en bloc qu'appliquer une moitié d'édition.
fn ensure_all_active(
    roster: &[Player],
    rows: &[RosterRowCommand],
) -> Result<(), UpdateRosterError> {
    let connus: BTreeSet<&str> = roster.iter().map(|p| p.id.0.as_str()).collect();
    match rows.iter().all(|r| connus.contains(r.player_id.0.as_str())) {
        true => Ok(()),
        false => Err(UpdateRosterError::UnknownOrInactivePlayer),
    }
}

/// L'unicité se juge sur l'**état résultant de l'effectif actif entier**, pas
/// sur le seul lot : donner à un joueur le numéro d'un coéquipier qu'on ne
/// touche pas est un conflit, alors que reprendre celui d'un renvoyé n'en est
/// pas un — ce dernier a quitté l'effectif et ne figure pas dans `roster`.
fn ensure_no_duplicates(
    roster: &[Player],
    rows: &[RosterRowCommand],
) -> Result<(), UpdateRosterError> {
    let soumis: HashMap<&str, &RosterRowCommand> =
        rows.iter().map(|r| (r.player_id.0.as_str(), r)).collect();

    let mut jerseys = BTreeSet::new();
    let mut ordres = BTreeSet::new();
    for player in roster {
        let ligne = soumis.get(player.id.0.as_str());
        let jersey = ligne.map_or(player.jersey, |r| r.jersey);
        let ordre = ligne.map_or(player.display_order, |r| Some(r.display_order));

        // Un joueur sans numéro n'entre en conflit avec personne : l'absence
        // n'est pas une valeur qu'on se dispute.
        if jersey.is_some_and(|j| !jerseys.insert(j)) {
            return Err(UpdateRosterError::DuplicateJersey);
        }
        if ordre.is_some_and(|o| !ordres.insert(o)) {
            return Err(UpdateRosterError::DuplicateDisplayOrder);
        }
    }
    Ok(())
}

fn build_entries(
    roster: &[Player],
    rows: &[RosterRowCommand],
) -> Result<Vec<Entry>, UpdateRosterError> {
    let mut entries = Vec::new();
    for row in rows {
        let Some(player) = roster.iter().find(|p| p.id == row.player_id) else {
            continue; // `ensure_all_active` l'a déjà garanti.
        };
        let events = events_for(player, row).map_err(UpdateRosterError::Domain)?;
        // Un joueur dont le nom **et** le numéro changent produit deux
        // événements : les versions s'enchaînent au sein de son propre flux.
        for (rang, event) in events.into_iter().enumerate() {
            let version = player.version + 1 + rang as i32;
            entries.push((player.id.clone(), player.team_id.clone(), event, version));
        }
    }
    Ok(entries)
}

/// Diff par champ : un champ inchangé n'émet rien. Sans ça, chaque
/// enregistrement gonflerait l'event store de trois événements par joueur,
/// même quand le coach n'a touché qu'une case.
fn events_for(
    player: &Player,
    row: &RosterRowCommand,
) -> Result<Vec<PlayerDomainEvent>, DomainError> {
    let mut events = Vec::new();
    if player.personal_name != row.personal_name {
        events.push(player.rename(row.personal_name.clone())?);
    }
    if player.jersey != row.jersey {
        events.push(player.change_jersey(row.jersey)?);
    }
    if player.display_order != Some(row.display_order) {
        events.push(player.reorder(row.display_order)?);
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::player::{Spp, ValueKpo};
    use crate::app::players::domain::value_objects::{
        DisplayOrder, JerseyVo, PersonalName, PositionNameVo, RosterLineId,
    };
    use crate::app::shared_kernel::identity::ids::SpaceId;
    use std::sync::Mutex;

    const EQUIPE: &str = "t1";

    /// Stocke les événements par joueur, `find_by_team_id` rejouant chacun via
    /// `from_events()` — `Player::apply` est privée hors du module domaine.
    ///
    /// N'implémente **pas** `append_batch` : l'implémentation par défaut du
    /// trait suffit, ce qui est précisément ce que visait la carte 291.
    #[derive(Default)]
    struct FakePlayerRepo {
        flux: Mutex<Vec<(String, PlayerDomainEvent)>>,
        echoue: bool,
    }

    impl FakePlayerRepo {
        fn avec(joueurs: Vec<PlayerDomainEvent>) -> Self {
            let flux = joueurs
                .into_iter()
                .map(|e| (player_id_de(&e), e))
                .collect::<Vec<_>>();
            Self {
                flux: Mutex::new(flux),
                echoue: false,
            }
        }

        fn evenements_appendus(&self) -> Vec<PlayerDomainEvent> {
            self.flux
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, e)| !matches!(e, PlayerDomainEvent::PlayerCreated { .. }))
                .map(|(_, e)| e.clone())
                .collect()
        }
    }

    fn player_id_de(event: &PlayerDomainEvent) -> String {
        match event {
            PlayerDomainEvent::PlayerCreated { player_id, .. }
            | PlayerDomainEvent::PlayerDismissed { player_id, .. }
            | PlayerDomainEvent::PlayerRenamed { player_id, .. }
            | PlayerDomainEvent::PlayerJerseyChanged { player_id, .. }
            | PlayerDomainEvent::PlayerReordered { player_id, .. } => player_id.0.clone(),
            autre => panic!("événement non géré par la doublure : {autre:?}"),
        }
    }

    #[async_trait::async_trait]
    impl IPlayerRepository for FakePlayerRepo {
        async fn append(
            &self,
            player_id: &PlayerId,
            _: &TeamId,
            event: &PlayerDomainEvent,
            _version: i32,
        ) -> Result<(), RepositoryError> {
            if self.echoue {
                return Err(RepositoryError::ConcurrentWrite);
            }
            self.flux
                .lock()
                .unwrap()
                .push((player_id.0.clone(), event.clone()));
            Ok(())
        }
        async fn find_by_id(&self, _: &PlayerId) -> Result<Option<Player>, RepositoryError> {
            Ok(None)
        }
        async fn find_by_team_id(&self, _: &TeamId) -> Result<Vec<Player>, RepositoryError> {
            let flux = self.flux.lock().unwrap();
            let mut ids: Vec<String> = Vec::new();
            for (id, _) in flux.iter() {
                if !ids.contains(id) {
                    ids.push(id.clone());
                }
            }
            Ok(ids
                .iter()
                .filter_map(|id| {
                    let events: Vec<PlayerDomainEvent> = flux
                        .iter()
                        .filter(|(pid, _)| pid == id)
                        .map(|(_, e)| e.clone())
                        .collect();
                    Player::from_events(&events)
                })
                .collect())
        }
        async fn find_events_by_id(
            &self,
            _: &PlayerId,
        ) -> Result<Vec<PlayerDomainEvent>, RepositoryError> {
            Ok(vec![])
        }
        async fn has_spent_spp_since_match(
            &self,
            _: &TeamId,
            _: &str,
        ) -> Result<bool, RepositoryError> {
            Ok(false)
        }
    }

    fn cree(id: &str, jersey: Option<u16>) -> PlayerDomainEvent {
        PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId(id.into()),
            team_id: TeamId(EQUIPE.into()),
            space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
            jersey: jersey.map(|j| JerseyVo::try_new(j).unwrap()),
            base_skills: vec![],
            starting_spp: Spp(0),
            starting_value: ValueKpo(100),
        }
    }

    fn ligne(id: &str, nom: Option<&str>, jersey: Option<u16>, ordre: u32) -> RosterRowCommand {
        RosterRowCommand {
            player_id: PlayerId(id.into()),
            personal_name: nom.map(|n| PersonalName::try_new(n.to_string()).unwrap()),
            jersey: jersey.map(|j| JerseyVo::try_new(j).unwrap()),
            display_order: DisplayOrder::new(ordre),
        }
    }

    fn commande(rows: Vec<RosterRowCommand>) -> UpdateRosterCommand {
        UpdateRosterCommand {
            team_id: TeamId(EQUIPE.into()),
            rows,
        }
    }

    fn bus() -> EventBus {
        crate::common::services::event_bus::event_bus::new_bus()
    }

    #[tokio::test]
    async fn update_roster_rejects_unknown_or_inactive_player_and_persists_nothing() {
        let repo = FakePlayerRepo::avec(vec![cree("un", Some(1))]);

        let erreur = execute(
            commande(vec![ligne("fantome", Some("Grok"), Some(5), 0)]),
            &repo,
            &bus(),
        )
        .await
        .unwrap_err();

        assert!(matches!(erreur, UpdateRosterError::UnknownOrInactivePlayer));
        assert!(
            repo.evenements_appendus().is_empty(),
            "une ligne refusée ne doit rien laisser passer"
        );
    }

    #[tokio::test]
    async fn update_roster_rejects_duplicate_jersey_against_untouched_active_player() {
        // « deux » n'est pas dans le lot, mais porte déjà le 2 : le conflit doit
        // quand même être vu.
        let repo = FakePlayerRepo::avec(vec![cree("un", Some(1)), cree("deux", Some(2))]);

        let erreur = execute(commande(vec![ligne("un", None, Some(2), 0)]), &repo, &bus())
            .await
            .unwrap_err();

        assert!(matches!(erreur, UpdateRosterError::DuplicateJersey));
        assert!(repo.evenements_appendus().is_empty());
    }

    #[tokio::test]
    async fn update_roster_rejects_duplicate_display_order_against_untouched_active_player() {
        let repo = FakePlayerRepo::avec(vec![cree("un", Some(1)), cree("deux", Some(2))]);

        // On range d'abord « deux » au rang 3, puis on tente d'y mettre « un ».
        execute(
            commande(vec![ligne("deux", None, Some(2), 3)]),
            &repo,
            &bus(),
        )
        .await
        .unwrap();
        let avant = repo.evenements_appendus().len();

        let erreur = execute(commande(vec![ligne("un", None, Some(1), 3)]), &repo, &bus())
            .await
            .unwrap_err();

        assert!(matches!(erreur, UpdateRosterError::DuplicateDisplayOrder));
        assert_eq!(repo.evenements_appendus().len(), avant);
    }

    #[tokio::test]
    async fn update_roster_ignores_dismissed_player_when_checking_uniqueness() {
        // « parti » portait le 2 et a été renvoyé : son numéro est libre.
        let repo = FakePlayerRepo::avec(vec![
            cree("un", Some(1)),
            cree("parti", Some(2)),
            PlayerDomainEvent::PlayerDismissed {
                player_id: PlayerId("parti".into()),
                team_id: TeamId(EQUIPE.into()),
            },
        ]);

        let effectif = execute(commande(vec![ligne("un", None, Some(2), 0)]), &repo, &bus())
            .await
            .expect("reprendre le numéro d'un renvoyé doit être permis");

        assert_eq!(
            effectif.len(),
            1,
            "le renvoyé ne fait plus partie de l'effectif"
        );
        assert_eq!(effectif[0].jersey.unwrap().into_inner(), 2);
    }

    #[tokio::test]
    async fn update_roster_only_emits_events_for_changed_fields() {
        let repo = FakePlayerRepo::avec(vec![cree("un", Some(1))]);

        // Seul le nom change : le numéro est identique, l'ordre est nouveau.
        execute(
            commande(vec![ligne("un", Some("Grok"), Some(1), 0)]),
            &repo,
            &bus(),
        )
        .await
        .unwrap();

        let events = repo.evenements_appendus();
        assert_eq!(
            events.len(),
            2,
            "attendu un renommage et un rangement : {events:?}"
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, PlayerDomainEvent::PlayerRenamed { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, PlayerDomainEvent::PlayerReordered { .. })));
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, PlayerDomainEvent::PlayerJerseyChanged { .. })),
            "le numéro n'a pas changé, aucun événement ne doit le dire"
        );

        // Rejouer la même commande ne produit plus rien du tout.
        let avant = repo.evenements_appendus().len();
        execute(
            commande(vec![ligne("un", Some("Grok"), Some(1), 0)]),
            &repo,
            &bus(),
        )
        .await
        .unwrap();
        assert_eq!(repo.evenements_appendus().len(), avant);
    }

    #[tokio::test]
    async fn update_roster_leaves_players_absent_from_batch_untouched() {
        let repo = FakePlayerRepo::avec(vec![cree("un", Some(1)), cree("deux", Some(2))]);

        execute(
            commande(vec![ligne("un", Some("Grok"), Some(1), 0)]),
            &repo,
            &bus(),
        )
        .await
        .unwrap();

        assert!(
            repo.evenements_appendus()
                .iter()
                .all(|e| player_id_de(e) == "un"),
            "aucun événement ne doit viser un joueur hors du lot"
        );
    }

    #[tokio::test]
    async fn update_roster_propagates_concurrent_write_as_is() {
        let mut repo = FakePlayerRepo::avec(vec![cree("un", Some(1))]);
        repo.echoue = true;

        let erreur = execute(
            commande(vec![ligne("un", Some("Grok"), Some(1), 0)]),
            &repo,
            &bus(),
        )
        .await
        .unwrap_err();

        // Remontée telle quelle : c'est au handler d'en faire un message, le use
        // case n'a pas à traduire une collision d'écriture en règle métier.
        assert!(matches!(
            erreur,
            UpdateRosterError::Repository(RepositoryError::ConcurrentWrite)
        ));
    }
}
