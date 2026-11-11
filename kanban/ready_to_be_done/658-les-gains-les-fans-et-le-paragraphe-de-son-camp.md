# Les gains, les fans et le paragraphe de son camp

**Épic :** E22 — Le mode « Chacun son équipe » · **Ordre :** 7 · **Dépend de :** 657
**Conception :** `docs/specs/saisie-mobile/E22-cadrage.md` (R7, R8, § 3, § 5, § 6, § 8 « fusion », § 12.7, § 12.8)
**Maquette :** `app-mobile-duo-step5.html` (B4)

Fusion des cartes « Les gains et le fan factor de son camp » et « Deux
paragraphes de résumé » de l'épic : même formulaire, même événement.

## Le constat

- `PostMatchRecorded` porte les **deux** camps — gains, bonus, `FanFactorMod`,
  titre et corps du résumé (`src/app/match_report/domain/events.rs:170-186`) —
  et `record_post_match` exige le MVP **des deux camps**
  (`domain/match_report_pre_match.rs:697-712`) ; `GET step5` renvoie au premier
  camp sans MVP (`io/web/step5_controller.rs:153-163`).
- **L'après-match n'est pas émis sur le bus** : `record_post_match_use_case`
  appende sans émettre (`use_cases/record_post_match_use_case.rs:53-70`).
- **`step5.html` ne déclare pas sa présence**.
- **Le récapitulatif n'affiche le résumé — et le « Publié par » — que si un
  titre existe** (`templates/recap.html:131-145`) : en mode B, le paragraphe du
  visiteur disparaîtrait quand le domicile n'écrit pas de titre.

## Ce qui change

- **`TeamPostMatchRecorded { team_side, gain: MatchGain, no_stalling_bonus:
  NoStallingBonus, fan_mod: FanFactorMod, summary_title: Option<String>,
  summary_paragraph: Option<String>, recorded_by: CoachId }`**. Il remplit
  `home_gain`, `home_no_stalling_bonus`, `home_fan_mod`… **du seul camp** —
  les champs du mode A, pour que le payload publié ne change pas — et un
  paragraphe par camp, champs ajoutés aux trois états. `summary_title` n'est
  accepté que du camp domicile (ou de l'admin, § 12.6).
- **`PostMatchClosed`**, sans champ, appendé **avec le second**
  `TeamPostMatchRecorded` dans le même `append_many` (§ 12.8) : `PreMatch` →
  `ReadyToPublish` au rejeu, `phase = 'ReadyToPublish'` en projection. En
  `ReadyToPublish` (correction), un `TeamPostMatchRecorded` remplace les valeurs
  de son camp.
- L'après-match d'un camp exige **son** MVP (R7).
- **Le use case émet** les deux événements ; app event
  `MatchPostMatchRecorded` (signal, sans toast, § 12.7) ; `PostMatchClosed`
  rejoint le bras groupé du publisher.
- **`step5` en mode B** : ses gains, son bonus, sa modification de fans, son
  paragraphe — et le titre pour le domicile ; le camp de l'autre en lecture
  seule (« ✓ London Rats · 40 kPo, saisis par Morgane ») dans un widget
  rechargé sur `matchChanged`, **le formulaire n'étant jamais rechargé** ; la
  page déclare sa présence. La suite mène au récapitulatif.
- **Le récapitulatif** affiche le résumé dès qu'il y a un titre ou un
  paragraphe : le titre s'il existe, puis le paragraphe de chaque camp, chacun
  avec son équipe ; « Publié par » ne dépend plus du titre.

## Fichiers touchés

`domain/events.rs`, `domain/match_report_pre_match.rs`,
`domain/match_report_ready_to_publish.rs`, `domain/match_report_published.rs`,
`domain/match_report_state.rs`, `use_cases/record_post_match_use_case.rs`,
`io/web/step5_controller.rs`, `templates/step5.html`,
`io/web/recap_controller.rs`, `templates/recap.html`, `io/web/view_models.rs`,
`io/app_events/app_event_publisher.rs`, `io/repository/match_report_repository.rs`,
`src/app/shared_kernel/app_events/match_report_app_events.rs`,
`src/web/realtime/toast.rs`.

## Règles et invariants

- Chacun son après-match ; le titre au domicile (R8) ; un paragraphe par camp.
- Le rapport passe en `ReadyToPublish` quand les deux camps ont enregistré leur
  après-match, ni avant ni après.
- Le payload de `MatchReportPublished` d'un rapport mode B égale celui du mode A
  aux mêmes valeurs.
- Les champs de résumé restent du texte libre (`Option<String>`), exception
  admise par `CLAUDE.md`, comme dans `PostMatchRecorded`.

## Tests

- **Unitaires** : après-match d'un camp sans le MVP de l'autre accepté ; titre
  refusé du visiteur ; le second appende `PostMatchClosed` ; rejeu mode B
  jusqu'à `ReadyToPublish` ; égalité des payloads mode A / mode B ; émission ;
  le récapitulatif rend un paragraphe sans titre.
- **E2E** : deux contextes ; chacun ses gains, ses fans et son paragraphe ; le
  domicile voit les gains du visiteur arriver sans perdre sa saisie ; le
  récapitulatif montre les deux paragraphes ; un rapport mode A sans titre
  montre son corps.

## Conséquences sur les données de production

Deux événements **ajoutés**, propres au mode B ; des champs de paragraphe dans
l'agrégat seulement (aucune colonne). `PostMatchRecorded` reste celui du mode A
et se rejoue tel quel. **Le récapitulatif change pour l'existant** : un rapport
publié avec un corps mais sans titre affiche désormais son corps, qu'il
masquait. Base de dev au 2026-11-11 : 11 `PostMatchRecorded` ont un corps sans
titre ; la mesure sur dump de production est consignée sur la carte.

## Terminé quand

En mode B, chaque coach enregistre les gains, les fans et le paragraphe de son
équipe, voit ceux de l'autre arriver, et le récapitulatif montre le titre du
domicile suivi des deux paragraphes.
