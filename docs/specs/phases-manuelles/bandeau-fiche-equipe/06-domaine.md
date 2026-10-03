# Bandeau de la fiche équipe · Phase 6 : domaine

Tout se passe dans l'agrégat `Team` (`teams/domain/team.rs`) et ses value
objects. Les identifiants sont en anglais (règle 18).

## 1. Les types neufs

- **`PhaseEntry { PostMatch, Override }`**, `PostMatch` par défaut — champ
  `phase_entry` de l'agrégat, **dérivé** : reconstruit au rejeu, jamais stocké.
  Aucune migration.
- **`OverridablePhase { PlayerImprovement, Recruitment, Dismissals }`** :
  - `ALL`, d'où viennent les cartes du panneau ;
  - `parse(&str)` et `as_str()` — `player_improvement`, `recruitment`,
    `dismissals` ;
  - `game_phase()`, la `GamePhase` correspondante.
- **`OverrideReason`** (`value_objects.rs`) : `trim`, 200 caractères au plus,
  charset `TEXTE_SAISI`.

## 2. Les événements

```rust
ManualImprovementPhaseOpened { admin_id: CoachId, admin_name: CoachName, reason: Option<OverrideReason> },
ManualRecruitmentPhaseOpened { admin_id: CoachId, admin_name: CoachName, reason: Option<OverrideReason> },
ManualDismissalsPhaseOpened  { admin_id: CoachId, admin_name: CoachName, reason: Option<OverrideReason> },
ManualPhaseClosed { phase: GamePhase },
```

Nommés en termes de domaine, ce qui s'est passé — pas d'où vient le
déclencheur.

## 3. La commande d'ouverture

```rust
pub fn open_phase_override(&self, phase: OverridablePhase, admin_id: CoachId,
    admin_name: CoachName, reason: Option<OverrideReason>) -> Result<TeamDomainEvent, DomainError>
```

`expect_phase(ReadyToPlay)` d'abord — `WrongGamePhase`, l'erreur existante,
depuis toute autre phase. Puis l'événement d'ouverture de la phase choisie.

**Une équipe dont le match vient d'être publié est dans son vrai après-match**,
pas prête à jouer : l'ouverture lui est refusée par cette même règle. C'est
voulu (2026-10-03).

## 4. `apply()`

| Événement | `game_phase` | `phase_entry` |
|---|---|---|
| les trois ouvertures | la phase ouverte | `Override` |
| `ManualPhaseClosed` | `ReadyToPlay` | `PostMatch` |
| `PostMatchSequenceStarted` (existant) | `PlayerImprovement` (inchangé) | **remise à `PostMatch`** |

Sans la remise à zéro sur `PostMatchSequenceStarted`, l'après-match du vrai
match suivant serait pris pour une phase manuelle.

## 5. Les trois sorties

Elles gardent leur `expect_phase` et consultent l'entrée de phase :

- `validate_improvement_phase` : `Override` → `ManualPhaseClosed { phase:
  PlayerImprovement }` ; sinon `PlayerImprovementPhaseValidated`, comme
  aujourd'hui ;
- `validate_recruitment_phase` : `Override` → `ManualPhaseClosed { phase:
  Recruitment }` ;
- `validate_dismissals_phase` : `Override` → `ManualPhaseClosed { phase:
  Dismissals }`, **quelle que soit la trésorerie** — pas d'erreurs coûteuses ;
  sinon le comportement actuel, seuil compris.

## 6. `returns_to_ready_to_play()`

Méthode de `TeamDomainEvent`, `match` **exhaustif, sans joker** :

- **vrai** : `TeamEnrolled`, `DismissalsPhaseValidated`,
  `MatchReportingCancelled`, `CostlyMistakesApplied`, `ManualPhaseClosed` ;
- **faux** : tous les autres, chacun nommé.

`GamePhaseOverridden` — l'ancienne carte 46, jamais émis en production — est
classé faux, commentaire à l'appui. Les deux listeners (valeur d'équipe, purge
des paniers) appellent cette méthode à la place de leurs copies.

## 7. L'annulation d'un après-match

`revert_post_match_sequence` exige aujourd'hui `PlayerImprovement` et un
`last_post_match` qui correspond au rapport. Or `last_post_match` n'est jamais
effacé à la fin d'un cycle normal : une équipe passée manuellement en dépense de
SPP après son dernier match remplissait les deux conditions, et le domaine
aurait annulé ce match par-dessus tout ce qui s'est passé depuis.

Elle refuse désormais une phase d'entrée `Override`, avec l'erreur existante
`NoPostMatchToRevert` : une phase ouverte à la main ne prolonge aucun match. Le
filtre de l'adapter de correction (phase 7) est un confort ; **la garde est
ici**.

## 8. Tests

- Ouverture uniquement depuis `ReadyToPlay` : refus depuis toute autre phase,
  une ouverture réussie pour chacune des trois.
- L'ouverture pose la phase et `Override`.
- Chaque sortie d'une phase manuelle rend `ManualPhaseClosed`, ramène à
  `ReadyToPlay` et remet `PostMatch`.
- Une sortie de renvois manuelle au-dessus du seuil ne déclenche pas d'erreurs
  coûteuses.
- Un vrai après-match après une phase manuelle se comporte normalement.
- L'annulation d'un après-match est refusée pendant une phase manuelle.
- Le rejeu reconstruit `phase_entry`.
- `returns_to_ready_to_play()` est vrai pour exactement les cinq événements du
  point 6.

## Règles métier

Question posée le 2026-10-03 — confirmée à cette étape :

10. **Une équipe dans son vrai après-match ne peut pas recevoir de phase
    manuelle** : elle n'est pas prête à jouer.
