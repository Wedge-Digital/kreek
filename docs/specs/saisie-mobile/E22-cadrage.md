# E22 — Le mode « Chacun son équipe » — cadrage

Fiche de cadrage de l'épic `kanban/epics/ready_to_be_done/E22-mode-chacun-son-equipe.md`,
**brouillon du 2026-11-11, décisions reportées au § 12**, lu sur `6560b4bd`
(branche `demo`). Les décisions de l'épic (2026-11-01) ne sont pas
rediscutées ; ce que le code contredit est au § 11, les questions du § 10 sont
tranchées au § 12, et ce qui change dans l'épic est au § 13.

## 1. Ce que le code fait aujourd'hui

Chemins relatifs à `src/app/match_report/` sauf mention contraire.

**Le mode de saisie**

- `EntryMode` n'a qu'une variante, `SoloEntry` (`domain/value_objects.rs:44-47`) ;
  le commentaire `:40-42` réserve la seconde à E22, « avec ce qui la fait exister ».
- `EntryModeChosen { mode, started_by }` (`domain/events.rs:45-48`) est appendé
  avec `SelectionConfirmed` (`use_cases/create_match_report_use_case.rs:147-160`) ;
  le rejeu pose `SoloEntry` à la confirmation puis le remplace
  (`domain/match_report_state.rs:100-111`) ; la projection fait le même pli
  (`io/repository/match_report_repository.rs:89-118`), colonne
  `match_report_proj.entry_mode TEXT NOT NULL DEFAULT 'SoloEntry'`, sans contrainte
  de valeur (`migrations/20261101000001_match_report_proj_entry_mode.sql:16`).
- Le formulaire : `read_entry_mode` n'accepte que `SoloEntry` ou l'absence
  (`io/web/match_selection_controller.rs:363-366`) ; la carte « Qui saisit ? »
  montre « Chacun son équipe » désactivée, sans valeur
  (`io/web/templates/match_report/entry-mode-card.html:19-26`).
- **« TeamEntry » sert aujourd'hui d'exemple de mode inconnu** dans trois tests :
  `match_selection_controller.rs:726`, `use_cases/match_report_access_service.rs:481-483`,
  `tests/e2e/test_match_report_entry_mode.py:40-57`.

**Le droit d'écrire et le camp**

- `can_write_report` (`use_cases/match_report_access_service.rs:114-128`) :
  admin toujours, tiers jamais, puis `coach_may_write` (`:131-148`) — brouillon :
  tout coach ; mode inconnu : **refus** (`:140-143`) ; `SoloEntry` : le démarreur,
  ou les deux coachs si le démarreur n'en coache aucune équipe (`:155-165`).
- **Le droit est par compte rendu, jamais par camp.** `camp_du_coach`
  (`:219-235`, carte 621) répond Domicile / Visiteur / LesDeux / Aucun, mais
  aucune garde ne s'en sert pour écrire.
- `can_delete_actions` n'est qu'un alias de `can_write_report` (`:63-71`) : le ✕
  du journal est le même pour toutes les lignes (`io/web/widgets/action_log_widget.rs:88-89`),
  seul un MVP tiré l'efface.

**Les gardes de route**

- `garde_suivi` / `garde_saisie` (`io/web/garde_saisie.rs:51-80`), posées par
  `route_layer` sur deux groupes (`router.rs:64-70`, `96-166`). Elles ne lisent
  dans le chemin que `space_id` et `match_report_id` (`garde_saisie.rs:128-139`).
- Refus d'écriture : 403 sur un POST/DELETE ou une requête htmx, 303 vers `/live`
  pour une page complète (`garde_saisie.rs:179-187`).
- Le camp n'est porté par le chemin que pour quatre familles de routes :
  `/inducements/{team_id}`, `/step2/{team_id}/mercenaires`, `/mvp/{side}`,
  `/mvp/{side}/draw` (`routes.rs:6-7`, `22-25`, `47-48`) ; `step3*`/`step4*`
  sont des routes distinctes par camp (`router.rs:130-165`). **`step2`, `step5`,
  `cancel-inducements`, `/actions/{action_id}` et `/recap/*` ne portent pas de camp.**
- Publication et dépublication se gardent elles-mêmes par `can_write_report`
  (`io/web/recap_controller.rs:477-508`).

**L'avant-match et `FanFactorRecorded`**

- `FanFactorRecorded` porte les **deux** jets et les deux `DedicatedFans`
  (`domain/events.rs:49-57`) ; le formulaire exige les deux jets
  (`io/web/pre_match_controller.rs:167-171`).
- Le use case appende **toujours** `FanFactorRecorded` puis `TeamValuesRecorded`,
  les valeurs d'équipe **relues à chaque fois** (`use_cases/record_fan_factor_use_case.rs:101-122`) :
  rien ne les fige aujourd'hui. Il n'a pas de bus et **n'émet rien**
  (`:51-68`).
- Rejeu : le dernier `FanFactorRecorded` gagne, en `PreMatch` comme en
  `ReadyToPublish` (`match_report_state.rs:119-135`, `331-347`). Projection :
  `FanFactorRecorded` n'écrit rien (`match_report_repository.rs:119`),
  `TeamValuesRecorded` écrase `home_team_value`/`away_team_value` (`:120-135`).
- `into_pre_match` remet les valeurs d'équipe à `None` (`domain/match_report_ready_to_publish.rs:191-192`).
- Base de dev (lecture) : 436 `FanFactorRecorded`, 436 `TeamValuesRecorded` ;
  97 rapports ont enregistré leur fan factor plusieurs fois, aucun n'a vu ses
  valeurs d'équipe changer entre deux enregistrements.

**Les coups de pouce**

- **L'ordre favori/outsider existe déjà** (carte 627) : `check_inducements_open_for`
  refuse l'outsider tant que le favori n'a pas validé (`DomainError::TopdogHasNotChosen`,
  `domain/match_report_pre_match.rs:269-281`), rendu en **409**
  (`io/web/inducements_controller.rs:442-449`).
- « Passer » enregistre une liste vide (`templates/inducements.html:169`,
  `use_cases/record_inducements_use_case.rs:98-100`) : **`InducementsRecorded`
  vide dit déjà « le favori a fini ses achats »** (`domain/events.rs:106-110`).
- `inducements_turn` (`pre_match.rs:286-299`) ; le GET redirige vers la page de
  l'équipe dont c'est le tour (`inducements_controller.rs:187-201`) : l'outsider
  est renvoyé chez le favori, il n'a **pas d'écran d'attente**.
- L'annulation globale (E20, carte 626) : `cancel_inducements` (`pre_match.rs:357-376`),
  refusée dès qu'une action vivante existe (`:378-389`). Elle est émise
  (`use_cases/cancel_inducements_use_case.rs:96-100`) et notifiée (E23).
- `InducementsRecorded` est émis et sort en `MatchInducementsRecorded`
  (`io/app_events/app_event_publisher.rs:180-190`), avec un toast
  (`src/web/realtime/toast.rs:89`).

**Les actions (étapes 3/4)**

- Stockées par camp : `home_actions`/`away_actions`, `ActionRecorded.team_side`
  (`domain/events.rs:143-153`) ; `match_report_actions.recorded_by` (E21, carte 635).
- Un blessé s'inscrit au camp du joueur blessé (`MatchActionType::Blesse`,
  `domain/value_objects.rs:390-398`) : chaque coach saisit naturellement les
  faits de ses joueurs.
- `/live` : « ＋ Nouvelle action » ouvre un choix d'équipe
  (`templates/live-match.html:85-108`), liens `step3`/`step4`
  (`io/web/live_match_controller.rs:224-231`) ; le bandeau « X saisit tout le
  match » et la présence ne s'affichent qu'à qui ne peut pas écrire
  (`live-match.html:50-67`).
- `delete_action` retrouve le camp de l'action dans l'agrégat
  (`pre_match.rs:598-628`) ; la route `DELETE /actions/{action_id}` ne le porte pas.

**Le MVP**

- `record_mvp` : un par camp, tour 16 (`pre_match.rs:484-505`). L'étape visiteur
  **renvoie à l'étape domicile** tant que le domicile n'a pas son MVP
  (`io/web/mvp_step_controller.rs:127-134`) ; la suite du domicile est le MVP
  visiteur, celle du visiteur l'étape 5 (`:339-345`) ; `/live` mène à `mvp/home`
  (`live_match_controller.rs:229`).

**L'étape 5 et `PostMatchRecorded`**

- Un seul événement pour les deux camps : gains, bonus, `FanFactorMod`, titre et
  corps du résumé (`domain/events.rs:170-186`). `record_post_match` exige le MVP
  **des deux camps** (`pre_match.rs:697-712`) et fait passer `PreMatch` →
  `ReadyToPublish` (`:722-734`, rejeu `match_report_state.rs:296-322`) ; en
  `ReadyToPublish`, il remplace les valeurs (`match_report_ready_to_publish.rs:207-246`,
  rejeu `match_report_state.rs:465-490`). Projection : `phase = 'ReadyToPublish'`
  (`match_report_repository.rs:297-308`).
- `GET step5` renvoie à l'étape MVP du premier camp sans MVP (`io/web/step5_controller.rs:153-163`).
- `record_post_match_use_case` n'émet rien (`use_cases/record_post_match_use_case.rs:53-70`).

**Le résumé** — revérifié : `summary_title`/`summary_body` n'existent que dans
le BC (événement, trois états, `step5`, `recap`). Pas de colonne en projection,
rien dans `MatchReportPublishedPayload` (`src/app/shared_kernel/app_events/match_report_app_events.rs:189-217`).
**Le récapitulatif n'affiche le résumé — et le « Publié par » — que si un titre
existe** (`templates/recap.html:131-145`).

**La publication**

- `publish` : `ReadyToPublish` → `Published`, `published_by` = qui clique
  (`match_report_ready_to_publish.rs:253-263`, `use_cases/publish_match_report_use_case.rs:36-72`),
  un seul `append`, émis.
- Le publisher relit l'agrégat publié et construit le payload
  (`app_event_publisher.rs:392-410`, `746-780`) : `published_by` **ne sort pas**
  du BC. Consommateurs de `MatchReportPublished` : `teams`, `ranking`,
  `competitions` (`*/io/app_events/match_report_published_listener.rs`).
  **`players` ne l'écoute pas** : il reçoit les `PlayerMatchImpactAppEvent` que
  le publisher émet dans la foulée (`app_event_publisher.rs:407`, `496-560`).
- Le payload lit `home_gain`, `home_fan_mod`… sur l'agrégat : si les événements
  par camp remplissent ces mêmes champs, **rien ne change en aval**.

**La correction**

- `correction_eligibility_service::evaluate` : bloqué si des SPP ont été
  dépensés depuis le match ou si une équipe est en phase d'amélioration
  (`use_cases/correction_eligibility_service.rs:19-58`). `unpublish` rend un
  `ReadyToPublish` marqué `was_published_before` (`domain/match_report_published.rs:81-95`),
  émis (`use_cases/unpublish_match_report_use_case.rs:47-90`).

**Le démarrage concurrent** — **déjà traité par E21** :
`POST /{id}` sur un rapport déjà démarré rend 409 + rechargement, et l'on arrive
sur l'état du match (`match_selection_controller.rs:683-705`) ; `POST /new` sur
une rencontre au calendrier déjà démarrée réussit en silence
(`create_match_report_use_case.rs:123-142`) puis ouvre `/{id}`, qui aiguille
`PreMatch` → `step2` (`match_selection_controller.rs:259-261`). Le premier choix
validé l'emporte ; l'autre n'est pas dit au second coach.

**L'écriture concurrente** — une seule version par flux : la contrainte
`match_report_es_version` refuse toute seconde écriture simultanée
(`match_report_repository.rs:393-400`, `522-528`), rendue en 409 +
`HX-Refresh` (`io/web/write_conflict.rs`). En « Je saisis tout », un seul
saisisseur : le cas est rare. En mode B, deux coachs écrivent en même temps
sur des camps indépendants.

**Ce qui est déjà par camp / ce qui saisit encore les deux d'un coup**

| | Par camp | Les deux d'un coup |
|---|---|---|
| Fan factor | — | `FanFactorRecorded`, formulaire à deux jets |
| Valeurs d'équipe | — | `TeamValuesRecorded` (doit le rester : favori et petite monnaie) |
| Coups de pouce | `InducementsRecorded { team_id }`, ordre R9 | annulation globale (voulu) |
| Actions, MVP | `team_side` | ✕ et garde par rapport |
| Après-match | — | `PostMatchRecorded`, MVP exigé des deux camps |
| Résumé | — | un titre, un corps |
| Publication | — | `MatchReportPublished`, un clic |

## 2. Les règles

| # | Règle |
|---|---|
| R1 | En « Chacun son équipe », **chaque coach écrit dans son camp seul** : fan factor, coups de pouce, actions, MVP, gains, fan factor d'après-match, paragraphe, validation. Le camp de l'autre est en lecture seule. |
| R2 | Le **camp** d'un coach est celui de `camp_du_coach`. Deux équipes d'un même coach ne démarrent pas en mode B, quel que soit le démarreur (§ 12, Q9, élargi). L'admin écrit et valide les deux camps, titre compris (§ 12, Q6). |
| R3 | Écrire dans le camp de l'autre est refusé **par la route** : 403 sur une écriture ou un fragment, 303 vers sa propre page pour une page complète (forme de E21). |
| R4 | Les **valeurs d'équipe** se figent au premier fan factor de mode B, pour les deux équipes ; le second fan factor ne les relit pas. |
| R5 | **Coups de pouce** : le favori achète d'abord ; « J'ai fini mes achats » est la validation existante, même vide. L'outsider voit un écran d'attente qui s'ouvre quand le favori a validé. L'annulation globale reste ouverte aux deux coachs, aux conditions de E20. |
| R6 | Une action, un MVP ne s'écrivent et ne s'effacent que dans son camp. |
| R7 | L'après-match d'un camp exige **le MVP de ce camp**, pas celui de l'autre. |
| R8 | **Titre** au coach à domicile ; un paragraphe par camp. |
| R9 | Chacun **valide son camp** ; valider exige son MVP et son après-match. La seconde validation publie, dans le même append ; `published_by` est le second validateur. |
| R10 | La dépublication suit `correction_eligibility_service`, par l'un ou l'autre coach ; elle **remet les deux validations à zéro**. |
| R11 | Le mode ne change pas en cours de match ; le premier démarrage validé l'emporte. |

**Ce qui ne change pas** : le mode A et ses événements ; l'événement publié et
ses consommateurs ; l'éligibilité à la correction ; le rejeu de tout flux
existant.

## 3. Les événements

Tous appendés par les use cases, **émis** par `emettre` (aujourd'hui ni
`FanFactorRecorded` ni `PostMatchRecorded` ne le sont), et rejoués à côté des
formes du mode A, qui restent intactes.

| Événement | Carte | Champs | `apply()` / rejeu | Sortie du BC |
|---|---|---|---|---|
| `EntryModeChosen` | 652 | inchangé ; `mode` reçoit `EntryMode::TeamEntry` | inchangé | bras groupé, inchangé |
| `TeamFanFactorRecorded` | 654 | `team_side: TeamSide`, `fan_roll: D3Roll`, `dedicated_fans: DedicatedFans`, `recorded_by: CoachId` | pose le jet et les fans **du seul camp** ; valide en `PreMatch` et `ReadyToPublish`, comme `FanFactorRecorded` | **sort** : `MatchFanFactorRecorded` (signal, sans toast) |
| `TeamValuesRecorded` | 654 | inchangé | inchangé ; **appendé avec le premier fan factor seulement** | bras groupé |
| *(aucun)* « favori a fini » | 655 | — | `InducementsRecorded` vide existe déjà | déjà `MatchInducementsRecorded` |
| `TeamPostMatchRecorded` | 658 | `team_side`, `gain: MatchGain`, `no_stalling_bonus: NoStallingBonus`, `fan_mod: FanFactorMod`, `summary_title: Option<String>` (domicile seul), `summary_paragraph: Option<String>`, `recorded_by` | remplit `home_gain`… **du seul camp** — les mêmes champs que le mode A, pour que le payload publié ne change pas — et le paragraphe | **sort** : `MatchPostMatchRecorded` (signal) |
| `PostMatchClosed` | 658 | aucun | `PreMatch` → `ReadyToPublish` ; **appendé avec le second `TeamPostMatchRecorded`**, même `append_many` (§ 12, Q8) | bras groupé |
| `TeamSideValidated` | 659 | `team_side`, `validated_by: CoachId` | marque le camp validé ; en `PreMatch` ou `ReadyToPublish` | **sort** : `MatchSideValidated` (signal + toast « London Rats validés par Morgane ») |
| `MatchReportPublished` | 659 | inchangé | inchangé ; appendé **avec** la seconde validation | inchangé |
| `MatchReportUnpublished` | 660 | inchangé | remet aussi les deux validations à zéro | inchangé |
| `TeamSideValidationWithdrawn` | 659 (§ 12, Q5) | `team_side`, `withdrawn_by` | retire la validation | sort (signal) |

Les champs texte du résumé restent des `Option<String>` : exception « texte
libre sans validation domaine » de `CLAUDE.md`, comme aujourd'hui.

**Les noms** disent le fait du domaine (« le fan factor d'une équipe est
enregistré »), pas le mode qui le produit. Préfixe `Team` plutôt que `Side`
pour coller à `ActionRecorded.team_side` et au vocabulaire du jeu ; à arbitrer
à la relecture, sans conséquence.

**L'aiguillage du publisher** est exhaustif sans joker
(`app_event_publisher.rs:107-213`) : chaque événement ajouté casse la
compilation et se range — il sort, ou rejoint le bras groupé `:202-212`. Il en
va de même de `MatchReportAppEvent::event_type`/`match_report_id`
(`match_report_app_events.rs:287-311`), de `toast::notice_of`
(`src/web/realtime/toast.rs:84-100`) et de `update_projection_in_tx`
(`match_report_repository.rs:19-350`). Les listeners de `teams`, `ranking`,
`competitions` et `players` ne sont pas exhaustifs (constat E23) : rien n'y casse.

**Pourquoi des app events pour le signal** : `MatchChanged` ne part que d'un app
event lu par `notification_router::handle` (`src/web/realtime/notification_router.rs:70-83`).
Un fait qui ne sort pas du BC ne réveille aucune page.

## 4. Les routes

Mêmes routes qu'en « Je saisis tout ».

| Route | Mode A (inchangé) | Mode B |
|---|---|---|
| `GET /new`, `GET /{id}` (brouillon) | « Qui saisit ? » | l'option « Chacun son équipe » s'ouvre (659, § 12, Q1) |
| `POST /new`, `POST /{id}` | `SoloEntry` | accepte `TeamEntry` (652) |
| `GET /{id}` (`PreMatch`) | `step2` | `step2` (652) |
| `GET/POST /step2` | les deux jets | son jet ; celui de l'autre en lecture seule, ✓ « lancé par » (654) |
| `GET/POST /inducements/{team_id}`, `/step2/{team_id}/mercenaires` | redirige vers le tour | **son équipe seule** ; outsider avant la validation du favori : écran d'attente au lieu de la redirection (655) |
| `POST /cancel-inducements` | démarreur, admins | l'un ou l'autre coach (655) |
| `GET /live` | « Nouvelle action » + choix d'équipe | « ＋ Action de [mon équipe] » vers sa seule étape ; présence et état de l'adversaire affichés aux deux (656) |
| `GET /step3*`, `POST /step3/actions` | démarreur, admins | coach domicile (656) |
| `GET /step4*`, `POST /step4/actions` | idem | coach visiteur (656) |
| `DELETE /actions/{id}` | ✕ partout | son camp seul ; ✕ par ligne (656) |
| `GET/POST /mvp/{side}`, `/mvp/{side}/draw` | domicile puis visiteur | son camp, sans attendre l'autre ; suite : `step5` (657) |
| `GET/POST /step5` | les deux camps | son camp ; l'autre en lecture seule ; titre au domicile (658) |
| `GET /recap` non publié | « Publier » | « Valider [mon équipe] » ; « validé par » pour chaque camp (659) |
| `POST /recap/publish` | publie | **valide son camp** ; publie si l'autre a validé (659) |
| `POST /recap/unpublish` | démarreur, admins | l'un ou l'autre coach (660) |

**Le prédicat**, à côté de `can_write_report` :

```rust
/// Peut écrire dans ce camp : admin (§ 12, Q6), coach de ce camp en mode B,
/// et la règle de `can_write_report` en mode A.
pub async fn can_write_side(deps, user, space_id, scope: &PorteeRapport,
                            entry: &EntryScope, side: TeamSide) -> bool;
```

`coach_may_write` (`match_report_access_service.rs:131-148`) gagne une branche
`TeamEntry` qui ouvre le rapport aux deux coachs ; c'est `can_write_side` qui
coupe ensuite.

**La garde par camp** : un troisième `route_layer`, `garde_camp`, sur les routes
dont le chemin désigne le camp — `team_id` (comparé à la portée), `side`, ou le
groupe lui-même pour `step3*` (domicile) et `step4*` (visiteur). Même refus que
`garde_saisie`. Les routes sans camp dans le chemin (`step2`, `step5`,
`/recap/publish`) prennent le camp **du coach**, jamais du formulaire : écrire
chez l'autre y est impossible par construction. `DELETE /actions/{id}` lit le
camp de l'action dans l'agrégat ; le domaine refuse (`ActionOutsideSide`, à
créer) — 403.

## 5. Le temps réel

Ce qui existe (E23) : `MatchChanged` à toutes les connexions **présentes** sur le
match (`src/web/realtime/registry/presence.rs:118-132`) ; la présence ne
s'inscrit que sur `actions-step.html:7`, `mvp-step.html:7`, `live-match.html:12`.
Toasts : démarrage, action, coups de pouce validés, annulés (`toast.rs:39-44`).

Ce que E22 branche :

| Attente | Déclencheur | Ce qu'il faut |
|---|---|---|
| L'outsider attend le favori (655) | `MatchInducementsRecorded` (existe) | présence sur `inducements.html` ; l'écran d'attente recharge sur `matchChanged` et ouvre la page d'achat quand c'est son tour ; le toast existe |
| L'avant-match de l'autre (654) | `MatchFanFactorRecorded` (nouveau) | présence sur `pre-match.html` ; le bloc « adversaire » en widget |
| Les gains de l'autre (658) | `MatchPostMatchRecorded` (nouveau) | présence sur `step5.html` ; bloc « adversaire » en widget, **le formulaire du coach n'est jamais rechargé** (il perdrait sa saisie) |
| Les validations (659) | `MatchSideValidated` (nouveau), puis `MatchReportPublished` | présence sur `recap.html` ; le bloc des validations recharge ; à la publication, la page recharge |

`notification_router` ne change pas : il signale tout app event de match et
route les toasts que `notice_of` lui donne. `notice_of` gagne
`MatchSideValidated` ; `TOAST_EVENT_TYPES` aussi (`toast.rs:39-44`). Le
`MatchReportConfirmed` ne porte pas le mode : le toast de démarrage reste
« Saisie du match démarrée par … », et son lien mène à `/live`
(`toast.rs:104-108`). En mode B, `/live` doit renvoyer à `step2` un coach dont
l'avant-match n'est pas fini (656, § 12, Q3).

## 6. Conséquences sur les données de production

Vérifié : **aucun compte rendu n'est en mode B**, et aucun ne peut l'être —
`EntryMode` n'a qu'une variante, un autre mode ne se désérialiserait pas
(`value_objects.rs:44-47`). Base de dev : 1866 lignes de `match_report_proj`,
toutes `SoloEntry` ; un seul `EntryModeChosen`, `SoloEntry`.

| Carte | Données | Effet sur l'existant |
|---|---|---|
| 652 | une **valeur** ajoutée, `TeamEntry`, dans `EntryModeChosen.mode` et `match_report_proj.entry_mode` ; pas de migration | aucun |
| 653 | aucune | une écriture concurrente en mode A est réessayée au lieu d'un 409 (§ 12, Q2) |
| 654 | un événement ajouté | `FanFactorRecorded` reste celui du mode A, valeurs d'équipe toujours relues en mode A |
| 655 | aucune | aucune |
| 656, 657 | aucune | aucune |
| 658 | deux événements ajoutés ; paragraphes dans l'agrégat seulement | `PostMatchRecorded` inchangé ; le payload publié est assemblé des mêmes champs. Le récapitulatif doit afficher le résumé **sans titre** (`recap.html:131`), sans quoi le paragraphe visiteur disparaît quand le domicile n'écrit pas de titre |
| 659 | deux événements ajoutés (§ 12, Q5) ; pas de colonne | `MatchReportPublished` inchangé ; `published_by` = second validateur |
| 660 | aucune | règle d'éligibilité inchangée |

Aucune projection nouvelle n'est nécessaire : les pages lisent l'agrégat, la
garde lit `entry_mode` et la portée déjà projetées. La projection de la phase
passe à `ReadyToPublish` sur `PostMatchClosed` (§ 12, Q8).

## 7. Les tests

**Unitaires, par carte**

- 652 : `read_entry_mode("TeamEntry")` ; `can_write_side` — coach de son camp,
  de l'autre, des deux, admin, tiers, mode A inchangé ; les trois tests qui
  prennent `TeamEntry` pour un mode inconnu passent à une autre valeur.
- 653 : un conflit sur une commande de camp est réessayé une fois puis relayé.
- 654 : rejeu d'un flux mode B (un fan factor par camp, valeurs d'équipe une
  fois) et d'un flux mode A ; refus du camp de l'autre.
- 655 : écran d'attente tant que `inducements_turn` désigne le favori ; route
  d'achat de l'outsider toujours 409 (`TopdogFirst`).
- 656 : ✕ par ligne ; suppression dans le camp de l'autre refusée.
- 657 : `mvp/away` sans MVP domicile accessible en mode B, redirigé en mode A.
- 658 : `TeamPostMatchRecorded` exige le MVP de son camp seul ; le second clôt ;
  le payload publié d'un flux mode B égale celui du mode A aux mêmes valeurs.
- 659 : seconde validation → `TeamSideValidated` + `MatchReportPublished` dans
  un append ; validation sans après-match refusée.
- 660 : la dépublication remet les deux validations à zéro ; rejeu de trois cycles.

**E2E à deux coachs** — sur le modèle de `tests/e2e/test_realtime_three_matches.py` :
deux contextes, DevCoach (domicile) et un coach réellement connecté
(`realtime_helpers.login`, `:54`) ; saisies par `http_session_of(context)`
(`:110`) pour que l'auteur soit la vraie identité ; `wait_for_stream` (`:68`)
avant d'attendre un signal ; `assert_none_after` (`:97`) pour une absence ;
`cliquer_quand_cable` après tout swap.

- `create_draft` (`match_report_helpers.py:30`) ne poste pas de mode : il gagne
  `entry_mode=None` (absent = `SoloEntry`, décision E21 § 8.2). `start_match`
  (`:124`) passe les coups de pouce d'un seul coach : il ne sert qu'au mode A.
- Ce qui casse : `test_match_report_entry_mode.py:26-37` (option « bientôt »,
  à la carte qui l'ouvre) et `:40-57` (`TeamEntry` refusé, carte 652).
- Chaque carte porte ses e2e ; la **661** rejoue le « Terminé quand » : deux
  navigateurs, chacun son fan factor, l'outsider qui attend puis achète, chacun
  ses actions, son MVP, ses gains, son paragraphe ; un POST direct dans le camp
  de l'autre sur chaque famille de routes rend 403 ; la seconde validation
  publie, le récapitulatif montre les deux paragraphes.
- `tests/impact-map.toml` et `tests/e2e/visual/urls.py` suivent les états
  ajoutés (écran d'attente, validation par camp).

## 8. Le découpage revu

Prochain numéro libre : **652** (le plus haut est 651).

| # | Carte | Apport | Fichiers | Dépend de |
|---|---|---|---|---|
| 652 | Le second coach rejoint le match | `EntryMode::TeamEntry` ; `read_entry_mode` ; branche de `coach_may_write` ; `can_write_side` ; `garde_camp` et son groupe de routes, vide d'abord ; le second coach arrive à `step2` | `value_objects.rs`, `match_selection_controller.rs`, `match_report_access_service.rs`, `garde_saisie.rs`, `router.rs` | — |
| 653 | **Deux coachs écrivent en même temps** (ajoutée) | réessai unique d'une commande de camp sur `ConcurrentWrite` | use cases d'écriture par camp, `write_conflict.rs` | 652 |
| 654 | L'avant-match de son camp | `TeamFanFactorRecorded`, gel des valeurs d'équipe, `step2` réduite, présence et widget adverse | `events.rs`, `match_report_state.rs`, `match_report_pre_match.rs`, `record_fan_factor_use_case.rs`, `pre_match_controller.rs`, `pre-match.html`, publisher, `match_report_app_events.rs` | 652, 653 |
| 655 | Le favori a fini ses achats | **sans événement** : écran d'attente, « J'ai fini mes achats », `/inducements/{team_id}` et mercenaires dans `garde_camp`, présence | `inducements_controller.rs`, `inducements.html` | 654 |
| 656 | Les actions de son camp | `step3*`/`step4*` dans `garde_camp` ; ✕ par ligne ; refus de suppression ; `/live` mode B | `action_log_widget.rs`, `live_match_controller.rs`, `live-match.html`, `delete_action_use_case.rs`, `pre_match.rs` | 652, 655 (§ 12, Q3) |
| 657 | Le MVP de son camp | `/mvp/{side}` dans `garde_camp` ; plus de renvoi au domicile ; suite vers `step5` ; état adverse | `mvp_step_controller.rs`, `mvp-step.html` | 652 |
| 658 | Les gains, le fan factor **et le paragraphe** de son camp (**fusion** de deux cartes) | `TeamPostMatchRecorded`, `PostMatchClosed`, `step5` réduite, titre au domicile, deux paragraphes au récapitulatif | `events.rs`, trois états, `record_post_match_use_case.rs`, `step5_controller.rs`, `step5.html`, `recap.html`, `view_models.rs` | 657 |
| 659 | Valider son camp | `TeamSideValidated`, publication à la seconde, « validé par », ouverture de l'option dans « Qui saisit ? » | `recap_controller.rs`, `publish_match_report_use_case.rs`, `recap.html`, `entry-mode-card.html`, `toast.rs` | 658 |
| 660 | Corriger un compte rendu à deux | dépublication par l'un ou l'autre, validations remises à zéro | `match_report_published.rs`, `match_report_state.rs`, `recap_controller.rs` | 659 |
| 661 | Les tests e2e à deux coachs | le « Terminé quand » | `tests/e2e/` | 652-660 |

**Pourquoi fusionner « Gains et fan factor » et « Deux paragraphes »** : même
formulaire, même événement. Séparées, la seconde ajouterait au
`TeamPostMatchRecorded` né une carte plus tôt un champ en `serde(default)` —
un événement retouché avant même d'avoir servi — et l'étape 5 du mode B
passerait par un état intermédiaire où le résumé resterait à deux camps dans un
formulaire qui ne l'est plus.

**Pourquoi ajouter 653** : en mode B, deux écritures simultanées sont la norme ;
aujourd'hui l'une rend 409 et recharge la page (`write_conflict.rs`), ce qui
efface un paragraphe en cours de saisie. Retenue le 2026-11-11 (§ 12, Q2).

**« Le favori a fini ses achats » perd son événement** (§ 11) mais garde sa carte :
l'écran d'attente, la garde par camp des coups de pouce et le réveil par le
direct restent à faire.

## 9. Les risques

- **Le mode B choisissable avant d'être complet.** Ouvrir l'option à la 652
  livrerait des matchs réels à moitié coupés en deux (§ 12, Q1 : écarté).
- **Les conflits de version** (653) : réessayer est sûr parce que le domaine
  revalide sur l'état relu ; il ne l'est plus si une commande dépend d'un
  calcul fait avant le chargement (valeurs d'équipe relues, budget) — à
  recalculer dans la boucle.
- **Le récapitulatif conditionné au titre** (`recap.html:131`) : un défaut
  latent que le mode B rend visible.
- **Le lien des toasts** mène à `/live` : sans la redirection de 656, le second
  coach y arrive avant son avant-match.
- **La garde lit le chemin** : une route de camp ajoutée hors de `garde_camp`
  serait ouverte à l'autre coach ; seuls les e2e de 661 le verraient. Un test
  unitaire qui parcourt les routes du groupe le fermerait.
- **`rehydrate`** (`match_report_state.rs:84-523`) dépasse déjà de loin la
  limite de 20 lignes ; chaque événement y ajoute deux bras. Extraire les bras
  par état est une refacto hors épic, à signaler plutôt qu'à faire en passant.
- **Un coach absent** bloque la publication : rien ne publie sans les deux
  validations, sauf l'admin, qui peut valider pour un absent (§ 12, Q6).

## 10. Les questions ouvertes — tranchées le 2026-11-11

**Q1 — Quand « Chacun son équipe » devient-il choisissable ?** — tranchée, cf. § 12
(a) dès la 652 ; (b) à la 659, quand la validation existe ; les cartes
intermédiaires se testent par un POST `entry_mode=TeamEntry`.
*Recommandation* : (b). *Conséquence* : `test_match_report_entry_mode.py:26-37`
change à la 659 ; aucun match réel n'est démarré dans un mode inachevé.

**Q2 — Deux écritures simultanées dans deux camps.** — tranchée, cf. § 12
(a) garder le 409 + rechargement de E21 ; (b) réessayer une fois côté serveur
les commandes d'un camp.
*Recommandation* : (b), carte 653. *Conséquence* : le coach ne perd plus sa
saisie ; un vrai conflit (deux écritures sur le même camp) reste un 409.

**Q3 — Quand le match commence-t-il en mode B ?** — tranchée, cf. § 12
(a) chaque coach saisit ses actions dès son propre avant-match fini ;
(b) seulement quand les deux fans factors et les deux sélections de coups de
pouce sont enregistrés.
*Recommandation* : (b). *Conséquence* : une action du favori ne peut plus
fermer l'annulation globale (`pre_match.rs:378-381`) pendant que l'outsider
achète encore ; `/live` renvoie à `step2` ou aux coups de pouce tant que ce
n'est pas le cas.

**Q4 — La validation est-elle un geste distinct de l'enregistrement de l'étape 5 ?** — tranchée, cf. § 12
(a) oui, sur le récapitulatif, comme la maquette B4 → B5 ; (b) « Enregistrer »
à l'étape 5 vaut validation.
*Recommandation* : (a), possible dès que son camp a son MVP et son après-match,
sans attendre l'autre. *Conséquence* : `TeamSideValidated` se rejoue en
`PreMatch` comme en `ReadyToPublish`.

**Q5 — Un camp validé est-il figé ?** — tranchée, cf. § 12
(a) oui, toute écriture dans ce camp est refusée (409) jusqu'à la publication ;
(b) une écriture dans son camp retire sa validation ; (c) figé, avec un bouton
« Retirer ma validation » tant que l'autre n'a pas validé.
*Recommandation* : (c). *Conséquence* : un événement de plus
(`TeamSideValidationWithdrawn`) ; une erreur vue après sa validation se corrige
sans passer par une dépublication.

**Q6 — L'admin en mode B.** — tranchée, cf. § 12
(a) il écrit et valide les deux camps, titre compris ; (b) il lit seulement.
*Recommandation* : (a), comme en mode A où l'admin écrit toujours.
*Conséquence* : un coach absent ne bloque pas le match ; `can_write_side` rend
vrai pour l'admin quel que soit le camp.

**Q7 — Quels faits nouveaux font un toast ?** — tranchée, cf. § 12
(a) la validation d'un camp seule ; (b) aussi le fan factor et l'après-match
de l'autre.
*Recommandation* : (a) ; les deux autres ne font que le signal. *Conséquence* :
`TOAST_EVENT_TYPES` passe de quatre à cinq.

**Q8 — Comment le passage en `ReadyToPublish` se dit-il ?** — tranchée, cf. § 12
(a) un événement `PostMatchClosed` appendé avec le second après-match ;
(b) la projection déduit la phase de deux colonnes par camp.
*Recommandation* : (a) — la projection ne reçoit que l'événement
(`match_report_repository.rs:19-25`), pas l'état. *Conséquence* : un événement
de plus, à classer dans le bras groupé du publisher.

**Q9 — Un coach des deux équipes peut-il choisir « Chacun son équipe » ?** — tranchée, cf. § 12
(a) oui, il écrit les deux camps ; (b) l'option lui est masquée.
*Recommandation* : (b). *Conséquence* : `match_mode_page` consulte
`camp_du_coach` ; le mode B garde toujours deux interlocuteurs.

## 11. Ce que le code contredit dans l'épic

- **« Favori a fini — un événement ajouté »** : `InducementsRecorded`, même vide
  par « Passer », dit déjà que le favori a fini, et l'outsider est déjà refusé
  avant lui en 409 (`pre_match.rs:269-281`, `inducements_controller.rs:442-449`,
  carte 627). Aucun événement n'est nécessaire.
- **« Les valeurs d'équipe se figent au premier fan factor enregistré »** n'est
  pas un acquis : aujourd'hui chaque enregistrement les relit
  (`record_fan_factor_use_case.rs:119-121`), et `into_pre_match` les efface
  (`match_report_ready_to_publish.rs:191-192`). C'est une règle à écrire pour le
  mode B (R4).
- **« Rejoindre — données modifiées : aucune »** : vrai du schéma, mais la carte
  introduit la valeur `TeamEntry` dans l'event store et la projection, et trois
  tests l'utilisent comme mode inconnu.
- **« Démarrage concurrent »** est déjà tenu par E21 (§ 1) ; seule manque
  l'information du second coach que le mode n'est pas le sien.
- **« `MatchReportPublished` vers `teams`, `ranking`, `players` et
  `competitions` »** : `players` ne l'écoute pas, il reçoit les
  `PlayerMatchImpactAppEvent` du publisher. Sans effet sur l'épic.
- **« Le dernier qui enregistre écrase l'autre sans le savoir »** : vrai en
  séquence ; en simultané, l'event store refuse le second en 409 et recharge sa
  page — d'où la 653.
- **« Le résumé ne sort pas du BC (vérifié) »** : confirmé, ni payload ni
  projection.

## 12. Décisions du 2026-11-11

Les neuf recommandations du § 10 sont retenues telles quelles.

1. **Le choix du mode B s'ouvre à la 659** (Q1). D'ici là, « Chacun son
   équipe » reste grisée et marquée « bientôt » ; les cartes 652 à 658 se
   testent par un POST `entry_mode=TeamEntry`. `test_match_report_entry_mode.py:26-37`
   change à la 659.
2. **Une commande d'un camp est réessayée une fois sur conflit de version**
   (Q2), carte 653. Un second conflit reste un 409 avec rechargement.
3. **Le match commence quand les deux fans factors et les deux sélections de
   coups de pouce sont enregistrés** (Q3). Avant, `/live` et les étapes
   d'action renvoient le coach à son avant-match ou à ses coups de pouce ; une
   action ne peut plus fermer l'annulation globale pendant que l'outsider achète.
4. **La validation est un geste distinct**, sur le récapitulatif (Q4), possible
   dès que son camp a son MVP et son après-match, sans attendre l'autre :
   `TeamSideValidated` se rejoue en `PreMatch` comme en `ReadyToPublish`.
5. **Un camp validé est figé** ; « Retirer ma validation » reste possible tant
   que l'autre n'a pas validé (Q5). Un événement de plus,
   `TeamSideValidationWithdrawn`.
6. **L'admin écrit et valide les deux camps en mode B, titre compris** (Q6) :
   `can_write_side` rend vrai pour lui quel que soit le camp ; un coach absent
   ne bloque pas le match.
7. **Seule la validation d'un camp fait un toast** (Q7) ; le fan factor et
   l'après-match de l'autre ne font que le signal. `TOAST_EVENT_TYPES` passe à
   cinq.
8. **`PostMatchClosed` dit le passage en `ReadyToPublish`** (Q8), appendé avec
   le second `TeamPostMatchRecorded` dans le même `append_many` ; la projection
   y passe la phase.
9. **Un coach des deux équipes ne se voit pas proposer le mode B** (Q9) :
   `match_mode_page` et la sélection hors calendrier consultent `camp_du_coach`,
   et le POST le refuse en 400.
   **Élargi le 2026-11-11, à la réalisation de la 652** : le mode B est refusé
   **dès que les deux équipes ont le même coach, quel que soit le démarreur,
   admin compris** — sans quoi un admin démarrerait en B un match qui n'a qu'un
   interlocuteur. `may_choose_entry_mode` compare les coachs des deux équipes
   (`ITeamDataPort::find_team_coach`, ajouté), sans regarder qui démarre ; une
   équipe introuvable ou un port en erreur refusent, 400 et `tracing::warn!`.

## 13. Correction de l'épic

Reportées dans `kanban/epics/ready_to_be_done/E22-mode-chacun-son-equipe.md` le
2026-11-11 :

- **Le tableau des cartes** reçoit ses numéros, 652 à 661 ; « Gains et fan
  factor » et « Deux paragraphes » fusionnent en 658 ; **653 « Deux coachs
  écrivent en même temps »** est ajoutée.
- **« Le favori a fini ses achats » n'ajoute pas d'événement** :
  `InducementsRecorded`, même vide, le dit déjà, et l'outsider est déjà refusé
  avant le favori (carte 627).
- **Le gel des valeurs d'équipe** est une règle nouvelle du mode B, non un
  acquis : en mode A, chaque fan factor les relit.
- **« Rejoindre » touche aux données** : la valeur `TeamEntry` entre dans
  l'event store et la projection, sans migration.
- **Le démarrage concurrent est déjà tenu par E21** ; la carte 652 n'y ajoute
  rien.
- **`players` n'écoute pas `MatchReportPublished`** : il reçoit les
  `PlayerMatchImpactAppEvent` du publisher.
- **L'écrasement silencieux n'a lieu qu'en séquence** ; en simultané, l'event
  store refuse la seconde écriture, d'où la 653.
- **Le tableau des conséquences en production** suit le § 6.
- **L'état** passe à `ready_to_be_done`, et le fichier dans
  `kanban/epics/ready_to_be_done/`.
