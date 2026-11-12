# Le favori a fini ses achats

**Épic :** E22 — Le mode « Chacun son équipe » · **Ordre :** 4 · **Dépend de :** 654
**Conception :** `docs/specs/saisie-mobile/E22-cadrage.md` (R5, § 1 « Les coups de pouce », § 5, § 11)
**Maquettes :** `app-mobile-real-2b-inducements.html` (B1b), `app-mobile-duo-wait.html` (B1b, attente)

## Le constat

- **L'ordre existe déjà** (carte 627) : l'outsider est refusé en 409 tant que
  le favori n'a pas validé (`src/app/match_report/domain/match_report_pre_match.rs:393-407` — lu à 269-281 avant la 654,
  `io/web/inducements_controller.rs:440-449`, `refus_d_achat`). « Passer » enregistre une liste
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

## Réalisation — précisions et écarts (2026-11-12)

Dix décisions prises avec l'utilisateur avant le code :

1. **Le favori qui a fini attend lui aussi**, sur sa propre page : en mode B, le
   POST du favori rend `RecordInducementsOutcome::AwaitOpponent`, qui le ramène
   à `/inducements/{son équipe}` ; la page y montre l'attente de l'outsider,
   puis le récapitulatif. Sans cela, la redirection vers la page de l'outsider
   lui était refusée par `garde_camp`.
2. **L'admin** voit ce que voit le coach de l'équipe de l'URL, plus un lien
   « Saisir pour <équipe attendue> » (§ 12.6, comme en 654).
3. **Le `team_id` reste dans le chemin** ; `garde_camp` le lit déjà.
4. **L'outsider qui arrive après la validation** tombe directement sur ses
   achats : rien de particulier.
5. **La page s'ouvre par `HX-Location`** sur `#app-content`, pas par
   `HX-Refresh` : un rechargement complet détruirait le toast qui arrive au
   même moment.
6. **L'annulation, en mode B, ramène chacun sur la page de son équipe** ;
   l'admin et le coach des deux équipes, chez le favori. Elle renvoyait chez le
   favori, page refusée à l'outsider.
7. **« J'ai fini mes achats » vaut pour les deux camps** en mode B ; « Passer »
   disparaît du mode B.
8. **L'attente ne prétend pas que le favori achète** tant qu'il n'a pas lancé
   son fan factor : « X n'a pas encore lancé son fan factor ».
9. **La présence** est déclarée sur l'attente et, en mode B, sur la page
   d'achat. Le récapitulatif ne change pas (cf. « Points ouverts »).
10. **Le mode A** passe sous `garde_camp` sans changer de règle
    (`can_write_side` y rend `can_write_report`) ; seul le refus d'une page
    complète passe par `/step2` avant `/live` — deux 303 au lieu d'un.

Ce que la carte ne disait pas :

- **Une seule règle pour la page et le widget** : `view_for` (contrôleur)
  traduit `inducements_turn()` et le mode en `Purchase`, `Recap`,
  `Wait(équipe attendue)` (mode B) ou `RedirectTo(équipe)` (mode A). Elle
  remplace `redirect_unless_turn_of`.
- **Le widget d'attente** (`GET /inducements/{team_id}/wait`) est sous
  `garde_camp`, qui vérifie du même coup que l'équipe est du match. Il nomme le
  coach attendu (`find_team_coach`, `find_coach_name`), son équipe à défaut.
- `CancelInducementsOutcome` porte le mode et la portée du match, pour que le
  contrôleur choisisse la page de reprise ; `PorteeRapport` dérive `Debug` et
  `Clone`.
- `post_inducements` est découpé (`build_command`, `execute_command`,
  `outcome_response`, `record_error_response`, `refusal_response`), sans
  changer ses réponses.
- `ensure_team_entry_inducements` passe chaque page par le profil de son
  coach ; `tests/e2e/visual/urls.py` reçoit aussi l'avant-match d'un camp
  (`step2/{side}`), oublié par la 654.

## Points ouverts

- Le coach resté sur le **récapitulatif** quand l'autre annule ne reçoit que le
  toast : la page ne se recharge pas d'elle-même.
- Un tier sans coups de pouce n'a pas d'attente : l'avant-match mène au direct,
  comme avant (coup d'envoi en mode B : carte 656).

## Conséquences sur les données de production

Aucune. Aucun événement ajouté : `InducementsRecorded` porte déjà la validation
du favori, et le mode A enchaîne comme aujourd'hui. Ni colonne, ni migration ;
le mode B ne se choisit pas avant la 659. En mode A, seul le refus d'une page
d'achat à qui ne peut pas écrire fait un 303 de plus (décision 10).

## Terminé quand

En mode B, l'outsider voit qu'il attend le favori, et sa page d'achat s'ouvre
d'elle-même dès que le favori a déclaré « J'ai fini mes achats ».
