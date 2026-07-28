# `match_report` / `teams` — Annulation d'un rapport engagé

**Priorité : haute**
**Dépend de :** rien
**Fichiers :** `src/app/match_report/domain/{events,match_report_pre_match,match_report_ready_to_publish,match_report_state}.rs`, `src/app/match_report/io/app_events/{app_event_publisher,pairing_deleted_listener}.rs`, `src/app/match_report/io/repository/match_report_repository.rs`, `src/app/shared_kernel/app_events/match_report_app_events.rs`, `src/app/teams/domain/team.rs`, `src/app/teams/io/repository/team_repository.rs`, `src/app/teams/io/app_events/match_report_cancelled_listener.rs` (nouveau), `src/app/teams/context.rs`

## Objectif

Supprimer un pairing dont le rapport est **engagé mais non publié** (`PreMatch`,
`ReadyToPublish`) doit annuler ce rapport **et libérer les deux équipes**.

Aujourd'hui `pairing_deleted_listener` n'annule que les rapports en `Draft` et
sort silencieusement (`_ => continue`) sur tous les autres états. Le pairing est
pourtant déjà supprimé : le rapport survit, orphelin, et — pire — les équipes
restent verrouillées.

## Le bug de fond : le verrou d'équipe

Dès la confirmation de la sélection (`Draft → PreMatch`), `teams` réagit à
`MatchReportConfirmed` et appelle `Team::start_match_reporting()` :

```rust
self.game_phase              = Some(GamePhase::MatchReporting);
self.current_match_report_id = Some(match_report_id);
```

La **seule** sortie de `MatchReporting` est aujourd'hui
`start_post_match_sequence()`, c'est-à-dire la publication. Annuler un rapport
confirmé sans compensation laisse donc les deux équipes verrouillées
**définitivement** : `start_match_reporting()` exige `ReadyToPlay`, elles ne
peuvent plus jouer aucun autre match.

Rien d'autre n'est à compenser : seuls `competitions` et `teams` écoutent
`MatchReportConfirmed`, et côté `competitions` la ligne de
`competition_match_display_proj` est déjà supprimée dans la transaction du
`delete_pairing`. `players` et `ranking` ne réagissent qu'à la publication.

## Conception

### 1. Domaine `match_report` — annuler depuis trois états

`cancel()` n'existe que sur `MatchReportDraft`. L'ajouter à
`MatchReportPreMatch` et `MatchReportReadyToPublish`, à l'identique :

```rust
pub fn cancel(self, reason: String) -> MatchReportDomainEvent
```

`rehydrate` sait passer de `Draft` et de `PreMatch` à `Cancelled`, mais **pas de
`ReadyToPublish`** — transition à ajouter.

### 2. Le domain event doit porter les équipes

`MatchReportCancelled { reason }` ne suffit plus. Le publisher reconstruit ses
payloads en **relisant l'agrégat** après l'append, or l'état `Cancelled` ne
retient que `id` et `reason` : les ids d'équipes seraient perdus, et le listener
`teams` n'aurait personne à libérer.

```rust
MatchReportCancelled {
    reason:       String,   // arch:ok texte libre
    home_team_id: TeamId,
    away_team_id: TeamId,
},
```

Value objects, pas de primitives (règle CQRS du CLAUDE.md). Les trois `cancel()`
lisent ces ids sur l'état courant.

**Migration d'événements** : les `MatchReportCancelled` déjà persistés n'ont pas
ces champs. Ils sont tous issus d'annulations de `Draft`, sans effet à
compenser — désérialiser en tolérant l'absence (`#[serde(default)]` sur un
`Option`, ou valeur de repli) plutôt que casser la rehydratation de l'existant.

### 3. App event, émis par le publisher

Nouveau `MatchReportAppEvent::MatchReportCancelled { event_id, match_report_id,
home_team_id, away_team_id }`, produit dans `app_event_publisher.rs`
(`handle_envelope`), **jamais depuis un use case ni un listener** — règle
« App events vs Domain events » du CLAUDE.md.

À noter : `MatchReportConfirmed` est aujourd'hui émis directement depuis les use
cases. Écart préexistant, non reproduit ici et non corrigé par cette carte.

### 4. `pairing_deleted_listener` — annuler quel que soit l'état non publié

```rust
let (version, cancel_event) = match state {
    MatchReportState::Draft(d)          => (d.version, d.cancel(reason)),
    MatchReportState::PreMatch(pm)      => (pm.version, pm.cancel(reason)),
    MatchReportState::ReadyToPublish(r) => (r.version, r.cancel(reason)),
    MatchReportState::Cancelled(_)      => continue,          // idempotent
    MatchReportState::Published(_)      => { tracing::error!(…); continue }
};
```

La branche `Published` devient un cas **anormal** : la carte 239 pose le
garde-fou qui la rend inatteignable, donc elle se journalise en `error` et ne se
tait plus.

### 5. `teams` — libérer l'équipe

```rust
pub fn cancel_match_reporting(&self, match_report_id: MatchReportId)
    -> Result<TeamDomainEvent, DomainError>
```

Gardes, sur le modèle de `revert_post_match_sequence` :
1. `expect_phase(GamePhase::MatchReporting)`
2. `current_match_report_id` renseigné **et** égal à ce `match_report_id` — sans
   quoi on libérerait une équipe déjà repartie sur un autre match

Événement `MatchReportingCancelled { match_report_id }`, et `apply` :

```rust
self.game_phase              = Some(GamePhase::ReadyToPlay);
self.current_match_report_id = None;
```

Le retour est `ReadyToPlay` et non `Enrolled` : c'est la phase d'où
`start_match_reporting` est parti, l'équipe redevient exactement disponible.

La projection `team_projection` porte `game_phase` — mise à jour **dans la
transaction de l'append** (règle ES du CLAUDE.md). Piège connu : un événement
non traité par la fonction de projection compile sans broncher et laisse
l'affichage figé sur « match en cours ».

### 6. Listener

`teams/io/app_events/match_report_cancelled_listener.rs`, filtre
`MatchReportCancelled`, traite les deux équipes sans jamais croiser home et away
(un échec sur l'une ne prive pas l'autre de sa libération), câblé dans
`teams::context`. Souscription à `app_event_bus` → listener cross-BC, hors du
périmètre transaction unique (exception documentée du CLAUDE.md).

## Checklist

- [ ] `cancel()` sur `MatchReportPreMatch` et `MatchReportReadyToPublish`
- [ ] Transition `ReadyToPublish → Cancelled` dans `rehydrate`
- [ ] `MatchReportCancelled` porte les deux `TeamId`, tolérant aux événements déjà persistés
- [ ] App event `MatchReportCancelled` émis par le publisher
- [ ] `pairing_deleted_listener` annule les trois états non publiés, journalise `Published`
- [ ] `Team::cancel_match_reporting()` avec ses 2 gardes
- [ ] Événement `MatchReportingCancelled` + `type_name()` + `apply`
- [ ] Projection `team_projection` mise à jour dans la transaction de l'append
- [ ] Listener `teams` créé et câblé
- [ ] Test : annulation depuis `Draft`, `PreMatch`, `ReadyToPublish`
- [ ] Test : rehydratation d'un flux se terminant par `MatchReportCancelled` depuis chacun des trois états
- [ ] Test : un `MatchReportCancelled` sans ids d'équipes (ancien format) se rehydrate encore
- [ ] Test : `cancel_match_reporting` nominal → `ReadyToPlay`, id remis à `None`
- [ ] Test : refus si la phase n'est pas `MatchReporting`
- [ ] Test : refus si le `match_report_id` ne correspond pas à celui en cours
- [ ] Test : une seconde annulation ne produit rien
- [ ] Test d'intégration : la projection reflète la phase libérée
- [ ] Test E2E : supprimer un pairing dont le rapport est confirmé → les deux équipes peuvent démarrer un nouveau rapport
- [ ] `make test` passe
- [ ] `make check-arch` passe
