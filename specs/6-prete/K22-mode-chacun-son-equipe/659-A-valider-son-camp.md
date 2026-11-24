---
id: 659-A-valider-son-camp
parent: K22
status: approved
---

# Valider son camp — la seconde validation publie

**Épic :** E22 — Le mode « Chacun son équipe » · **Ordre :** 8 · **Dépend de :** 658  
**Dépend de** : 658-les-gains-les-fans-et-le-paragraphe-de-son-camp
**Conception :** `docs/specs/saisie-mobile/E22-cadrage.md` (R9, § 3, § 4, § 12.4 à 12.6)
**Maquette :** `app-mobile-duo-validate.html` (B5)

> Première des trois cartes issues du découpage de l'ancienne 659 (2026-11-24) :
> 659-A la validation et la publication, 659-B le retrait, 659-C le direct et
> l'ouverture du mode. Le mode B ne se démarre encore que par un POST
> `entry_mode=TeamEntry`.

## Le constat

- La publication est un clic de qui peut écrire : `ReadyToPublish` →
  `Published`, `published_by` = qui clique
  (`src/app/match_report/domain/match_report_ready_to_publish.rs:297`,
  `use_cases/publish_match_report_use_case.rs:36-72`, un seul `append`).
- La garde de `POST /recap/publish` est `authorize_recap_action`
  (`io/web/recap_controller.rs:480-498`), règle `can_write_report` : en mode B
  elle laisse passer les deux coachs (652). Le camp n'y est pas lu.
- Rien ne retient qu'un camp est validé ; le récapitulatif non publié n'offre
  que « Publier → » (`io/web/templates/recap.html:222-229`).
- **Les corrections d'un camp passent par `into_pre_match()`** quand le rapport
  est en `ReadyToPublish` (`use_cases/record_fan_factor_use_case.rs:107`,
  `use_cases/record_action_use_case.rs:194`, `io/web/mvp_step_controller.rs:231`,
  `io/web/team_step5_controller.rs:144`) : l'état des validations doit survivre
  à l'aller-retour `into_pre_match` / `from_pre_match`, sans quoi le gel se
  contourne.
- Le droit par camp existe : `can_write_side`
  (`use_cases/match_report_access_service.rs:181-197`) et `camp_du_coach`.

## Ce qui change

- **`TeamSideValidated { team_side, validated_by: CoachId }`** : exige le MVP
  et l'après-match **de ce camp** (`post_match_of(side)`), pas ceux de l'autre ;
  se rejoue en `PreMatch` comme en `ReadyToPublish` (§ 12.4) et passe
  `into_pre_match` / `from_pre_match`. Valider un camp déjà validé est refusé.
- **La seconde validation** appende `TeamSideValidated` puis
  `MatchReportPublished` (`published_by` = second validateur) dans le même
  `append_many`. Elle a toujours lieu en `ReadyToPublish` : le second camp ne
  peut avoir son après-match qu'avec `PostMatchClosed`.
- **Un camp validé est figé** (§ 12.5) : le domaine refuse toute écriture dans
  ce camp — fan factor, ajout et suppression d'action, MVP choisi ou tiré,
  après-match. Erreur domaine dédiée, rendue en 409.
- **`POST /recap/publish` en mode B valide un camp** : celui du coach, d'après
  `camp_du_coach`, jamais d'après le formulaire ; l'admin choisit le camp par
  un champ `team_side` (§ 12.6). Mode A inchangé.
- **Le récapitulatif non publié en mode B**, rendu par le serveur : pour
  chaque camp « À valider » ou « Validé ✓ par [coach] » ; le bouton
  « Valider les [mon équipe] », « … et publier » si l'autre a validé ; l'admin
  a un bouton par camp non validé. Pas de rechargement en direct : c'est la
  659-C.
- **Le premier camp valide depuis l'écran d'attente** (décision du
  2026-11-24) : le récapitulatif n'existe pas en `PreMatch`
  (`io/web/recap_controller.rs:247-258`), et le camp qui a fini son
  après-match attend l'autre sur `team-step5`
  (`io/web/team_step5_controller.rs:328-332`). Cet écran d'attente porte
  « Valider les [mon équipe] », puis « Validé ✓ » ; la validation y suit la
  même commande que sur le récapitulatif. Le récapitulatif reste réservé à
  `ReadyToPublish`.
- **App event `MatchSideValidated`** (signal, sans toast — le toast est à la
  659-C) : l'aiguillage du publisher, `MatchReportAppEvent::event_type` /
  `match_report_id` et `toast::notice_of` sont exhaustifs, l'ajout s'y range.
  La projection n'a pas de colonne à changer : l'événement rejoint le bras sans
  effet de `update_projection_in_tx`.

## Fichiers touchés

`domain/events.rs`, `domain/match_report_pre_match.rs`,
`domain/match_report_ready_to_publish.rs`, `domain/match_report_state.rs`,
`domain/team_post_match.rs`, `domain/error.rs`, un use case de validation,
`use_cases/publish_match_report_use_case.rs`, `io/web/recap_controller.rs`,
`io/web/templates/recap.html`, `io/web/team_step5_controller.rs`,
`io/web/templates/team-step5.html`, `io/app_events/app_event_publisher.rs`,
`io/repository/match_report_repository.rs`,
`src/app/shared_kernel/app_events/match_report_app_events.rs`,
`src/web/realtime/toast.rs` (bras sans toast) ;
`tests/e2e/test_match_report_team_post_match.py` ou un fichier voisin,
`tests/impact-map.toml`.

(Chemins sous `src/app/match_report/` sauf mention.)

## Règles et invariants

- Chacun valide son camp ; la seconde validation publie (R9).
- Valider exige son MVP et son après-match, pas ceux de l'autre (§ 12.4).
- Un camp validé ne s'écrit plus, par aucune route (§ 12.5).
- Le camp validé par un coach est le sien, jamais celui que dit le formulaire ;
  l'admin valide l'un ou l'autre (§ 12.6).
- L'événement publié et ses consommateurs ne changent pas ; le mode A publie
  comme aujourd'hui.

## Tests

- **Unitaires** : validation sans après-match refusée, sans MVP refusée ;
  validation en `PreMatch` rejouée, et conservée par `into_pre_match` ; seconde
  validation → deux événements dans un append, `published_by` = second
  validateur ; écriture dans un camp validé refusée pour chaque famille (fan
  factor, action, suppression, MVP, tirage, après-match) ; double validation
  refusée ; un coach qui poste `team_side` de l'autre valide le sien ; mode A
  inchangé ; `MatchSideValidated` sort du publisher.
- **E2E** (deux contextes, mode B démarré par POST) : le visiteur valide, le
  domicile recharge et voit « Validé ✓ par … » et le bouton « … et publier » ;
  sa validation publie ; une écriture directe dans un camp validé rend 409.

## Conséquences sur les données de production

Un événement **ajouté**, propre au mode B ; aucune colonne. La publication
émet le même `MatchReportPublished` qu'aujourd'hui ; `published_by` est le
second validateur. Aucun rapport réel n'est en mode B tant que la 659-C n'a pas
ouvert l'option.

## Terminé quand

En mode B, chaque coach valide son équipe, un camp validé ne s'écrit plus, la
seconde validation publie, et le récapitulatif dit qui a validé chaque camp.
