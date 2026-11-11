# Le second coach rejoint le match

**Épic :** E22 — Le mode « Chacun son équipe » · **Ordre :** 1 · **Dépend de :** —
**Conception :** `docs/specs/saisie-mobile/E22-cadrage.md` (R1 à R3, R11, § 4, § 12.1, § 12.6, § 12.9)
**Maquette :** `assets/rawpages/html/app-mobile-parcours.html`, section 3B

## Le constat

`EntryMode` n'a qu'une variante, `SoloEntry` (`src/app/match_report/domain/value_objects.rs:44-47`).
Le droit d'écrire est **par compte rendu** : `can_write_report` et
`coach_may_write` (`use_cases/match_report_access_service.rs:114-148`) refusent
tout mode inconnu, et les gardes ne lisent dans le chemin que `space_id` et
`match_report_id` (`io/web/garde_saisie.rs:128-139`). Rien ne sait dire « ce
coach écrit dans ce camp ».

Le démarrage concurrent est **déjà tenu** par E21 : `POST /{id}` sur un rapport
démarré rend 409 et recharge (`io/web/match_selection_controller.rs:683-705`),
`POST /new` réussit en silence et ouvre `/{id}`, qui mène à `step2`. Cette
carte n'y touche pas.

## Ce qui change

- **`EntryMode::TeamEntry`**, forme écrite `TeamEntry` ; `read_entry_mode`
  l'accepte (`match_selection_controller.rs:363-366`). Un coach des deux
  équipes qui l'envoie reçoit 400 (§ 12.9).
- **`coach_may_write`** gagne la branche `TeamEntry` : les deux coachs écrivent
  dans le rapport.
- **`can_write_side(deps, user, space_id, scope, entry, side)`**, à côté de
  `can_write_report` : admin, toujours (§ 12.6) ; mode A, la règle de
  `can_write_report` ; mode B, le coach de ce camp (`camp_du_coach`).
- **`garde_camp`**, troisième `route_layer` (`router.rs`) sur un groupe
  `routes_de_camp()`, **vide** à cette carte : le camp vient du paramètre
  `side`, du `team_id` comparé à la portée, ou du groupe lui-même pour
  `step3*`/`step4*`. Refus : 403 sur une écriture ou un fragment, 303 vers
  `step2` pour une page complète. Les cartes 654 à 659 y rangent leurs routes.
- Le second coach qui ouvre le match en mode B arrive à `step2`, qui reste à
  deux camps jusqu'à la 654.
- L'option « Chacun son équipe » **reste grisée** (`entry-mode-card.html:19-26`) :
  elle s'ouvre à la 659 (§ 12.1).

## Fichiers touchés

`domain/value_objects.rs`, `io/web/match_selection_controller.rs`,
`use_cases/match_report_access_service.rs`, `io/web/garde_saisie.rs`,
`router.rs` (chemins relatifs à `src/app/match_report/`) ;
`tests/e2e/test_match_report_entry_mode.py`.

## Règles et invariants

- Le mode est choisi une fois et ne change plus ; le premier démarrage validé
  l'emporte (R11).
- Un contrôle d'accès échoue fermé : un mode inconnu, une erreur de port
  refusent.
- L'admin écrit les deux camps (§ 12.6).
- Un coach des deux équipes ne démarre pas en mode B (§ 12.9).

## Tests

- **Unitaires** : `read_entry_mode("TeamEntry")` ; `can_write_side` — coach de
  son camp, de l'autre, admin, tiers, mode A inchangé ; un coach des deux
  équipes refusé en mode B. Les trois tests qui prenaient `TeamEntry` pour un
  mode inconnu (`match_selection_controller.rs:726`,
  `match_report_access_service.rs:481-483`) passent à une autre valeur.
- **E2E** : `test_match_report_entry_mode.py:40-57` change de valeur inconnue ;
  un rapport démarré par POST en `TeamEntry` laisse entrer les deux coachs à
  `step2`, et refuse un tiers.

## Conséquences sur les données de production

Aucune migration. Une **valeur nouvelle**, `TeamEntry`, peut entrer dans
`EntryModeChosen.mode` et `match_report_proj.entry_mode` (colonne `TEXT` sans
contrainte). Aucun compte rendu existant n'est concerné : aucun n'est en mode B,
et l'option reste fermée dans l'interface.

## Terminé quand

Un rapport démarré par POST en `TeamEntry` mène chacun des deux coachs à son
avant-match ; un tiers reçoit 403 ; un rapport en « Je saisis tout » se
comporte exactement comme avant.
