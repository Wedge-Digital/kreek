# Valider son camp

**Épic :** E22 — Le mode « Chacun son équipe » · **Ordre :** 8 · **Dépend de :** 658
**Conception :** `docs/specs/saisie-mobile/E22-cadrage.md` (R9, § 3, § 5, § 12.1, § 12.4 à 12.7)
**Maquette :** `app-mobile-duo-validate.html` (B5)

## Le constat

- La publication est un clic de qui peut écrire : `ReadyToPublish` →
  `Published`, `published_by` = qui clique
  (`src/app/match_report/domain/match_report_ready_to_publish.rs:253-263`,
  `use_cases/publish_match_report_use_case.rs:36-72`, garde
  `io/web/recap_controller.rs:477-508`).
- **`recap.html` ne déclare pas sa présence** : la validation de l'autre n'y
  arriverait pas.
- L'option « Chacun son équipe » est grisée depuis E21
  (`templates/match_report/entry-mode-card.html:19-26`).

## Ce qui change

- **`TeamSideValidated { team_side, validated_by: CoachId }`** : exige le MVP
  et l'après-match de son camp ; se rejoue en `PreMatch` comme en
  `ReadyToPublish` (§ 12.4). **La seconde validation** appende
  `TeamSideValidated` puis `MatchReportPublished` (`published_by` = second
  validateur) dans le même `append_many`.
- **Un camp validé est figé** : toute écriture dans ce camp est refusée (409)
  — fan factor, actions, MVP, après-match (§ 12.5).
- **`TeamSideValidationWithdrawn { team_side, withdrawn_by }`** : « Retirer ma
  validation », tant que l'autre n'a pas validé (§ 12.5).
- **`POST /recap/publish`** en mode B : valide le camp du coach (l'admin choisit
  le camp, § 12.6) ; publie si l'autre a validé. Mode A inchangé.
- **Le récapitulatif non publié** en mode B : « À valider » / « Validé ✓ » pour
  chaque camp, le bouton « Valider les [mon équipe] » (« … et publier » si
  l'autre a validé), « Retirer ma validation » ; « validé par » pour chaque
  camp à côté du « Publié par » de E19. La page déclare sa présence ; le bloc
  des validations se recharge sur `matchChanged`, la page entière à la
  publication.
- **App events** : `MatchSideValidated` (toast « Les London Rats sont validés
  par Morgane », § 12.7, `TOAST_EVENT_TYPES` à cinq) ;
  `MatchSideValidationWithdrawn` (signal).
- **L'option « Chacun son équipe » s'ouvre** dans « Qui saisit ? » (§ 12.1),
  masquée à un coach des deux équipes (§ 12.9).

## Fichiers touchés

`domain/events.rs`, les trois états, `domain/match_report_state.rs`,
`domain/error.rs`, un use case de validation et de retrait,
`use_cases/publish_match_report_use_case.rs`, `io/web/recap_controller.rs`,
`templates/recap.html`, `io/web/match_selection_controller.rs`,
`templates/match_report/entry-mode-card.html`,
`io/app_events/app_event_publisher.rs`, `io/repository/match_report_repository.rs`,
`src/app/shared_kernel/app_events/match_report_app_events.rs`,
`src/web/realtime/toast.rs` ; `tests/e2e/test_match_report_entry_mode.py`,
`tests/impact-map.toml`, `tests/e2e/visual/urls.py`.

## Règles et invariants

- Chacun valide son camp ; la seconde validation publie (R9).
- Valider exige son MVP et son après-match, pas ceux de l'autre (§ 12.4).
- Un camp validé ne s'écrit plus ; sa validation se retire tant que l'autre
  n'a pas validé (§ 12.5).
- L'événement publié et ses consommateurs ne changent pas.

## Tests

- **Unitaires** : validation sans après-match refusée ; seconde validation →
  deux événements dans un append ; écriture dans un camp validé refusée ;
  retrait possible puis impossible après la validation adverse ; toast de
  `MatchSideValidated`.
- **E2E** : deux contextes ; le visiteur valide le premier, le domicile voit
  « Validé ✓ » et le bouton « … et publier » sans rechargement, reçoit le
  toast ; sa validation publie, les deux pages montrent le rapport publié ; la
  carte « Qui saisit ? » propose « Chacun son équipe »
  (`test_match_report_entry_mode.py:26-37` migré).

## Conséquences sur les données de production

Deux événements **ajoutés**, propres au mode B ; aucune colonne. La publication
émet le même `MatchReportPublished` qu'aujourd'hui ; `published_by` est le
second validateur. À partir de cette carte, des comptes rendus réels peuvent
naître en mode B.

## Terminé quand

En mode B, chaque coach valide son équipe, la seconde validation publie, et le
récapitulatif dit qui a validé chaque camp.
