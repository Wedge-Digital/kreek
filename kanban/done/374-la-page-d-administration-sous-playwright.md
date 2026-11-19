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

- [x] Neuf scénarios
- [ ] ~~le dernier administrateur a son sélecteur figé~~ — **le scénario ne peut
      pas exister**, voir ci-dessous. Remplacé par deux mutations enchaînées,
      qui vérifient ce qui reste observable : l'échange de ligne lui-même
- [x] Chaque action attend l'**état résultant**, jamais une durée
- [x] Aucun `dispatch_event`
- [x] Suite lancée **cinq fois** sans échec
- [x] `make e2e` : 206 passés, 3 échecs — **aucun de mon fait**, cf. ci-dessous
- [x] `make lint`, `make check-arch`, `make test` passent — 1144 tests

## Ce qu'on a appris en la faisant

**Le scénario annoncé comme la raison d'être de la carte n'existe pas.**

`role_locked = is_self || (is_admin && admins == 1)`. Pour que la seconde clause
joue sans la première, il faudrait une cible seule administratrice, un spectateur
distinct d'elle, et ce spectateur administrateur — sinon la page rend 403. Le
spectateur serait donc un second administrateur, et la cible ne serait plus
seule.

C'est **le même raisonnement** qui a montré, en carte 371, que
`DernierAdministrateur` est inatteignable depuis le web. Deux découvertes
convergentes : le souci du dernier administrateur est tenu par `is_admin()` et
`is_self`, et tout ce qui est bâti dessus est de la défense en profondeur.

**Les tests ont trouvé deux défauts.**

`hx-on::after-request` est évalué par HTMX dans un scope où l'état Alpine
n'existe pas : `envoye` y était inconnu, et le bouton de réinitialisation ne
basculait jamais. Corrigé en `@htmx:after-request`, où c'est Alpine qui écoute.

Et une assertion ne prouvait rien : elle vérifiait la présence du bouton de
retrait après promotion, or il est là avant comme après. Elle porte désormais sur
le rôle affiché.

**Chaque exécution crée son propre espace**, et ce n'est pas du confort.
`bypass_auth` connecte toujours DevCoach, et `Espace E2E` est partagé par toute
la suite : y muter un rôle ou retirer un membre casserait les autres fichiers.

**Trois échecs à la suite complète, aucun de cette carte.** Celui de la carte
360, connu. Et `test_phase4_notifications` et `test_player_spp_spending`, qui
viennent des commits de `demo` sur lesquels ce travail est rebasé — aucun des 55
fichiers modifiés depuis ne touche `players` ni les notifications. Signalés, non
corrigés : hors périmètre.
