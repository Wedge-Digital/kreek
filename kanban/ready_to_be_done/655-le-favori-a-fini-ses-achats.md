# Le favori a fini ses achats

**Épic :** E22 — Le mode « Chacun son équipe » · **Ordre :** 4 · **Dépend de :** 654
**Conception :** `docs/specs/saisie-mobile/E22-cadrage.md` (R5, § 1 « Les coups de pouce », § 5, § 11)
**Maquettes :** `app-mobile-real-2b-inducements.html` (B1b), `app-mobile-duo-wait.html` (B1b, attente)

## Le constat

- **L'ordre existe déjà** (carte 627) : l'outsider est refusé en 409 tant que
  le favori n'a pas validé (`src/app/match_report/domain/match_report_pre_match.rs:269-281`,
  `io/web/inducements_controller.rs:442-449`). « Passer » enregistre une liste
  vide (`templates/inducements.html:169`) : **`InducementsRecorded` vide dit
  déjà « le favori a fini »**. Aucun événement n'est à créer.
- **L'outsider n'a pas d'écran d'attente** : le GET le redirige vers la page du
  favori (`inducements_controller.rs:187-201`), qu'en mode B il n'a pas le droit
  d'ouvrir.
- **`inducements.html` ne déclare pas sa présence** : le signal de la
  validation du favori n'y arriverait pas.
- `/inducements/{team_id}` et `/step2/{team_id}/mercenaires` sont dans
  `garde_saisie`, sans contrôle de camp.

## Ce qui change

- Les deux routes passent dans `routes_de_camp()` (652) : le `team_id` du chemin
  doit être l'équipe du coach.
- **L'écran d'attente**, mode B : l'outsider qui ouvre sa page avant la
  validation du favori voit « [Coach] achète ses coups de pouce » et son
  avant-match ; un widget se recharge sur `matchChanged` et, quand
  `inducements_turn` le désigne, la page s'ouvre sur ses achats. La page déclare
  sa présence. Le toast `MatchInducementsRecorded` existe déjà.
- Le bouton de validation du favori se lit « J'ai fini mes achats », panier
  vide compris ; « Passer » disparaît en mode B.
- `POST /cancel-inducements` reste ouvert aux deux coachs (règles de E20) ;
  le toast d'annulation existe déjà.
- Le mode A est inchangé : redirection vers le tour, comme aujourd'hui.

## Fichiers touchés

`io/web/inducements_controller.rs`, `io/web/templates/inducements.html`, un
widget « attente des coups de pouce », `router.rs`, la feuille de la page ;
`tests/impact-map.toml`, `tests/e2e/visual/urls.py`.

## Règles et invariants

- Le favori achète d'abord ; l'outsider attend (R5). Le domaine ne change pas.
- Chacun n'achète que pour son équipe.
- Une erreur se corrige par l'annulation globale, ouverte à l'un ou l'autre
  coach, tant qu'aucune action n'existe (E20) — que la décision § 12.3
  garantit en mode B jusqu'à la fin des deux sélections.

## Tests

- **Unitaires** : la page de l'outsider rend l'attente tant que le favori n'a
  pas validé, sa page d'achat ensuite ; un `team_id` qui n'est pas celui du
  coach est refusé par `garde_camp`.
- **E2E** : deux contextes ; l'outsider voit l'attente ; le favori valide
  « J'ai fini mes achats » sans rien acheter ; la page de l'outsider s'ouvre
  sur ses achats sans rechargement, et il reçoit le toast ; un POST direct de
  l'outsider avant cela rend 409, un POST sur l'équipe adverse 403.

## Conséquences sur les données de production

Aucune. Aucun événement ajouté : `InducementsRecorded` porte déjà la validation
du favori, et le mode A enchaîne comme aujourd'hui.

## Terminé quand

En mode B, l'outsider voit qu'il attend le favori, et sa page d'achat s'ouvre
d'elle-même dès que le favori a déclaré « J'ai fini mes achats ».
