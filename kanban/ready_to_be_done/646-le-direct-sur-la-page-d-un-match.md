# Le direct sur la page d'un match

**Épic :** E23 — Le temps réel : connexions, présence et notifications · **Ordre :** 3 · **Dépend de :** 642, 643, 644, 645
**Priorité :** normale
**Conception :** `docs/specs/saisie-mobile/E23-cadrage.md` (R4, § 4 « Le client », « Le routage », § 5, § 9, § 11.3, 11.8)
**Maquette :** `assets/rawpages/html/app-mobile-solo-follow.html`

## Objectif

Qui regarde `/live` voit le journal et le score bouger au moment où l'autre
saisit, sans recharger et sans interroger le serveur toutes les 15 s. Le canal
n'envoie qu'un signal — « le match X a changé » — et la page recharge ses
widgets par leurs routes, qui gardent leurs droits.

## Ce que le code fait

- **Le suivi interroge déjà** : pour qui ne peut pas écrire, le journal se
  recharge `every 15s` (`src/app/match_report/io/web/templates/live-match.html:56`, carte 636).
- **Le score est rendu dans la page** (`live-match.html:19-40`) : seul le
  journal est un widget (`:54-59`), et un signal ne le rafraîchirait pas.
- Le journal de saisie se recharge sur `actionRecorded`/`actionDeleted` locaux
  (`actions-step.html:119-121`).
- Aucun écran « l'outsider attend » ni « validations des camps » en « Je saisis
  tout » : ce direct-là appartient à E22.

## Ce qu'il faut faire

- **Le signal** : le routeur de la 645 envoie, pour tout `MatchReportAppEvent`,
  un message `match` (`{ id }`) aux connexions présentes sur ce match, auteur
  compris ; le client émet `matchChanged` sur `body`.
- **Le score en widget** : `GET /{id}/live/score` dans `routes_de_suivi()`
  (garde de E19), rendu depuis le même calcul que la page aujourd'hui ; la page
  ne garde que son conteneur.
- **Les déclencheurs** : le journal et le score de `/live` prennent
  `matchChanged[detail.id=='{{ id }}'] from:body, realtimeResync from:body`.
  Le `every 15s` est **retiré** (décision 8).
- **Le journal** : la livraison d'un signal est comptée dans les compteurs de
  la connexion (`livres`), sans ligne propre.

## Fichiers touchés

`src/web/realtime/notification_router.rs`, `src/app/match_report/io/web/templates/live-match.html`,
`src/app/match_report/io/web/live_match_controller.rs` (le score en widget, son
gabarit et son VM), `src/app/match_report/routes.rs`, `src/app/match_report/router.rs`,
`tests/e2e/visual/urls.py`.

## Tests

- **Unitaires** : le signal ne va qu'aux présents du match, pas aux présents
  d'un autre ; le VM du score, extrait de la page, rend les mêmes chiffres.
- **E2E** (`tests/e2e/test_realtime_direct.py`) : le membre simple suit `/live`,
  DevCoach saisit un touchdown dans un second contexte ; le journal et le score
  changent chez le membre simple sans rechargement, en moins de 15 s. Couper le
  flux (`page.context.set_offline`) puis le rétablir recharge les widgets.
- `test_match_report_access.py` et `test_match_report_solo_entry.py`, qui
  comptaient sur le rechargement, suivent le signal.
- `tests/impact-map.toml` mis à jour.

## Hors périmètre

- L'attente de l'outsider et l'état des validations : E22.
- Le direct sur les pages de saisie : celui qui saisit a déjà ses
  rechargements locaux.

## Conséquences sur les données de production

Aucune. Le suivi cesse d'interroger le serveur toutes les 15 s : la charge
baisse. Si le canal tombe, `/live` ne se met plus à jour qu'au rechargement —
c'est voulu, une panne du canal doit se voir.

## Terminé quand

- [ ] L'autre coach voit le journal et le score changer sur `/live` sans recharger
- [ ] `/live` n'interroge plus le serveur à intervalle fixe
- [ ] Après une coupure du flux, les widgets se rechargent à la reconnexion
- [ ] Un signal d'un autre match ne recharge rien
- [ ] `make lint`, `make check-arch`, `make test`, e2e impactés
