# Deux tests lisent la projection avant que le bus ait fini

**Priorité : haute** — un test rouge par intermittence, un test vert pour rien
**Contexte :** suite e2e · **Sans épic**

## Le constat

`test_haine_journalier.py` publie un rapport de match, puis lit `players_proj`
dans la foulée :

```python
_publier(space_id, mr)
assert _competences_de_blessure(domicile) == avant + 1
```

Or la publication déclenche des **app events**, traités dans une tâche séparée :
c'est `player_match_impact_listener` qui écrit la Haine dans `acquired_skills`.
`CLAUDE.md` le pose en exception explicite — un listener cross-BC reçoit un
événement déjà commité ailleurs, « ce cas reste asynchrone par nature ».

Le fichier ne contient **aucune** attente : `grep` sur `_wait|attendre|sleep`
n'y rend rien. Ses voisins, eux, en ont — et disent pourquoi :

> « Les impacts joueur transitent par l'app event bus : le statut n'est pas à
> jour au retour de la requête de publication. »
> — `test_player_availability_after_injury.py`

## Deux défauts, pas un

**`test_la_haine_d_un_joueur_permanent_atteint_l_effectif` tombe par
intermittence.** Observé : vert sur une suite en 6 min 05, rouge sur la même
suite en 8 min 07 — machine plus chargée, fenêtre élargie. Message trompeur, il
accuse le produit (« la Haine doit rejoindre ses compétences acquises ») pour un
défaut de test.

**`test_la_haine_d_un_journalier_n_atteint_aucun_joueur` est vert pour rien.**
Il assert `apres == avant`, c'est-à-dire *qu'il ne s'est rien passé*. Sans preuve
que le pipeline a tourné, l'assertion passe aussi bien si **rien n'arrive
jamais** — chaîne cassée comprise. C'est exactement le reproche que son propre
frère lui adresse en docstring : « sans lui, "aucune compétence en mode Injury"
passerait aussi bien si la chaîne entière était cassée ». Le frère a été écrit
pour couvrir ce trou, et il est le seul des deux à pouvoir échouer.

Il porte en plus une troisième course : `assert journalier` au second match
suppose que la blessure du premier a été projetée. Si elle ne l'est pas encore,
l'équipe est encore à onze, aucun journalier n'est adjoint, et le test échoue en
accusant la règle des journaliers.

## Ce que ce n'est pas

**Pas le piège du câblage htmx.** `page` vaut `about:blank` dans la trace : le
navigateur n'a jamais navigué, tout passe par `requests` et `query_db`. Aucun
clic, donc aucun câblage en jeu. C'est une **seconde famille de course**, à
distinguer de la première.

**Pas une régression.** Le test passait sur la suite complète de la veille —
354 passés, zéro échec.

## La correction

Une attente conditionnelle partagée, `attendre_que`, dans `db_helpers.py` — à
côté de `query_db`, puisque c'est la projection qu'on interroge. **Pas un
`sleep`** : une durée fixe n'a pas de marge sur une machine chargée, et c'est
précisément là que la course s'ouvre.

Pour l'assertion **positive**, la condition *est* l'assertion : on attend que le
compte atteigne sa valeur, et l'échec au bout du délai porte le bon message.

Pour l'assertion **négative**, il faut d'abord une preuve que le pipeline a
tourné, sans quoi on ne mesure que sa propre impatience. `players_proj.version`
s'incrémente à chaque événement appliqué : on attend que la somme des versions
de l'effectif bouge, **puis** on vérifie que le compte de compétences n'a pas
changé. Sans ce marqueur, l'assertion négative reste creuse.

## Ce que la carte ne fait pas

**Elle ne mutualise pas les attentes des autres fichiers.**
`test_player_spp_spending.py` porte son `_wait_for`,
`test_player_availability_after_injury.py` son `_wait_status` : trois copies de
la même idée. Les faire converger toucherait des tests qui passent, pour un gain
d'écriture — à faire quand un quatrième apparaîtra.

**Elle ne traite pas l'autre famille de flake.** Le câblage htmx reste entier :
59 `page.click` sur 60 n'attendent pas, alors que `cliquer_quand_cable()` existe.

## Checklist

- [x] `attendre_que(condition, ...)` dans `db_helpers.py`, sans `sleep` fixe
- [x] Le test positif attend son compte au lieu de l'asserter sèchement
- [x] Le test négatif attend `players_proj.version` avant d'asserter
- [x] La course sur `assert journalier` fermée elle aussi
- [x] `make lint`, `make check-arch` (17 axes), `make test`,
      `make e2e` (**356 passés**, suite complète 67/67, 0 échec)

## Terminé quand — et ce qui n'a pas pu être vérifié

Les deux tests passent sur la suite complète. C'est constaté : 356 passés, zéro
échec, avec ces changements dans l'arbre.

**Ce qui n'a pas été démontré** : que le correctif attrape la course. Sur une
machine déchargée, la projection est déjà à jour au retour de la publication —
avec un délai ramené à 10 ms, le test positif passe encore. La course ne s'ouvre
que sous charge, et la reproduire demanderait de charger la machine
artificiellement.

Ce qui a été vérifié à la place, directement sur le helper :

    ✓ échoue après 1,0 s : « une condition qui ne vient jamais n'est toujours
      pas vraie après 1 s — le bus d'app events n'a rien projeté, ou la chaîne
      est cassée »
    ✓ rend la main immédiatement quand c'est déjà vrai (0,000 s)

Le garde-fou garde, et ne coûte rien quand tout va bien. **La démonstration
qu'il attrape le cas réel reste à faire le jour où la course se reproduit.**
