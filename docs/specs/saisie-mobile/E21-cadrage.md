# E21 — Le mode de saisie et l'après-match en séquence — cadrage

Fiche de cadrage de l'épic `kanban/epics/to_be_refined/E21-mode-de-saisie-et-apres-match-en-sequence.md`,
**brouillon du 2026-11-01, décisions reportées au § 8**. Comme pour E19, une épic qui fait
évoluer l'existant part de ce que le code fait, des règles qu'elle pose, des
événements qu'elle touche, des routes et des tests. Les décisions de l'épic
(2026-11-01) ne sont pas rediscutées ici ; celles prises sur ce cadrage sont
au § 8 ; tous sont tranchés.

## 1. Ce que le code fait aujourd'hui

Les références sont prises sur `b994a75e`, chemins relatifs à `src/app/match_report/`.

**La confirmation et son auteur**

- `SelectionConfirmed { confirmed_by }` (`domain/events.rs:35-37`) est le seul
  passage de `Draft` à `PreMatch` (`domain/match_report_draft.rs:87-94`).
- **Le rejeu oublie `confirmed_by`** : `rehydrate` reconstruit l'avant-match par
  `MatchReportPreMatch::from_draft(draft)` sans lire l'événement
  (`domain/match_report_state.rs:87-90`). Aucun état — `PreMatch`,
  `ReadyToPublish`, `Published` — ne retient qui a confirmé ; seul
  `created_by` traverse les états, et il vaut l'identifiant de compétition pour
  un brouillon né du calendrier (constat de E19, carte 624).
- **Trois chemins confirment**, tous avec l'utilisateur courant :
  - `create_match_report_use_case::execute`, hors calendrier, qui confirme aussitôt
    (`use_cases/create_match_report_use_case.rs:82-84`, `confirmed_by = created_by`) ;
  - `confirmer_si_brouillon`, quand le POST `/new` désigne une rencontre déjà au
    calendrier (`create_match_report_use_case.rs:103-119`, appelé par
    `io/web/match_selection_controller.rs:343`) ;
  - `update_match_selection_use_case::execute`, le POST `/{id}`
    (`use_cases/update_match_selection_use_case.rs:97-105`).
- Les trois émettent le domain event sur le bus du BC ; le publisher en fait
  `MatchReportConfirmed` en relisant l'agrégat (`io/app_events/app_event_publisher.rs:113-115`,
  `358-386`, carte 352).
- `CreateMatchReportError` n'a pas de variante `Conflict` : un conflit sur les
  deux premiers chemins reste un 500 (la carte 623 n'a couvert que la sélection).
- `append_many` écrit plusieurs événements et leur projection dans une seule
  transaction (`io/repository/match_report_repository.rs:432-468`).

**L'arrivée depuis le calendrier**

- `from_pairing` (`io/web/match_selection_controller.rs:110-132`), hors garde,
  redirige vers `/{id}`. `edit_match_report` (`:220-290`) aiguille : `Draft` →
  écran de sélection, `PreMatch` → `step2`, `ReadyToPublish` → `step5`,
  `Published` → **409**, `Cancelled` → 410.
- Le calendrier, les résultats et le détail de journée admin y mènent
  (`competitions/io/web/calendrier_tab_controller.rs:239`, `resultats_view.rs:437`,
  `schedule-round-detail.html:167`).

**Les droits**

- `garde_saisie` (`io/web/garde_saisie.rs:52-115`), posée une fois sur
  `routes_de_saisie()` (`router.rs:56-58`, `83-133`) : admin ou coach d'une des
  deux équipes (`is_authorized`, `use_cases/match_report_access_service.rs:41-51`),
  lecture et écriture confondues. La portée est lue sur la projection par
  `find_access_scope` (compétition, domicile, visiteur).
- `camp_du_coach` (`match_report_access_service.rs:105-120`, carte 621) répond
  domicile, visiteur, les deux, aucun.
- Publication et dépublication se gardent elles-mêmes par `is_authorized`
  (`io/web/recap_controller.rs:463-474`).

**Les actions et le journal**

- `ActionRecorded` porte `recorded_by: CoachId` **depuis sa création**
  (`d1b0d820`, 2026-06-28), sans `serde(default)` (`domain/events.rs:126-136`) :
  un événement qui en serait privé rendrait son rapport illisible au rejeu.
  Aucune migration de données n'a écrit d'`ActionRecorded` ni de ligne de
  `match_report_actions` (`infrastructure/data_migrations/m003` à `m005`
  n'insèrent que `MatchReportCreated`, `SelectionUpdated`, `MatchReportCancelled`).
- **`match_report_actions` ne stocke pas l'auteur** : ni la migration
  (`migrations/20260628000001_match_report_actions.sql`, `20260629000001_…position.sql`),
  ni l'insertion (`io/repository/match_report_repository.rs:181-224`).
  `MatchAction` non plus (`domain/value_objects.rs:441-449`).
- Le journal lit **un seul camp** (`find_actions_by_match_and_side`,
  `match_report_repository.rs:537-575`, tri par tour croissant) ; chaque ligne
  porte un ✕ sans condition (`io/web/widgets/action_log_widget.rs:50-87`,
  `templates/action-log-widget.html`).
- `step3`/`step4` (`io/web/actions_step_controller.rs:116-133`) : retour vers
  `step2`/`step3`, suite vers `step4`/`step5`. L'avant-match et les coups de
  pouce redirigent vers `step3` (`pre_match_controller.rs:228`,
  `inducements_controller.rs:270`) ; l'étape 5 revient vers `step4`
  (`step5_controller.rs:250`).

**Le MVP**

- `MatchActionType::Mvp` (`domain/value_objects.rs:325`), saisi par le bouton ⭐
  du panneau d'action (`templates/action-panel-widget.html:147-150`) et accepté
  par le POST d'action (`io/web/record_action_controller.rs:226`), à n'importe
  quel tour.
- **Aucune règle** : `record_action` (`domain/match_report_pre_match.rs:282-329`)
  accepte plusieurs MVP par camp — un test le fixe
  (`record_two_mvp_same_team`, `:1132`).
- **Le rejeu ne passe pas par `record_action`** : il construit le `MatchAction`
  directement (`domain/match_report_state.rs:189-215` et `359-385`). Une règle
  posée sur la saisie ne peut donc pas toucher un rapport ancien.
- `TurnNumber` va de 1 à 16 (`domain/value_objects.rs:237-244`) : le tour 16 est
  déjà valide.

**L'aléa**

- Le précédent du dépôt est le tirage du calendrier (cartes 612 à 618) : le
  générateur est **reçu** — `rng: &mut impl Rng`, graine fixe en test,
  `StdRng::from_os_rng()` dans le contrôleur (`competitions/domain/draw.rs:319`,
  `competitions/use_cases/admin/draw/preview_draw_use_case.rs:88`,
  `competitions/io/web/admin/schedule_draw_actions.rs:240`). `rand = "0.9"`
  est déjà une dépendance. Aucun trait d'aléa n'existe.

**L'indicateur d'étapes**

- `.mr-steps` est recopié dans six gabarits — `match-selection.html:27`,
  `pre-match.html:68`, `inducements.html:41`, `actions-step.html:55`,
  `step5.html:25` — sur l'ancien découpage : 1 Sélection · 2 Avant-match ·
  3 Domicile · 4 Visiteur · 5 Après-match. Le récapitulatif n'en a pas.

**Ce qu'on peut nommer sans port nouveau** : `ICoachDataPort::find_coach_name`
(`ports.rs:211-212`) pour « saisi par », `ITeamDataPort` pour le camp.

## 2. Les règles

| # | Règle |
|---|---|
| R1 | Le **mode de saisie** est choisi par celui qui démarre, **une fois**, et ne change plus. E21 n'en livre qu'un : « Je saisis tout ». |
| R2 | Le **démarreur** est celui qui confirme le match. Pour tout compte rendu confirmé avant E21, c'est le `confirmed_by` de son `SelectionConfirmed`, et le mode est « Je saisis tout ». |
| R3 | Démarrer est un **geste explicite** : ouvrir le lien d'un match du calendrier ne confirme rien. |
| R4 | Un match du calendrier ne change plus d'équipes ni de journée par la sélection ; le déplacement admin reste. |
| R5 | En « Je saisis tout », **seuls le démarreur et les admins écrivent** — saisie, publication, dépublication, correction. L'autre coach suit, sans aucun bouton d'écriture. |
| R6 | Le journal montre **les deux camps** ; chaque action dit qui l'a saisie ; ✕ n'apparaît que pour qui peut écrire. |
| R7 | Le MVP se saisit dans **sa propre étape**, après le match, domicile puis visiteur, au **tour 16**, **un par camp**. La règle vaut à la saisie, jamais au rejeu. |
| R8 | Le **tirage** se fait côté serveur parmi **six** joueurs désignés, distincts ; les désignés sont conservés ; le résultat est définitif. |
| R9 | Le parcours compte **cinq étapes** : Avant-match · Match · MVP · Gains et fans · Validation. |

**Ce qui ne change pas** : le domaine des actions — un MVP reste une action
`Mvp` ; ce qui sort du BC à la publication est identique ; le récapitulatif
publié ; les feuilles imprimables ; la garde de E19 pour les tiers.

## 3. Les événements

| Événement | Statut | Contenu | Rejeu des anciens flux |
|---|---|---|---|
| `EntryModeChosen` | **ajouté** (631) | `mode: EntryMode`, `started_by: CoachId` | absent : mode `SoloEntry`, démarreur = `confirmed_by` |
| `SelectionConfirmed` | inchangé | — | le rejeu lit désormais `confirmed_by` |
| `ActionRecorded` | inchangé | le MVP est une action `Mvp` au tour 16 | — |
| `MvpDrawn` | **ajouté** (638) | `team_side`, `action_id` de l'action tirée, `designated` : six joueurs avec leur nom au moment du tirage | absent : MVP saisi à la main |

**`EntryModeChosen` est appendé avec `SelectionConfirmed`, dans le même
`append_many`** : un démarrage ne peut pas exister sans son mode. Deux
« Commencer » simultanés : le second échoue sur la version (`ConcurrentWrite`),
409 et rechargement (carte 623) — le premier choix l'emporte.

**Le rejeu, en deux temps**, qui est aussi ce que fera la projection :
`(Draft, SelectionConfirmed)` pose `started_by = confirmed_by` et
`entry_mode = SoloEntry` ; `(PreMatch, EntryModeChosen)` les remplace. Les deux
champs traversent `PreMatch`, `ReadyToPublish` et `Published` comme `created_by`
le fait aujourd'hui (`from_draft`, `from_pre_match`, `into_pre_match`,
`from_ready_to_publish`, `unpublish`).

**`EntryMode` n'a qu'une variante en E21**, `SoloEntry`. Une variante
« Chacun son équipe » sans son comportement serait un mode qu'on peut choisir
et qui ne fait rien ; E22 l'ajoute avec ce qui la fait exister.

**`MvpDrawn` est appendé avec l'`ActionRecorded` qu'il désigne**, dans le même
`append_many`. Rien ne sort du BC : le publisher ignore les deux, comme il
ignore aujourd'hui `ActionRecorded`.

**Le publisher a un joker** (`io/app_events/app_event_publisher.rs:150`,
`_ => {}`) : les deux nouveaux événements y tomberaient sans que le compilateur
le dise. La règle de `CLAUDE.md` vise `to_app_event` ; l'aiguillage du BC en a
la même forme : 631 nomme les variantes ignorées une à une.

## 4. Les routes

« Pas de route propre au mobile » : les routes existantes changent de
comportement selon le mode, et le parcours est le même sur ordinateur. Les
**nouvelles** routes ci-dessous sont des étapes nouvelles du parcours, pour les
deux appareils.

| Route | Aujourd'hui | Après E21, « Je saisis tout » |
|---|---|---|
| `GET /new` | sélection | sélection **+ carte « Qui saisit ? »** (632) |
| `POST /new` | crée et confirme, ou `confirmer_si_brouillon` | confirme **avec le mode** (632) |
| `GET /pairing/{id}` | redirige vers `/{id}` | inchangé ; c'est `/{id}` qui change (633) |
| `GET /{id}` sur un brouillon | sélection | **« Qui saisit ? » seul**, sans sélection d'équipes (633) |
| `POST /{id}` | change la sélection et confirme | confirme les équipes du brouillon **avec le mode** ; les équipes du formulaire sont ignorées (633) |
| `GET/POST /step2`, `/inducements/{team}` | garde E19 | démarreur et admins ; la fin des coups de pouce mène au **match en direct** (636) |
| `GET /live` | — | **nouvelle** : le match en direct. Démarreur et admins : « Nouvelle action », ✕, « Fin du match → MVP ». L'autre coach : même écran, **sans aucun bouton d'écriture** (636) |
| `GET /action-log` | `step3/action-log`, `step4/action-log`, un camp chacun | **une** route, les deux camps, « saisi par », ✕ selon le droit (635) |
| `GET /step3`, `/step4` | navigation domicile → visiteur → étape 5 | la page de saisie d'un camp ; **seul retour : « ← Retour au journal »** ; plus de ⭐ (636, 637) |
| `POST /step3/actions`, `/step4/actions` | toute action, MVP compris | démarreur et admins ; **refuse `MVP`** (637) |
| `DELETE /actions/{id}` | garde E19 | démarreur et admins ; refuse un MVP tiré, même en correction (638) |
| `GET/POST /mvp/{side}` | — | **nouvelle** : l'étape MVP d'un camp, saisie manuelle (637) |
| `POST /mvp/{side}/draw` | — | **nouvelle** : le tirage (638) |
| `GET/POST /step5` | retour vers `step4` | démarreur et admins ; retour vers `/mvp/away` (637) |
| `/recap` non publié | `is_authorized` | devient l'étape **Validation** (indicateur à 5) ; le bouton de publication pour le démarreur et les admins seuls (634, 637) |
| `/recap/publish`, `/recap/unpublish` | `is_authorized` | **démarreur et admins** (634) |
| Feuilles imprimables | session | inchangé |

**Où se pose la lecture seule** (634). Le prédicat, à côté de
`is_authorized` dans `match_report_access_service.rs` :

```rust
/// Peut écrire dans ce compte rendu : admin, ou démarreur en « Je saisis tout ».
/// Un brouillon n'a pas de démarreur : tout autorisé peut le démarrer.
pub async fn can_write_report(deps, user, space_id, scope: &PorteeRapport, entry: &EntryScope) -> bool;
```

Il se lit sur la projection — `entry_mode` et `started_by` —, comme la portée
de E19 et pour la même raison : la garde le demande à chaque requête.

Le routeur passe de deux groupes à trois, et la frontière reste une décision
lisible dans `router.rs` :

| Groupe | Garde | Contenu |
|---|---|---|
| `routes_ouvertes()` | inchangé | `/new`, `/pairing/{id}`, feuilles, récapitulatif |
| `routes_de_suivi()` — **nouveau** | `is_authorized` (garde de E19) | `/live`, `/action-log` |
| `routes_de_saisie()` | `is_authorized` **et** `can_write_report` | tout le reste |

Refus dans `routes_de_saisie()` pour l'autre coach : une écriture, ou une
requête HTMX, reçoit **403** ; une page complète est **redirigée vers `/live`**
(303), où il suit le match. Le tiers garde son 403 de E19.

## 5. Le journal des deux camps

- Colonne `match_report_actions.recorded_by TEXT`, remplie à l'insertion depuis
  `ActionRecorded.recorded_by`, et **par une migration SQL pour l'existant** :

  ```sql
  UPDATE match_report_actions a
     SET recorded_by = e.payload->>'recorded_by'
    FROM match_report_event_store e
   WHERE e.event_type = 'ActionRecorded'
     AND e.payload->>'action_id' = a.action_id;
  ```

  `ActionId` et `CoachId` se sérialisent en chaîne (`domain/value_objects.rs:229`,
  `shared_kernel/identity/sulid.rs:57-63`) : `->>` rend la valeur telle quelle.
  Une migration SQL suffit — pas besoin du corpus, donc pas de `DataMigration`.
- La colonne reste **nullable** : une action sans événement correspondant
  resterait sans auteur plutôt que d'en recevoir un inventé. La carte mesure
  sur un dump qu'il n'y en a aucune.
- `find_actions_by_match` remplace `find_actions_by_match_and_side` et rend le
  camp ; le tri devient « du plus récent au plus ancien », comme la maquette.
- « saisi par » : `ICoachDataPort::find_coach_name`, « toi » pour l'utilisateur
  courant.

## 6. Conséquences sur les données de production

| Carte | Données modifiées | Effet sur l'existant |
|---|---|---|
| 631 | un événement **ajouté** ; deux colonnes de `match_report_proj` : `entry_mode TEXT NOT NULL DEFAULT 'SoloEntry'`, `started_by TEXT` ; `started_by` rempli depuis `SelectionConfirmed.confirmed_by` | tout compte rendu confirmé est en « Je saisis tout », son démarreur est celui qui a confirmé. Un brouillon jamais confirmé garde `started_by` vide : il n'est pas démarré. L'event store n'est pas touché. Mesure sur dump : aucun rapport en `PreMatch`, `ReadyToPublish` ou `Published` sans `SelectionConfirmed` |
| 632 | aucune | — |
| 633 | aucune | chaque brouillon du calendrier devient démarrable sans sélection, mais **seulement par le bouton** : l'ouverture du lien ne confirme rien. **Perte acceptée** (2026-11-01) : plus de changement d'équipes ni de journée par la sélection d'un match du calendrier |
| 634 | aucune | **acceptée** (2026-11-01) : dans les comptes rendus en cours, l'autre coach que le démarreur est en lecture seule, même s'il saisissait ; un compte rendu publié ne se corrige plus que par son démarreur ou un admin. Un démarreur qui ne coache aucune des deux équipes ne compte pas : les deux coachs y écrivent (§ 8.8) |
| 635 | **une colonne `recorded_by` et son remplissage** dans `match_report_actions` ; l'event store n'est pas touché | toutes les actions, anciennes comprises, disent qui les a saisies. Mesure sur dump : zéro `ActionRecorded` sans `recorded_by`, zéro ligne de `match_report_actions` sans événement |
| 636 | aucune | un compte rendu en cours à la livraison reprend au match en direct au lieu de `step3` ; ses actions déjà saisies y sont |
| 637 | aucune | les anciens MVP — à tout tour, zéro ou plusieurs par camp — restent valides et s'affichent : la règle ne vaut qu'à la saisie (le rejeu ne passe pas par elle, § 1). En correction, l'étape montre le ou les MVP du camp ; pour en saisir un autre, on supprime d'abord. SPP et événement publié inchangés |
| 638 | un événement **ajouté** ; une colonne nullable `match_report_actions.draw_size` | ce qui sort du BC reste une action `Mvp` ; « tiré parmi » pour les seuls nouveaux |
| 639 | aucune | — |

## 7. Les tests

- **Unitaires** — rejeu : un flux sans `EntryModeChosen` donne `SoloEntry` et
  `confirmed_by` ; un flux avec le donne tel quel, à travers les quatre états.
  `can_write_report` : démarreur, admin, autre coach, tiers, brouillon.
  `record_mvp` : tour 16, refus d'un second MVP, ancien rapport à deux MVP qui
  se rejoue. `draw_mvp` sur une graine fixe : six désignés exigés, distincts,
  le tiré parmi eux, un MVP tiré qui ne se supprime pas.
- **E2E — ce qui change se migre dans la carte qui le change** (règle de
  l'épic) :

  | Test | Ce qui casse | Carte |
  |---|---|---|
  | `test_competition_matchs.py:262` (C7) | attend l'étape 1 active sur la sélection | 633 |
  | `test_match_report_access.py` — `test_le_coach_d_une_equipe_du_match_y_entre` | le membre simple, coach du visiteur, n'a pas démarré : il est redirigé vers `/live` | 634 |
  | `test_match_report_step3_4_actions.py` | navigation, journal à deux camps ; S7 et S8 saisissent `MVP` | 635, 636, 637 |
  | `test_match_report_recap.py:166-167`, `test_match_report_recap_spp.py:95`, `test_match_report_correction.py:233`, `test_manual_phase_override.py:207` | MVP saisi comme action, hors tour 16 | 637 |
  | `test_player_spp_spending.py:180,186` | **deux** MVP dans le même camp | 637 |
  | `test_player_detail.py:167` | MVP saisi comme action | 637 |

  L'épic en annonçait quatre ; un `grep` en trouve **sept fichiers** (§ 9).
- **Le helper** `match_report_helpers.py` ne change pas avant 637 si le mode
  absent vaut « Je saisis tout » (§ 8.2) ; 637 lui ajoute `record_mvp_api`. Treize
  fichiers postent eux-mêmes vers `/new`, neuf portent une copie privée de
  `_record_action_api` ou `_create_draft`.
- **639** écrit le parcours du « Terminé quand » : depuis le calendrier,
  « Je saisis tout », journal commun, MVP domicile tiré puis visiteur à la
  main, gains, validation, publication ; l'autre coach dans un second contexte,
  en lecture seule ; un rapport commencé avant se termine.
- `tests/impact-map.toml` et `tests/e2e/visual/urls.py` suivent les routes
  ajoutées.

## 8. Décisions du 2026-11-01

1. **La carte « Qui saisit ? »** (632) montre les deux options ; « Chacun son
   équipe » est grisée et marquée « bientôt », pour que la question ne change
   pas de forme à l'arrivée de E22.
2. **Un POST sans mode** (632, 633) vaut « Je saisis tout », le défaut déclaré
   par l'épic : les treize fichiers e2e qui postent vers `/new` ne bougent pas.
3. **Le POST d'action refuse `MVP`** (637) : le MVP n'entre que par son étape,
   et la règle « un par camp au tour 16 » n'a pas de porte de service.
4. **Un MVP tiré au sort est définitif, même en correction** (638) : ni ✕ au
   journal, ni suppression par la route.
5. **« tiré parmi 6 désignés » au journal** (638) : la colonne nullable
   `match_report_actions.draw_size` est ajoutée, vide pour tout l'existant.
6. **Le tirage reçoit `&mut impl Rng`** (638), comme le tirage du calendrier :
   le trait `Rng` tient lieu de port, une graine fixe le double en test.
7. **La lecture seule avant le match en direct** (634, 636) : en 634, l'autre
   coach reçoit un refus lisible, « Ce match est saisi par … » ; la 636 le
   remplace par la redirection vers `/live`.
8. **Le démarreur admin qui ne coache aucune des deux équipes ne compte pas
   comme démarreur** (634) : les deux coachs du match écrivent. Personne n'est
   bloqué, aucune mesure sur dump n'est nécessaire.
9. **L'étape 5 « Validation » est le récapitulatif non publié** (637),
   indicateur à 5, sans nouvelle route.
10. **Hors épic** : « Signaler une erreur » sur l'écran de suivi, et les SPP et
    compteurs par joueur sur l'étape MVP.

## 9. Correction de l'épic

Le tableau de l'épic attribue à la carte e2e « les quatre tests qui saisissent
le MVP comme une action ». Le code en compte davantage, et la règle de l'épic
— ce qui change se migre dans la carte qui le change — les répartit :

- **sept fichiers** saisissent `MVP` comme une action, tous migrés par **637** :
  `test_match_report_recap.py`, `test_match_report_recap_spp.py`,
  `test_match_report_correction.py`, `test_manual_phase_override.py`,
  `test_player_spp_spending.py` (deux MVP dans le même camp),
  `test_player_detail.py`, `test_match_report_step3_4_actions.py` (S7, S8) ;
- `test_competition_matchs.py` C7 casse avec le démarrage sans sélection :
  migré par **633** ;
- `test_match_report_access.py::test_le_coach_d_une_equipe_du_match_y_entre`
  casse avec la lecture seule (le membre simple n'a pas démarré) : migré par
  **634**.

La 639 n'en porte aucun : elle ajoute le parcours complet.
