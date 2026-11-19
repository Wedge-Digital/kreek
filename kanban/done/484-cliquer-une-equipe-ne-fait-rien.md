# Cliquer une équipe ne fait rien

**Priorité : haute** — défaut visible en production, reproductible à chaque clic
**Dépend de :** rien · **Sans épic**
**Trouvée par :** l'utilisateur, sur `demo.bloodbowlclub.com`

## Le symptôme

Depuis « Mes équipes », cliquer une équipe **ne change rien à l'écran**. Il faut
rafraîchir pour que la fiche apparaisse. Reproductible à tous les coups.

## Le mécanisme

Le clic est une navigation htmx — `hx-get` vers la fiche, `hx-target` et
**`hx-select` sur `#app-content`**. Le navigateur envoie donc `HX-Request: true`.

Or le handler de la fiche interprète cet en-tête comme « on me demande le
contenu d'un onglet » et ne renvoie que la zone d'onglets, qui ne contient pas
`#app-content`. `hx-select` n'y trouve rien, **htmx n'échange rien**, la page ne
bouge pas. Le rafraîchissement n'envoie pas l'en-tête : la page entière arrive.

Mesuré sur la démo, dans la session d'un utilisateur réel :

| | Octets | Contient `#app-content` |
|---|---|---|
| Le clic (`HX-Request`) | 2 662 | **non** |
| Le rafraîchissement | 8 878 | oui |

## L'origine

```
5fc5c93 · 2026-08-29 18:51 · feat(teams): [434] la fiche équipe accueille des onglets
```

La carte 434 a fait servir un fragment sous `HX-Request` pour le clic d'onglet,
sans voir que **la même route est aussi la cible d'une navigation htmx venue
d'ailleurs**, qui a besoin de la page entière.

Sa spec écartait explicitement une seconde route — « elle doublerait la surface
pour la même réponse ». L'argument tenait ; ce qui manquait, c'est que l'en-tête
`HX-Request` ne distingue pas les deux usages, puisqu'il est vrai des deux.

## Ce qui est atteint

Cinq points d'entrée partagent le patron `hx-select="#app-content"` :

| Où | Fichier |
|---|---|
| Carte d'équipe de « Mes équipes » | `components/team-card.html` |
| Carte d'équipe archivée | `widgets/my-teams-widget.html` |
| Retour « ← équipe » d'une fiche joueur | `player-detail.html` |
| Ligne du classement | `widgets/classement-widget.html` |
| Ligne du classement détaillé | `widgets/detailed-standings-widget.html` |

## Ce qui n'est pas atteint, et pourquoi c'est instructif

La page de compétition emploie **le même patron** `HX-Request → fragment` et
fonctionne : son fragment est enraciné sur `#app-content`, que `hx-select`
retrouve donc. Vérifié — 8 818 et 9 535 octets, l'identifiant présent dans les
deux.

**Le patron n'est donc pas fautif.** La faute est d'avoir renvoyé un fragment
*plus profond* que ce que les appelants savent sélectionner.

## La correction : renverser la charge

Le serveur rend **toujours la page entière**, et c'est la barre d'onglets qui
dit ce qu'elle veut, par `hx-select="#team-tab-zone"`.

C'est le rôle de `hx-select`, et surtout : **le défaut par défaut devient sûr**.
Tout nouveau point d'entrée fonctionne sans rien savoir de la page, là où
aujourd'hui il faut deviner à quelle profondeur elle répond.

L'autre voie — garder la branche en la fondant sur l'en-tête `HX-Target`, qui
distingue bien les deux cas — économiserait la bande passante mais laisserait le
serveur **deviner l'intention de l'appelant**. C'est exactement ce qui vient de
casser, et un sixième point d'entrée avec une autre cible casserait de nouveau.

Coût assumé : la page complète rendue à chaque clic d'onglet, ~9 Ko au lieu de
2,7. C'est déjà ce que fait la page de compétition.

## Tests

Le défaut a vécu deux jours sous une suite verte, et c'est ce qu'il faut
corriger d'abord.

| Test | Ce qu'il prouve |
|---|---|
| `la_fiche_repond_toujours_une_page_entiere` | l'invariant, unitaire — plus de branche |
| `chaque_onglet_selectionne_la_zone_et_non_la_page` | le pendant côté gabarit |
| `test_cliquer_une_equipe_depuis_mes_equipes_l_affiche` | e2e — **le clic de l'utilisateur** |
| `test_le_retour_depuis_une_fiche_joueur_affiche_l_equipe` | e2e — un second point d'entrée |

Aucun test existant ne pouvait le voir : ils appelaient la route directement, ou
cliquaient des onglets **déjà dans la page**. Aucun n'entrait dans la fiche par
un clic venu d'ailleurs.

## Checklist

- [x] `rendre_fiche` sans branche `HX-Request` — et `headers` retiré, devenu inutile
- [x] Les trois onglets en `hx-select="#team-tab-zone"` / `hx-swap="outerHTML"`
- [x] Cinq tests, chacun falsifié
- [x] Vérifié à l'écran : le clic depuis « Mes équipes » affiche la fiche
- [x] `make lint && make test && make check-arch && make e2e` — 336 passés, 7 ignorés

---

# Ce que la réalisation a appris

## `outerHTML`, et non `innerHTML`

`hx-select` retient l'élément **avec** son enveloppe. Un `innerHTML` aurait
niché `#team-tab-zone` dans lui-même — deux fois le même identifiant dans le
DOM, et le second échange n'aurait plus trouvé sa cible. Vérifié à l'écran après
un changement d'onglet : une seule zone, un seul conteneur de contenu.

## Un de mes tests ne gardait pas ce qu'il annonçait

`la_page_porte_le_conteneur_que_les_appelants_selectionnent` construit le gabarit
directement, comme tous les tests de ce module — il ne passe **jamais** par le
handler. Remettre la branche `HX-Request` dans `rendre_fiche` le laisse au vert :
constaté, pas supposé.

Son commentaire le dit maintenant, et renvoie au test e2e qui, lui, couvre ce
chemin. Un test qui prétend garder un invariant qu'il ne touche pas est pire
qu'un test absent — il occupe la place.

## Pourquoi rien ne pouvait le voir

Les tests unitaires appellent la route directement ou cliquent des onglets
**déjà présents dans la page**. Aucun n'entrait dans la fiche par un clic venu
d'ailleurs, et c'est là tout le défaut : **la fiche se rendait parfaitement,
seuls ses appelants ne pouvaient rien en extraire.**

C'est aussi ce qui explique les deux jours en production sous une suite verte, et
pourquoi la carte 434 a paru complète : ses six tests vérifiaient la page et ses
onglets, jamais l'entrée dans la page.

## Falsification

| Mutation | Constaté |
|---|---|
| Un onglet perd son `hx-select` | `chaque_onglet_selectionne_la_zone…` rouge |
| La branche `HX-Request` revient — le défaut d'origine | **les trois e2e rouges**, le test unitaire **survit** |

La seconde ligne est la leçon de cette carte : le seul filet qui tienne sur ce
chemin est au navigateur.

## Ce qui reste à faire, et qui n'est pas de mon ressort

La correction est sur `demo` ; **la démo tourne encore sur la version fautive**.
Le défaut y persistera jusqu'au prochain déploiement.
