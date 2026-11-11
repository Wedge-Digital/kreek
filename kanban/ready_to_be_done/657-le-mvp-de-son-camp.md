# Le MVP de son camp

**Épic :** E22 — Le mode « Chacun son équipe » · **Ordre :** 6 · **Dépend de :** 652, 656
**Conception :** `docs/specs/saisie-mobile/E22-cadrage.md` (R6, R7, § 4)
**Maquette :** `app-mobile-duo-mvp.html` (B3)

## Le constat

- L'étape MVP visiteur **renvoie à l'étape domicile** tant que le domicile n'a
  pas son MVP (`src/app/match_report/io/web/mvp_step_controller.rs:127-134`) :
  en mode B, le visiteur attendrait l'autre coach.
- La suite du domicile est le MVP visiteur, celle du visiteur l'étape 5
  (`mvp_step_controller.rs:339-345`) ; `/live` mène à `mvp/home`
  (`io/web/live_match_controller.rs:229`, corrigé par la 656).
- `/mvp/{side}` et `/mvp/{side}/draw` sont dans `garde_saisie`, sans contrôle
  de camp.

## Ce qui change

- Les deux routes passent dans `routes_de_camp()` : le `side` du chemin doit
  être celui du coach.
- En mode B, plus de renvoi vers le domicile ; la suite de chaque camp est
  `step5`.
- L'état de l'adversaire en bas de page (« MVP tiré parmi 6 désignés »,
  « en cours ») dans un widget rechargé sur `matchChanged`. La page déclare
  déjà sa présence (`templates/mvp-step.html:7`).
- Le mode A est inchangé.

## Fichiers touchés

`router.rs`, `io/web/mvp_step_controller.rs`, `io/web/mvp_draw_controller.rs`,
`templates/mvp-step.html`, un widget « MVP adverse ».

## Règles et invariants

- Un MVP par camp, au tour 16, saisi ou tiré, définitif s'il est tiré (E21).
- Chacun désigne et tire le MVP de son équipe seule.

## Tests

- **Unitaires** : en mode B, `mvp/away` sans MVP domicile s'affiche ; en mode A,
  il redirige ; la suite est `step5` dans les deux camps en mode B.
- **E2E** : deux contextes ; le visiteur tire son MVP avant que le domicile
  n'ait choisi le sien ; le domicile voit l'état du visiteur changer ; un POST
  `mvp/home` par le visiteur rend 403.

## Conséquences sur les données de production

Aucune. Les MVP sont des actions `Mvp`, déjà stockées par camp.

## Terminé quand

En mode B, chaque coach désigne le MVP de son équipe sans attendre l'autre, et
voit où en est l'autre.
