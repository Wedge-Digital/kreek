# La page d'administration sous Playwright

**Priorité : haute** — la règle de couverture du projet l'exige
**Dépend de :** 368 à 372
**Conception :** `docs/specs/space-admin/membres/07-integration.md`
**Fichiers :** `tests/e2e/test_space_admin.py`, `tests/e2e/visual/urls.py`

## Objectif

Le test unitaire vérifie la logique ; le test e2e vérifie que le HTML, HTMX et
Alpine produits fonctionnent réellement. Le `CLAUDE.md` rappelle pourquoi : le
bug du widget coach-search et celui des pickers de tiers n'étaient visibles
qu'en navigateur.

## Les scénarios

| Scénario | Vérifie |
|---|---|
| un administrateur ouvre la page | les quatre onglets, Membres actif |
| la liste affiche pseudo, email et rôle | le rendu, donc le VM |
| promouvoir un membre | la ligne se re-rend en Admin, le compteur passe à 2 |
| rétrograder, deux administrateurs | la ligne se re-rend en Membre, compteur à 1 |
| **le dernier administrateur a son sélecteur figé** | `role_locked`, après rétrogradation de l'autre |
| retirer un membre | la ligne disparaît, le compteur décroît |
| sa propre ligne | sélecteur désactivé, pas de bouton de retrait |
| la recherche filtre la liste | filtre Alpine, sans requête |
| un `SpaceUser` ouvre l'URL | 403 |

## Le cinquième est le plus utile de la liste

Il enchaîne deux opérations — rétrograder l'un, constater que l'autre se fige —
et c'est **le seul** qui vérifie que le re-rendu de ligne transporte bien le
nombre d'administrateurs **postérieur**.

C'est précisément ce qui a motivé le retour `ChangementDAppartenance` en phase 5.
Sans ce scénario, la conception est là et rien ne prouve qu'elle sert.

## Le piège des tests d'échange HTMX

Un clic sur un élément que sa propre requête remplace peut être **rejoué par
Playwright** : `click()` vérifie l'actionnabilité pendant l'action et recommence
si l'élément disparaît sous lui, ce qu'un `hx-swap` sur l'élément cliqué fait par
construction. Sur un bouton qui bascule, le second clic annule le premier.

Vécu sur `test_dismissals_phase`. Le remède n'est **pas** `dispatch_event`, qui
court-circuite l'actionnabilité et clique parfois trop tôt : c'est d'attendre
l'état réel après chaque action — les deux zones qu'un clic rafraîchit, pas
seulement la première.

## Checklist

- [ ] Les neuf scénarios
- [ ] Chaque action attend l'**état résultant**, jamais une durée
- [ ] Aucun `dispatch_event` — attendre les deux zones rafraîchies
- [ ] Suite lancée **cinq fois** de suite sans échec : un test d'échange HTMX
      instable ne se voit pas en une passe
- [ ] URL de la page dans `tests/e2e/visual/urls.py`, classe de portée dans
      `CLASSE_ATTENDUE`
- [ ] `decalages.py` rend **0 px** sur la page, desktop et sous 768 px
- [ ] `make e2e` passe — l'échec de la carte 360 mis à part s'il subsiste
