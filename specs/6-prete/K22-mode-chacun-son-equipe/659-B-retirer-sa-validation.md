---
id: 659-B-retirer-sa-validation
parent: K22
status: approved
---

# Retirer sa validation

**Épic :** E22 — Le mode « Chacun son équipe » · **Ordre :** 9 · **Dépend de :** 659-A  
**Dépend de** : 659-A-valider-son-camp
**Conception :** `docs/specs/saisie-mobile/E22-cadrage.md` (§ 3, § 12.5)
**Maquette :** `app-mobile-duo-validate.html` (B5)

## Le constat

- Depuis la 659-A, un camp validé est figé, et rien ne le dégèle avant la
  publication : un coach qui a validé trop tôt ne peut plus corriger son camp.
- La validation de l'autre publie aussitôt : « tant que l'autre n'a pas
  validé » revient à « tant que le rapport n'est pas publié ».

## Ce qui change

- **`TeamSideValidationWithdrawn { team_side, withdrawn_by: CoachId }`** :
  retire la validation d'un camp validé ; le camp redevient modifiable. Se
  rejoue en `PreMatch` comme en `ReadyToPublish`, comme `TeamSideValidated`.
  Retirer un camp non validé est refusé.
- **Qui** : le coach du camp, ou l'admin (§ 12.6) ; le camp se lit comme à la
  659-A — `camp_du_coach`, jamais le formulaire, sauf pour l'admin.
- **Une route** `POST /recap/withdraw-validation`, gardée comme
  `POST /recap/publish`. Mode A : 404.
- **Le récapitulatif** : « Retirer ma validation » sous un camp validé, pour
  qui peut le retirer.
- **App event `MatchSideValidationWithdrawn`** : signal, sans toast (§ 12.7).

## Fichiers touchés

`domain/events.rs`, les états `PreMatch` et `ReadyToPublish`,
`domain/match_report_state.rs`, `domain/error.rs`, un use case de retrait,
`io/web/recap_controller.rs`, le routeur, `io/web/templates/recap.html`,
`io/app_events/app_event_publisher.rs`,
`io/repository/match_report_repository.rs`,
`src/app/shared_kernel/app_events/match_report_app_events.rs`,
`src/web/realtime/toast.rs` (bras sans toast) ; le fichier e2e de la 659-A,
`tests/impact-map.toml`.

## Règles et invariants

- Une validation se retire tant que le rapport n'est pas publié (§ 12.5).
- Seuls le coach du camp et l'admin la retirent.
- Un camp dont la validation est retirée s'écrit de nouveau, et se revalide.

## Tests

- **Unitaires** : valider, retirer, écrire dans le camp, revalider — rejoué ;
  retrait d'un camp non validé refusé ; le coach adverse ne retire pas (403) ;
  `MatchSideValidationWithdrawn` sort du publisher.
- **E2E** : le visiteur valide, retire, modifie son fan factor d'après-match,
  revalide ; le domicile valide et publie.

## Conséquences sur les données de production

Un événement **ajouté**, propre au mode B ; aucune colonne.

## Terminé quand

Un coach qui a validé son camp retire sa validation, le corrige et le revalide,
tant que l'autre n'a pas validé.
