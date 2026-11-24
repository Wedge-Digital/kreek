---
id: 659-C-le-mode-chacun-son-equipe-s-ouvre
parent: K22
status: approved
---

# Le récapitulatif en direct, et le mode « Chacun son équipe » s'ouvre

**Épic :** E22 — Le mode « Chacun son équipe » · **Ordre :** 10 · **Dépend de :** 659-B  
**Dépend de** : 659-A-valider-son-camp, 659-B-retirer-sa-validation
**Conception :** `docs/specs/saisie-mobile/E22-cadrage.md` (§ 5, § 12.1, § 12.7, § 12.9)
**Maquette :** `app-mobile-duo-validate.html` (B5)

## Le constat

- **`recap.html` ne déclare pas sa présence** : la validation de l'autre n'y
  arrive pas (`src/app/match_report/io/web/templates/recap.html`). Les pages
  qui la déclarent portent un `div.realtime-presence`
  (`io/web/templates/actions-step.html:7`).
- `TOAST_EVENT_TYPES` en compte quatre (`src/web/realtime/toast.rs:39-44`) ;
  `MatchSideValidated` (659-A) n'en fait pas partie.
- L'option « Chacun son équipe » est grisée et marquée « bientôt », sans valeur
  (`io/web/templates/match_report/entry-mode-card.html:19-26`). La carte est
  incluse dans `match-mode.html` (équipes connues) et dans
  `match-selection.html` (hors calendrier, équipes choisies dans le formulaire).
- Le serveur refuse déjà le mode B quand les deux équipes ont le même coach
  (`may_choose_entry_mode`, `use_cases/match_report_access_service.rs:207`,
  appelé par `mode_choisi`, `io/web/match_selection_controller.rs:372-391`) :
  400 avant tout effet.

## Ce qui change

- **Le récapitulatif non publié déclare sa présence** ; le bloc des
  validations devient un widget rechargé sur `matchChanged` ; à la
  publication (`MatchReportPublished`), la page entière se recharge.
- **L'écran d'attente bascule vers le récapitulatif** (décision du
  2026-11-24) : le premier camp valide depuis `team-step5` et y reste ; son
  widget de l'adversaire
  (`io/web/widgets/opponent_post_match_widget.rs`) rend 204 dès que le rapport
  n'est plus en avant-match, et l'écran ne verrait jamais la publication. Il
  rend désormais un `HX-Redirect` vers le récapitulatif quand le rapport a
  quitté l'avant-match : les deux coachs s'y retrouvent, en direct.
- **Le toast** : `notice_of` rend, pour `MatchSideValidated`, « Les London Rats
  sont validés par Morgane » (§ 12.7) ; `TOAST_EVENT_TYPES` passe à cinq. Le
  retrait reste un signal.
- **L'option « Chacun son équipe » s'ouvre** (§ 12.1) : valeur `TeamEntry`,
  « bientôt » retiré.
  - Sur `match-mode.html`, les équipes sont connues : l'option est **masquée**
    quand `may_choose_entry_mode` la refuse (§ 12.9).
  - Sur la sélection hors calendrier, les équipes se choisissent dans le
    formulaire : l'option reste affichée, et le refus du serveur réaffiche le
    formulaire avec un message lisible (« Les deux équipes ont le même
    coach : chacun son équipe n'est pas possible. ») au lieu d'un 400 nu.

## Fichiers touchés

`io/web/templates/recap.html`, un widget des validations et sa route,
`io/web/recap_controller.rs`, `io/web/widgets/opponent_post_match_widget.rs`,
`src/web/realtime/toast.rs`,
`io/web/templates/match_report/entry-mode-card.html`,
`io/web/match_selection_controller.rs`, `io/web/templates/match-mode.html`,
`io/web/templates/match-selection.html` ;
`tests/e2e/test_match_report_entry_mode.py`, le fichier e2e de la 659-A,
`tests/impact-map.toml`, `tests/e2e/visual/urls.py`.

(Chemins sous `src/app/match_report/` sauf mention.)

## Règles et invariants

- Seule la validation d'un camp fait un toast (§ 12.7).
- Le formulaire d'un coach n'est jamais rechargé par le direct : seul le bloc
  des validations l'est, et la page à la publication.
- Un coach des deux équipes ne se voit pas proposer le mode B quand les
  équipes sont connues ; le serveur le refuse dans tous les cas (§ 12.9).

## Tests

- **Unitaires** : `notice_of(MatchSideValidated)` et son texte ; l'option
  masquée pour deux équipes d'un même coach, affichée sinon.
- **E2E** (deux contextes) : le visiteur valide, le domicile voit « Validé ✓ »
  et le bouton « … et publier » **sans rechargement** et reçoit le toast ; l'écran
  d'attente du visiteur bascule vers le récapitulatif ; sa validation publie,
  les deux pages montrent le rapport publié. La carte
  « Qui saisit ? » propose « Chacun son équipe » et démarre le mode B
  (`test_match_report_entry_mode.py:26-37` migré) ; elle ne le propose pas pour
  deux équipes d'un même coach.

## Conséquences sur les données de production

Aucune donnée modifiée. **À partir de cette carte, des comptes rendus réels
peuvent naître en mode B.**

## Terminé quand

Un coach choisit « Chacun son équipe » dans « Qui saisit ? » ; sur le
récapitulatif, la validation de l'autre arrive sans rechargement, avec un
toast, et la seconde validation montre le rapport publié aux deux.
