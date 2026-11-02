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

## Ce qu'il faut faire

1. Après le clic, attendre **aussi** que le navigateur ait quitté l'étape de
   validation (`page.wait_for_url` sur une URL qui ne finit plus par
   `/validation`, ou `expect_navigation` autour du clic), en gardant
   l'attente de la finalisation en base.
2. Vérifier la destination réelle du `HX-Redirect` pour écrire l'attente sur
   elle plutôt que sur l'absence de `/validation`, si elle est stable.
3. Chercher les autres aides qui finissent par un clic à `HX-Redirect` suivi
   d'un retour immédiat — même forme, même course.

## Hors du périmètre

Le code produit : la redirection après création est le comportement voulu.

## Terminé quand

- [ ] `create_full_competition` ne rend la main qu'une fois la redirection
      faite ;
- [ ] `test_special_rule_selector.py` passe sur dix exécutions de suite ;
- [ ] la suite complète passe, l'aide étant partagée par douze fichiers.

## Conséquences sur les données de production

Aucune : seule une aide de test change.
