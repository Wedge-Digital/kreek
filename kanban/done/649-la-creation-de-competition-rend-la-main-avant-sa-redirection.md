# La création de compétition des e2e rend la main avant sa redirection

**Épic :** aucune · **Dépend de :** rien
**Priorité : moyenne** — un échec qui change d'une exécution à l'autre, dans
une aide dont dépendent douze fichiers e2e
**Trouvée par :** la suite e2e complète de la carte 643 (2026-11-02). Sans
rapport avec elle : la 643 ne touche que `match_report`.

## Le symptôme

`test_special_rule_selector.py` échoue sur deux tests, puis passe quatre sur
quatre à la relance suivante, sans aucun changement :

```
playwright._impl._errors.Error: Page.goto: net::ERR_ABORTED at
http://localhost:3211/app/<space>/team/create
```

Au moment de l'échec, la page est encore sur l'étape de validation du
magicien de compétition (`…/competitions/create/<id>/<id>/validation`).

## La cause

`create_full_competition` (`tests/e2e/competition_lifecycle.py:187-188`)
clique « Créer la compétition », puis attend la finalisation **en base**
(`_attendre_finalisation(season_id)`) et rend la main.

Le serveur répond à ce clic par un `HX-Redirect`
(`src/app/competitions/io/web/new_competition_phase_5.rs:367`) : le navigateur
part donc vers une autre page **après** la réponse. Si le test enchaîne un
`page.goto(...)` pendant que cette redirection est encore à venir, htmx
l'interrompt — `ERR_ABORTED`.

La base est finalisée avant que la redirection ait eu lieu : l'attente porte
sur le bon fait métier, mais pas sur l'état du navigateur que le test réutilise.
Plus la machine est chargée, plus la fenêtre est large (cf. carte 641).

## La seconde cause — dans le produit (2026-11-11)

Après E23, le même fichier échouait aussi sur un autre symptôme :
`Page.wait_for_selector: Timeout 10000ms` sur
`#player-table-container .tbl-btn`, dans
`test_finalize_blocked_when_choice_roster_has_no_special_rule`. Sur dix
exécutions du fichier : cinq vertes, trois `ERR_ABORTED` (la cause ci-dessus),
quatre délais dépassés.

**La cause est dans le gabarit** `build-team.html`. `#player-table-container`
porte `hx-trigger="load, rosterSelected from:body"` et
`hx-vals='js:{"roster_uid": (event?.detail?.uid || …)}'`.

1. Le roster est choisi pendant que la requête `load` (`roster_uid=` vide) est
   encore en vol.
2. htmx 2.0.4 met la requête `rosterSelected` en file (stratégie `last` par
   défaut) et la rejoue quand la première revient.
3. Il évalue `hx-vals` par `Function("return (…)")()`, sans paramètre :
   `event` y est `window.event`. Au rejeu, ce n'est plus le choix du roster.
4. La requête part avec `roster_uid=` vide : le tableau reste sans roster.

Un coach peut y tomber aussi, s'il choisit son roster pendant le chargement.

**E23 élargit la fenêtre** : chaque onglet garde deux connexions ouvertes en
permanence (`/app/realtime/stream`, `/_tower-livereload/event-stream`) sur les
six que Chrome accorde par hôte en HTTP/1.1. Il en reste quatre pour les sept
chargements htmx de /build, et la requête `load` du tableau reste en vol plus
longtemps.

**Prouvée à coup sûr** : la requête `load` retenue par `page.route`, le roster
choisi, puis la requête relâchée — la requête rejouée part avec `roster_uid=`,
trois fois sur trois.

**Le correctif** : `hx-on::config-request` sur `#player-table-container` fixe
`roster_uid` depuis `event.detail.triggeringEvent`, que htmx conserve au rejeu.
`hx-sync="this:replace"` a été écarté : il annule la requête en vol, ce qui
écrit `htmx:sendAbort` dans la console (refusé par `conftest.py`) et laisse
côté serveur une requête interrompue (carte 317).

Les autres `hx-vals` qui lisent `event` ont la même fragilité : ils relèvent
de la carte **666**, pas de celle-ci.

## Ce qu'il faut faire

1. Après le clic, attendre **aussi** que le navigateur ait quitté l'étape de
   validation (`page.wait_for_url` sur une URL qui ne finit plus par
   `/validation`, ou `expect_navigation` autour du clic), en gardant
   l'attente de la finalisation en base.
2. Vérifier la destination réelle du `HX-Redirect` pour écrire l'attente sur
   elle plutôt que sur l'absence de `/validation`, si elle est stable.
3. Chercher les autres aides qui finissent par un clic à `HX-Redirect` suivi
   d'un retour immédiat — même forme, même course.

4. Seconde cause : `hx-on::config-request` sur `#player-table-container`, et
   un test de non-régression qui provoque la file à coup sûr.

## Hors du périmètre

- La redirection après création : c'est le comportement voulu. Côté produit,
  seul le gabarit `build-team.html` change, pour la seconde cause.
- Les autres `hx-vals` qui lisent `event` (`#league-selector-zone`,
  `#special-rule-selector-zone`, `finalize-team.html`) : carte 666.

## Terminé quand

- [ ] `create_full_competition` ne rend la main qu'une fois la redirection
      faite (`publish_competition`, partagée avec `test_ranking_bonus.py`) ;
- [ ] `test_player_table_keeps_roster_when_request_is_queued` échoue sur le
      gabarit d'avant et passe sur le nouveau ;
- [ ] `test_special_rule_selector.py` passe sur dix exécutions de suite ;
- [ ] la suite complète passe, l'aide étant partagée par douze fichiers.

## Conséquences sur les données de production

Aucune : seuls les paramètres d'une requête GET changent (le roster demandé
pour le tableau des joueurs), et des aides de test.
