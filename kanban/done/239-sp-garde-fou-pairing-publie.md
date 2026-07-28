# `competitions` — Un rapport publié interdit la suppression de son pairing

**Priorité : haute**
**Dépend de :** `238-sp-annulation-rapport-engage.md`
**Fichiers :** `src/app/competitions/ports.rs`, `src/infrastructure/competitions/match_report_status_adapter.rs` (nouveau), `src/app/match_report/domain/match_report_repository_port.rs`, `src/app/match_report/io/repository/match_report_repository.rs`, `src/app/competitions/use_cases/admin/delete_pairing_use_case.rs` (nouveau), `src/app/competitions/io/web/admin/schedule_actions.rs`, `src/app/competitions/io/web/admin/schedule_widgets.rs`, `src/app/competitions/io/web/templates/admin/widgets/schedule-round-detail.html`, `src/app/competitions/context.rs`, `src/main.rs`

## Objectif

La suppression d'un pairing est aujourd'hui inconditionnelle. Supprimer celui
d'un match **publié** retire la ligne du calendrier et des résultats, mais le
rapport et le classement conservent le match : incohérence silencieuse, et
irrattrapable côté `ranking`.

Règle retenue : **seul un rapport publié bloque**. `Draft`, `PreMatch` et
`ReadyToPublish` restent supprimables — la carte 238 les annule proprement.

Un admin qui veut vraiment supprimer un match publié dépublie d'abord le rapport
(fonctionnalité de correction déjà livrée), ce qui le ramène en
`ReadyToPublish`, donc supprimable. Le message de refus le dit explicitement.

## Conception

### 1. Consultation, pas propagation → port + adapter

« Ce rapport est-il publié **maintenant** ? » est un garde-fou métier bloquant :
critère « Consultation vs propagation » du CLAUDE.md → port synchrone, pas
d'event, pas de projection locale.

```rust
// src/app/competitions/ports.rs — DTO de lecture, primitives assumées
pub struct PublishedReportDto {
    pub pairing_id:     String,
    pub home_team_name: String,
    pub away_team_name: String,
}

#[async_trait]
pub trait IMatchReportStatusPort: Send + Sync {
    /// Parmi ces pairings, ceux dont le rapport est publié.
    async fn find_published_pairings(&self, pairing_ids: &[String])
        -> Result<Vec<PublishedReportDto>, String>;
}
```

Adapter dans `src/infrastructure/competitions/` — seul endroit autorisé à
importer `match_report`. Il s'appuie sur une lecture **batch** à ajouter au port
du repo `match_report` (`find_phases_by_pairings`, requête unique sur
`match_report_proj`), et non sur N `find_id_by_pairing` + `find_by_id` : la
carte 240 l'appellera avec toute une saison de pairings.

Instanciation dans `main.rs`, injection dans `CompetitionsContext` sous forme
d'`Arc<dyn IMatchReportStatusPort>`.

### 2. Use case

`use_cases/admin/delete_pairing_use_case.rs` :

1. consulte le port
2. refuse si le pairing a un rapport publié → `DeletePairingError::ReportPublished(Vec<PublishedReportDto>)`
3. sinon `delete_pairing`, puis **émet le domain event `PairingDeleted` sur le
   bus interne**

Le point 3 est un déplacement : c'est le handler qui émet aujourd'hui
(`emit_pairing_deleted_events`), ce que la règle « App events vs Domain events »
n'autorise pas. Le use case prend `&EventBus` (bus interne du BC), comme
`add_match_use_case`.

### 3. Handler

`delete_match` appelle le use case et traduit le refus en `422` +
`ErrorResult`, exactement comme `add_match_refused` :

> Match non supprimé : le rapport de *Les A – Les B* est publié. Dépubliez-le
> depuis son récapitulatif avant de supprimer la rencontre.

### 4. Front — bouton masqué en amont

Le widget `schedule-round-detail` reçoit, via le même port, l'ensemble des
pairings à rapport publié de la journée affichée, et n'émet pas le bouton
`.match-row-delete` pour ceux-là. Le refus `422` reste la ceinture : la page
peut être obsolète au moment du clic.

Le bouton restant ignore aujourd'hui le statut de la réponse (il déclenche
`scheduleChanged` quoi qu'il arrive) — il passe par
`window.handleScheduleActionResponse(res)` comme toutes les autres actions du
calendrier admin.

## Checklist

- [ ] `find_phases_by_pairings` sur le port repo de `match_report` (requête batch)
- [ ] Port `IMatchReportStatusPort` + DTO dans `competitions/ports.rs`
- [ ] Adapter dans `src/infrastructure/competitions/`, câblé dans `main.rs` et `context.rs`
- [ ] `delete_pairing_use_case` : refus, suppression, émission du domain event
- [ ] `emit_pairing_deleted_events` retiré du handler `delete_match`
- [ ] Handler : `422` + message orientant vers la dépublication
- [ ] Widget : bouton masqué pour les pairings à rapport publié
- [ ] Bouton câblé sur `handleScheduleActionResponse`
- [ ] Test : refus si le rapport est publié, aucun événement émis, pairing toujours là
- [ ] Test : suppression acceptée pour `Draft`, `PreMatch`, `ReadyToPublish`, `Cancelled`, et pour un pairing sans rapport
- [ ] Test : le domain event `PairingDeleted` est bien émis par le use case
- [ ] Test E2E : rapport publié → bouton absent, et l'appel direct de la route répond 422
- [ ] `make test` passe
- [ ] `make check-arch` passe
