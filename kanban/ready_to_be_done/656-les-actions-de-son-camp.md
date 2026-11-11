# Les actions de son camp

**Épic :** E22 — Le mode « Chacun son équipe » · **Ordre :** 5 · **Dépend de :** 652, 655
**Conception :** `docs/specs/saisie-mobile/E22-cadrage.md` (R6, § 4, § 5, § 12.3, § 12.6)
**Maquettes :** `app-mobile-duo-live.html` (B2), `app-mobile-real-3b-actions.html` (B2c)

## Le constat

- Les actions sont **déjà stockées par camp** (`ActionRecorded.team_side`,
  `src/app/match_report/domain/events.rs:143-153`) ; un blessé s'inscrit au camp
  du joueur blessé (`domain/value_objects.rs:390-398`) : chaque coach saisit les
  faits de ses joueurs.
- Le ✕ est **le même pour toutes les lignes** : `can_delete_actions` n'est que
  `can_write_report` (`use_cases/match_report_access_service.rs:63-71`,
  `io/web/widgets/action_log_widget.rs:88-89`). `DELETE /actions/{action_id}`
  ne porte pas le camp ; `delete_action` le retrouve dans l'agrégat
  (`domain/match_report_pre_match.rs:598-628`).
- `/live` propose un choix d'équipe (`templates/live-match.html:85-108`), et ne
  montre la présence et le bandeau « X saisit tout le match » qu'à qui ne peut
  pas écrire (`:50-67`).
- Le lien des toasts mène à `/live` (`src/web/realtime/toast.rs:104-108`) :
  le second coach peut y arriver avant d'avoir fait son avant-match.

## Ce qui change

- `step3*` (domicile) et `step4*` (visiteur) passent dans `routes_de_camp()`.
- **Le ✕ par ligne** : `can_delete_action(…, side)` sur `can_write_side` ;
  le contexte du journal porte les camps où l'utilisateur écrit.
- **La suppression** : `delete_action` reçoit les camps permis ; une action de
  l'autre camp est refusée par le domaine (`DomainError::ActionOutsideSide`,
  nouvelle) — 403.
- **`/live` en mode B** : « ＋ Action des [mon équipe] » mène droit à sa page de
  saisie, sans choix d'équipe ; la présence et l'état de l'adversaire sont
  affichés aux deux coachs ; « Fin du match → MVP » mène à `mvp/{mon camp}`.
  L'admin garde le choix d'équipe (§ 12.6).
- **Le coup d'envoi** (§ 12.3) : en mode B, `/live`, `step3*` et `step4*`
  renvoient le coach à son avant-match ou à ses coups de pouce tant que les deux
  fans factors et les deux sélections ne sont pas enregistrés ; le domaine
  refuse une action avant (`DomainError::MatchNotStarted`, nouvelle).

## Fichiers touchés

`router.rs`, `use_cases/match_report_access_service.rs`,
`io/web/widgets/action_log_widget.rs`, `templates/action-log-widget.html`,
`io/web/live_match_controller.rs`, `templates/live-match.html`,
`io/web/actions_step_controller.rs`, `use_cases/delete_action_use_case.rs`,
`use_cases/record_action_use_case.rs`, `domain/match_report_pre_match.rs`,
`domain/error.rs` ; `tests/e2e/visual/urls.py`.

## Règles et invariants

- Une action ne s'écrit et ne s'efface que dans son camp (R6).
- En mode B, le match commence quand les deux avant-matchs et les deux
  sélections de coups de pouce sont enregistrés (§ 12.3) ; la règle vaut à la
  saisie, jamais au rejeu.
- Un MVP tiré reste définitif (E21).

## Tests

- **Unitaires** : ✕ présent sur ses lignes seules, sur toutes pour l'admin ;
  suppression d'une action adverse refusée ; action refusée avant le coup
  d'envoi en mode B, acceptée en mode A.
- **E2E** : deux contextes sur `/live` ; chacun saisit une action, la voit chez
  l'autre sans rechargement, avec « saisi par » ; le ✕ n'apparaît que sur ses
  lignes ; un POST sur `step4/actions` par le domicile et un `DELETE` d'une
  action adverse rendent 403 ; `/live` avant le coup d'envoi renvoie à
  l'avant-match.

## Conséquences sur les données de production

Aucune. Les actions sont déjà stockées par camp avec leur auteur ; la règle du
coup d'envoi ne vaut qu'en mode B et qu'à la saisie.

## Terminé quand

En mode B, chaque coach saisit et efface les actions de son équipe seule, voit
celles de l'autre en direct, et une tentative directe dans le camp adverse est
refusée.
