# Le routage des notifications

**Épic :** E23 — Le temps réel : connexions, présence et notifications · **Ordre :** 2 · **Dépend de :** 642, 643
**Priorité :** normale
**Conception :** `docs/specs/saisie-mobile/E23-cadrage.md` (R2, R3, R5, R7, § 3, § 4 « Le routage », § 11.5, 11.6, 11.7, 11.13, 11.14, 11.17)

## Objectif

Un coach est prévenu, où qu'il soit dans l'application, de ce qui se passe sur
ses matchs : « Saisie du match démarrée par Morgane », « Touchdown : Griff #9
(London Rats) — par Morgane », « London Rats : Relance d'équipe ×2,
Pot-de-vin — validé par Morgane ».
Le joueur est écrit tel que `match_report` l'affiche (`player_display_name`) :
en tirer le seul numéro serait recomposer une valeur (arbitrage du
2026-11-02). Il ne reçoit que les faits de ses propres matchs, jamais
les siens, et rien n'est gardé pour plus tard.

## Ce que le code fait

- Les faits sortent par l'`app_event_bus` depuis la 643 ; le registre des
  connexions existe depuis la 642.
- `window.kreekToast(message, type)` (`src/web/templates/app-layout.html:96-115`) :
  texte par `textContent`, succès retiré après 2,5 s (`:113`). L'écouteur
  `showToast` (`:120-123`) sert les `HX-Trigger` de `team_creation`
  (`finalize_team.rs:221`, `:381`, `build_team/submit_team.rs:128`) et de
  `competitions` (`move_pairing_widget.rs:273`).
- **Aucun port ne rend le coach d'une équipe** : `TeamInfoDto` porte
  `coach_name`, pas `coach_id` (`src/app/match_report/ports.rs:121-128`) ;
  `is_coach_of_team` compare `team.coach_id`
  (`src/infrastructure/match_report/ref_team_data_adapter.rs:59-69`).
- La résolution de E02 (`competitions/use_cases/notification_recipients.rs:88`)
  parle d'invités et d'inscrits d'une saison : elle n'est pas réutilisée.
- Le bus a une capacité de 256 ; un abonné en retard reçoit `Lagged(n)`
  (`src/common/services/event_bus/event_bus.rs:4-8`).

## Ce qu'il faut faire

- **Le port** dans `src/web/ports.rs` :
  `coaches_of_match(space_id, home_team_id, away_team_id) -> Vec<String>`,
  implémenté par `src/infrastructure/web/match_audience_adapter.rs` sur le dépôt
  de `teams` : le `coach_id` de chaque équipe **dont l'espace est celui de
  l'événement** (décision 6), sans intersection avec les membres.
- **Le routeur**, `src/web/realtime/notification_router.rs`, un listener
  `spawn_listener` sur l'`app_event_bus`, `init(app_event_bus: &EventBus, …)` :
  pour un `MatchReportAppEvent` qui fait un toast (décision 7), destinataires
  = `coaches_of_match` moins l'auteur, croisés avec le registre, livrés à
  **toutes** leurs connexions — y compris celle qui regarde ce match
  (décision 13). Aucun appel de port par connexion.
- **La mise en forme**, `src/web/realtime/toast.rs` : titre, sous-titre, lien
  composés depuis les faits de l'app event ; le lien vers `/live` par
  `AppRoutes::default().match_report.live(…)` (`match_report/routes.rs:123`).
  Les phrases vouvoient quand elles s'adressent au coach (règle 19).
- **Le toast enrichi** : `kreekToast(message, type, options)` — le troisième
  argument, facultatif, porte sous-titre et lien ; avec un lien, le toast dure
  6 s et le sous-titre est cliquable (décision 14). Sans `options`, rien ne
  change : mêmes appels, même `showToast`, mêmes 2,5 s. Le type `warning`,
  ignoré aujourd'hui (`app-layout.html:98-100`), n'est **pas** corrigé ici.
- **`Lagged`** : `warn!` avec `n`, et `realtimeResync` envoyé aux connexions
  présentes sur un match.
- **Le journal** : `notification routée` en `info` (`event_id`,
  `match_report_id`, `destinataires`, `connectes`, `livres`) ; ni texte de toast
  ni nom de joueur.

## Fichiers touchés

`src/web/realtime/` (`notification_router.rs`, `toast.rs`), `src/web/ports.rs`,
`src/infrastructure/web/match_audience_adapter.rs`, `src/main.rs`,
`src/web/templates/app-layout.html`, `assets/static/css/components/toast.css`.
Et, arbitré le 2026-11-02, `ITeamRepository::find_coach_ids_in_space`
(`src/app/teams/ports.rs`, `team_repository.rs` et ses trois doublures) : une
lecture de `team_proj` — `coach_id`, jamais `coach_name` (carte 640) — plutôt
que deux rejeux d'agrégat par fait saisi.

## Tests

- **Unitaires**, doublures du port et registre réel : l'auteur ne reçoit rien ;
  l'autre coach reçoit sur ses deux onglets ; un coach d'un autre match ne
  reçoit rien ; une équipe d'un autre espace n'est pas destinataire ; un fait
  sans toast n'en produit pas ; `Lagged` produit `realtimeResync`. La mise en
  forme : une phrase par variant, la sélection vide de coups de pouce, le lien.
- **Adapter** (`#[sqlx::test]`) : une équipe d'un autre espace ne rend pas son
  coach.
- **E2E** (`tests/e2e/test_realtime_notifications.py`) : DevCoach saisit un
  touchdown, le membre simple — coach de l'autre équipe, sur une autre page de
  l'application — voit le toast avec son lien, qui mène à `/live`. Un troisième
  coach, connecté par `/auth/login` en `E2E Coach 02` / `SEED_PASSWORD`
  (`src/cli/seed_e2e.rs:37`), ne voit rien — constaté **après** le témoin
  positif. Les toasts de `team_creation` restent identiques
  (`test_team_creation*` inchangés et verts).
- `tests/impact-map.toml` mis à jour.

## Hors périmètre

- Le rechargement des widgets sur signal : 646.
- Une boîte de notifications, l'historique, l'e-mail : exclus par l'épic.
- La couleur des toasts `warning` (§ 12 de la fiche).

## Conséquences sur les données de production

Aucune : les notifications sont éphémères, rien n'est écrit. Un coach
déconnecté au moment du fait ne le recevra jamais. Un toast arrive dans chaque
onglet ouvert du destinataire.

## Terminé quand

- [ ] Un touchdown saisi par Morgane fait apparaître, chez le coach de l'autre équipe, « Touchdown : Griff #9 (London Rats) — par Morgane » avec le lien « Voir le match » vers `/live`
- [ ] Morgane ne reçoit pas le toast de son propre fait
- [ ] Un coach étranger au match ne reçoit rien
- [ ] La validation des coups de pouce affiche la liste de ce qui a été pris
- [ ] Les toasts existants, de `team_creation` et du calendrier, sont inchangés
- [ ] `make lint`, `make check-arch`, `make test`, e2e impactés
