# Calendrier · appariement des présents — use cases

## Le chargement commun — `use_cases/admin/draw/draw_input.rs`

Les quatre traitements lisent la même chose ; ils la chargent par un seul
chemin.

```rust
pub struct DrawContext {
    pub match_day: MatchDay,
    pub teams: HashMap<String, TeamInfoDto>, // inscrites, via ITeamInfoPort
    pub groups: Vec<GroupWithTeams>,         // ou une poule « Toutes les équipes » sans poule
    pub season_days: Vec<MatchDay>,          // historique et matchs joués
}

pub async fn load_draw_context(
    round_id: &MatchId,
    season_id: &SeasonId,
    deps: &DrawDeps<'_>,
) -> Result<DrawContext, DrawLoadError>; // RoundNotFound | Repository
```

Le fichier porte aussi les trois fonctions copiées de `entree_du_tirage.rs` —
`build_history`, `build_forbidden_pairs` (même coach), `build_matches_played` —
et la construction du `DrawInput` d'une poule à partir d'une liste d'équipes.

## 1. `draw_candidates_service::load` — lecture

```rust
// arch:no-instrument — service de lecture : assemble l'étape « Présents », sans intention métier
pub async fn load(
    round_id: &MatchId,
    season_id: &SeasonId,
    deps: &DrawDeps<'_>,
) -> Result<DrawCandidates, DrawCandidatesError>;
```

1. Charger le contexte.
2. `match_day.can_receive_draw()` — refuse une journée de repos ou déjà pourvue.
3. Par poule, les équipes inscrites avec leur nombre de matchs joués.

## 2. `preview_draw_use_case::execute` — aperçu, sans écriture

```rust
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: PreviewDrawCommand,
    deps: DrawDeps<'_>,
    rng: &mut impl Rng,
) -> Result<DrawPreview, PreviewDrawError>;
// RoundNotFound | IsRestDay | PairingsAlreadyExist | NotEnoughPresent | Repository
```

1. Charger le contexte, `can_receive_draw()`.
2. Par poule : les présents inscrits → `DrawInput` → `draw::draw(&input, rng)`.
3. Aucune poule à deux présents ou plus → `NotEnoughPresent` (R10, règle du
   domaine sur l'ensemble des poules).
4. Assembler `DrawPreview` — noms, historique.

Le générateur est **reçu en paramètre** : graine fixe en test,
`StdRng::from_os_rng()` côté handler, comme le tirage des poules.

## 3. `confirm_draw_use_case::execute` — écriture

```rust
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ConfirmDrawCommand,
    deps: DrawDeps<'_>,
    event_bus: &EventBus,
) -> Result<ConfirmOutcome, ConfirmDrawError>;
```

1. Charger le contexte.
2. `match_day.accept_draw(&cmd.pairings, cmd.exempted, &rules)` — `rules` porte
   les inscrites, les paires interdites et la poule de chaque équipe. C'est
   l'agrégat qui refuse (R4, R11) ; le use case relaie `Refused(DomainError)`.
3. Construire un `Pairing` et sa projection par rencontre
   (`build_new_pairing_projection`, existant).
4. `save_pairings(round_id, &all)` — **une transaction**, sous verrou ; journée
   remplie entre-temps → `PairingsAlreadyExist`.
5. Émettre `PairingCreated` par rencontre **après le commit**
   (`pairing_created.rs`), jamais avant.
6. `ConfirmOutcome` — les absentes sont les inscrites ni appariées ni exemptées.

## 4. `generate_pairings::execute` — modifié (R7, R12)

Signature et `GenerateOutcome` inchangées.

1. Charger le contexte, `can_receive_draw()`.
2. Par poule : tirer parmi **toutes** les inscrites ; équipes et poules
   ignorées toujours signalées.
3. Le même `match_day.accept_draw`.
4. **Un seul `save_pairings` pour toute la journée** — elle s'écrivait
   rencontre par rencontre.
5. Émettre après le commit.

`generate_all_pairings` est inchangé : il appelle ce use case journée par
journée.

## Ce qui est retiré — consommateurs vérifiés (règle 4)

| Élément | Consommateurs | Remplacé par |
|---|---|---|
| `generate_round_pairings` et ses tests (`domain/match_day.rs`) | `generate_pairings.rs` seul | `draw::draw` et ses tests copiés |
| `build_played_set`, `generate_and_save_group_pairings` (`generate_pairings.rs`) | locaux | `draw_input.rs`, `save_pairings` |
| `emit_pairing_created` (`generate_pairings.rs`) | local | `pairing_created.rs` |

Le commentaire de `tests/e2e/test_invariant_journee.py` qui cite
`generate_round_pairings` est mis à jour.

## Règles métier ajoutées à cette étape

| # | Règle |
|---|---|
| R14 | « Générer une journée complète » est tout ou rien : un échec d'écriture n'écrit aucune rencontre de la journée, pas même celles des poules déjà tirées. |
