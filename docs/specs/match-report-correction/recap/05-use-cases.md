# Phase 5 — Use cases · page Recap

## Inventaire

| Fichier | BC | Nature |
|---|---|---|
| `use_cases/correction_eligibility_service.rs` | `match_report` | **nouveau** — domain service |
| `use_cases/unpublish_match_report_use_case.rs` | `match_report` | **nouveau** — use case |
| `use_cases/revert_match_ranking_use_case.rs` | `ranking` | **nouveau** — use case |
| — | `teams` | pas de use case : le listener appelle la méthode domaine |
| — | `players` | pas de use case : le listener appelle la méthode domaine |
| — | `competitions` | pas de use case : SQL direct dans le listener |

Le choix « use case ou appel direct » **suit le précédent de chaque BC**, il n'est
pas rejoué ici : `ranking` passe déjà par `record_match_ranking_use_case`,
`teams` et `players` appellent déjà leurs méthodes domaine depuis leurs
listeners (décision actée dans `docs/specs/post-match-team-effects/README.md`),
`competitions` fait déjà du SQL direct sur sa projection.

---

## 1. `correction_eligibility_service` (domain service)

```rust
pub async fn evaluate(
    home_team_id:    &TeamId,
    away_team_id:    &TeamId,
    match_report_id: &MatchReportId,
    team_data:       &dyn ITeamDataPort,
    player_data:     &dyn IPlayerDataPort,
) -> CorrectionEligibility
```

Transforme les 4 réponses de port en un value object domaine. Aucun handler ni
use case ne manipule les booléens bruts (cf. CLAUDE.md, « Domain services pour
données inter-BCs »).

### Orchestration

Les **4 appels de port partent en parallèle** via `tokio::join!` —
`build_recap_template` utilise déjà ce pattern dans le même contrôleur. On paie
toujours les 4 appels, même si le premier bloque : ils sont bon marché, et
l'évaluation séquentielle doublerait la latence perçue sur une page déjà
chargée.

### Ordre d'évaluation, déterministe

```
home · SPP dépensés   → SppAlreadySpent { side: Home }
home · phase avancée  → PhaseAdvanced   { side: Home }
away · SPP dépensés   → SppAlreadySpent { side: Away }
away · phase avancée  → PhaseAdvanced   { side: Away }
sinon                 → Eligible
```

Home avant away : précision de la règle 3 actée en phase 4. SPP avant phase pour
un camp donné : c'est la cause que le coach peut relier à une action concrète —
« j'ai acheté une compétence » — là où la validation de phase est un effet de
bord qu'il a pu déclencher sans y penser.

### Découpage (règle des 20 lignes)

| Fonction | Rôle |
|---|---|
| `evaluate` | lance les 4 appels en parallèle, délègue le verdict |
| `verdict_from(home_spp, home_phase, away_spp, away_phase)` | pure, sans async — l'ordre d'évaluation ci-dessus |
| `blocker_for_side(side, spp_spent, in_improvement)` | pure — le blocker d'un camp, ou `None` |

`verdict_from` et `blocker_for_side` sont **pures et synchrones** : les tests
unitaires de l'ordre d'évaluation n'ont besoin d'aucun mock de port.

---

## 2. `unpublish_match_report_use_case`

```rust
pub async fn execute(
    cmd:         UnpublishMatchReportCommand,
    repo:        &dyn IMatchReportRepository,
    team_data:   &dyn ITeamDataPort,
    player_data: &dyn IPlayerDataPort,
    bus:         &EventBus,
) -> Result<(), UnpublishMatchReportError>
```

### Orchestration

1. Charger l'état via `repo.find_by_id()` — exiger `MatchReportState::Published`
2. Évaluer l'éligibilité via `correction_eligibility_service::evaluate()`
3. Appeler `published.unpublish(cmd.unpublished_by, eligibility)` — **le domaine décide**
4. `repo.append(&id, &event, rtp.version - 1)` — même convention de version que `publish`
5. `bus.send(event.to_enveloppe(&id))` — bus **interne** du BC

Strictement symétrique de `publish_match_report_use_case`, y compris la
convention `version - 1` sur l'append.

### Découpage (règle des 20 lignes)

| Fonction | Rôle |
|---|---|
| `execute` | enchaîne les 5 étapes |
| `load_published(repo, id)` | charge et exige l'état `Published`, sinon `NotFound` / `NotPublished` |

### Correspondance des erreurs

| État / résultat | Erreur applicative |
|---|---|
| introuvable | `NotFound` |
| `Draft`, `PreMatch`, `ReadyToPublish` | `NotPublished` |
| `Cancelled` | `NotPublished` |
| `DomainError::CorrectionNotAllowed(b)` | `NotEligible(b)` |
| échec repository | `Repository(String)` |

L'app event bus n'apparaît **pas** dans la signature : le use case émet un
domain event sur le bus interne, le publisher fait la conversion (cf. CLAUDE.md,
« Émission des app events »).

---

## 3. `revert_match_ranking_use_case` (BC `ranking`)

```rust
pub async fn execute(
    match_report_id: &MatchReportId,
    repo:            &dyn IRankingRepository,
) -> Result<(), RevertMatchRankingError>
```

Supprime les 2 lignes de classement du match. Nouvelle méthode de port :
`IRankingRepository::delete_lines_for_match(match_report_id)`.

Aucune règle de compétition à charger, aucun recalcul : les 2 lignes du match
sont les **dernières** de chaque équipe, garanti par le garde-fou « à chaud ».
C'est ce que la restriction achète.

---

## 4. Compensations sans use case

### `teams` — listener → méthode domaine

`Team::revert_post_match_sequence(match_report_id)` → `PostMatchSequenceReverted`.

Le listener charge l'équipe, appelle la méthode, append. Aucune décision dans le
listener : le domaine refuse si la phase n'est pas `PlayerImprovement` ou si le
dernier post-match ne concerne pas ce `match_report_id`.

### `players` — listener d'impact → méthode domaine

Sur `TeamMatchImpactReverted { team_id, match_report_id }`, pour chaque joueur de
l'équipe : `Player::revert_match_impact(match_report_id)`.

Traité **dans `player_match_impact_listener`**, même tâche séquentielle que les
events d'action (cf. `03-back.md` — contention sur la version optimiste).

### `competitions` — SQL direct dans le listener

`UPDATE competition_match_display_proj` : `match_status` → `in_progress`, scores
et sorties à `NULL`, `match_report_url` vers l'édition. **Aucun pairing recréé.**

---

## Idempotence des compensations (règle 11)

Chaque compensation doit pouvoir être rejouée sans effet supplémentaire — c'est
ce qui rend acceptable la posture « on accepte l'échec partiel ».

| BC | Mécanisme |
|---|---|
| `competitions` | `UPDATE` à valeurs absolues — naturellement idempotent |
| `ranking` | `DELETE` — un second passage supprime 0 ligne |
| `teams` | le domaine refuse si la phase n'est plus `PlayerImprovement`, ou si le dernier post-match ne porte pas ce `match_report_id` |
| `players` | le domaine ne produit rien si l'instantané « dernier match » du joueur ne porte pas ce `match_report_id` |

Pour `teams` et `players`, l'idempotence est donc une **règle domaine**, pas une
précaution d'infrastructure — elle est testable unitairement.

---

## Règle métier découverte en phase 5

**Que faire si un port du garde-fou échoue ?** Les deux méthodes retournent
`Result<bool, String>` : indisponibilité, erreur SQL, équipe introuvable.

Posture retenue : **échouer fermé**. Un garde-fou qui échouerait ouvert
autoriserait une correction qui aurait dû être refusée — c'est la direction
dangereuse, celle qui laisse la base incohérente. Échouer fermé empêche
temporairement une correction légitime : gênant, jamais destructeur.

### Conséquence — amendement à la phase 4

`CorrectionBlocker` a besoin d'un troisième variant, que le typage de la phase 4
n'avait pas anticipé :

```rust
pub enum CorrectionBlocker {
    SppAlreadySpent { side: TeamSide },
    PhaseAdvanced   { side: TeamSide },
    /// Un port du garde-fou n'a pas pu répondre — on échoue fermé.
    EligibilityUnknown,
}
```

Sans `side` : l'échec ne désigne aucun camp. Message associé, sans nom d'équipe :
« Impossible de vérifier si ce rapport est corrigeable pour le moment. »

Le fichier `04-dtos.md` est amendé en conséquence.
