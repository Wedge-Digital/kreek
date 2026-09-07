# La suite e2e ment une fois sur deux

**Priorité : haute — elle rend chaque carte suivante deux fois plus chère**
**Dépend de :** rien · **Sans épic**
**Trouvée par :** l'enquête de la carte 509, le 2026-09-07

## Le constat, mesuré

Cinq exécutions de `make test-impacted` sur la même machine, à quelques minutes
d'intervalle :

| État du code | Passage | Résultat |
|---|---|---|
| avec la 509 | 1 | `test_competition_admin_settings` — 10 erreurs en cascade |
| avec la 509 | 2 | `test_build_and_finalize_team`, `test_competition_full_lifecycle` |
| avec la 509 | 3 | `test_journeyman_recruitment` |
| **sans** la 509 | 1 | propre — 346 passés |
| **sans** la 509 | 2 | `test_journeyman_recruitment` — deux cas |

**Aucun de ces échecs n'est reproductible en isolation.**
`test_journeyman_recruitment` seul rend douze tests verts en quinze secondes,
deux fois de suite. Les deux autres fichiers passent seuls en cinquante-deux
secondes.

Le même fichier tombe avec et sans la modification en cours : ce n'est donc pas
une régression, c'est une **propriété de la suite**.

## Ce que ça coûte

L'enquête ci-dessus a demandé **cinq passages de sept minutes, deux
reconstructions du serveur et un aller-retour de remise**, pour établir qu'un
échec n'était pas causé par le code qu'on venait d'écrire.

C'est le vrai prix : une suite instable ne fait pas perdre les minutes de ses
faux échecs, elle fait perdre le temps de l'enquête qu'il faut mener à chaque
fois pour savoir s'il faut la croire. Et le jour où l'on cesse de mener cette
enquête, on cesse aussi de voir les vraies régressions.

## La cause probable — à confirmer, c'est le travail de raffinage

Tous les échecs observés sont des **dépassements de délai sur des attentes
asynchrones** :

```
waiting for locator("#player-table-container .tbl-btn") to be visible
Timeout 10000ms exceeded
```

Ce que ces écrans attendent n'arrive pas par la réponse HTTP mais par un app
event : un journalier créé par listener, une projection alimentée après coup.
Les helpers attendent une durée, ou une condition avec un délai fixe — et sous
la charge de trois cent cinquante tests enchaînés, ces durées ne suffisent
plus.

C'est exactement la famille de défaut que la **carte 483** a déjà corrigée une
fois : *« trois copies d'une boucle de recrutement comptaient les clics derrière
un délai fixe, pas les embauches enregistrées ; la CI de `demo` échouait une
fois sur deux »*. Le remède y était d'attendre une **condition observable** —
l'embauche enregistrée — plutôt qu'une durée.

Le même remède s'applique ici, et il reste à savoir **où** : quels helpers
attendent une durée, lesquels attendent une condition mais avec un plafond trop
court, et lesquels attendent la mauvaise chose.

## Ce qu'il faudra trancher au raffinage

**Relever les délais n'est pas la réponse.** Ça repousse le seuil sans rien
apprendre, et ça allonge la suite pour tout le monde. Un test qui attend trois
fois plus longtemps la même chose échouera trois fois plus tard.

**Trois questions ouvertes :**

1. Combien de points d'attente asynchrone la suite porte-t-elle, et lesquels
   sont des durées plutôt que des conditions ? (`_attendre`,
   `wait_ranking_lines`, `wait_ranking_points`, `page.wait_for_selector` avec
   `timeout=`…)
2. Certains échecs sont-ils des **cascades de fixture** plutôt que des tests
   propres ? Le premier passage a produit dix erreurs sur un seul fichier, ce
   qui ressemble à une fixture de session tombée.
3. La suite doit-elle rester séquentielle ? Sept minutes pour trois cent
   cinquante tests, c'est ce qui crée la charge sous laquelle les délais
   cèdent.

## Pourquoi cette carte est en `to_be_refined`

Le constat est solide et mesuré ; le périmètre ne l'est pas. On ne sait pas
encore si c'est un helper à corriger ou trente, ni si la cascade du premier
passage est le même défaut ou un second.

**Ne pas commencer par corriger un test.** Commencer par instrumenter : faire
tourner la suite complète cinq fois en enregistrant les échecs, et voir si
l'ensemble se referme sur une poignée de fichiers ou s'étale sur tout.
