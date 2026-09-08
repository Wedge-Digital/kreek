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

## Ce que le raffinage a mesuré, le 2026-09-07

### Sept passages, et une base repartie de zéro au milieu

| Base | Passage | Durée | Résultat |
|---|---|---|---|
| accumulée (155 Mo) | 1 | 6 min 50 | `admin_settings` — 10 erreurs en cascade |
| accumulée | 2 | 6 min 46 | `build_and_finalize_team`, `full_lifecycle` |
| accumulée | 3 | 6 min 25 | `journeyman_recruitment` |
| accumulée | 4 | 6 min 19 | **propre** |
| accumulée | 5 | 6 min 46 | `journeyman_recruitment` — deux cas |
| **neuve (9 Mo)** | 6 | **5 min 22** | `journeyman_recruitment` |
| **neuve** | 7 | **5 min 10** | **propre** |

### Hypothèse écartée : l'accumulation n'est pas la cause

**Et elle ne ralentissait presque pas non plus.** Le tableau ci-dessus compare
des passages qui n'exécutaient pas le même nombre de tests — 306 contre 368 — et
la différence de durée venait de là. À sélection identique, la base accumulée
coûte **2 %**, pas 25 %. La mesure a été refaite à la carte 536.

La base de développement n'est **jamais purgée**. Après une journée
d'exécutions elle pesait 155 Mo, dont 94 % de résidus de tests — 44 135
joueurs, 77 222 événements de joueur, 46 795 lignes de journal.

**Un échec est survenu dès le premier passage sur base neuve** : l'accumulation
ne dérègle rien.

Elle a néanmoins été traitée par la **carte 536** — non pour la vitesse, mais
pour que deux passages partent du même état. Sans cela, l'enregistrement demandé
ci-dessous mélangerait la dérive de la base et la vraie variabilité.

Elle mérite néanmoins sa propre carte : rien ne purge cette base, et un
développeur qui lance la suite chaque jour pendant un mois travaille sur
plusieurs gigaoctets sans qu'aucun signal ne le lui dise.

### Hypothèse écartée : les `sleep` nus

La suite en porte 83, mais **aucun des échecs observés ne tombe dessus**. Tous
portent sur des attentes **conditionnées** avec un plafond — `wait_for_selector`
à 10 s, `to_have_value` à 5 s, `_attendre` à 20-25 s. Ce sont des attentes
correctes dont le plafond ne suffit plus.

### Ce qui reste, et qui est le vrai sujet

**Les échecs dépendent de la charge, jamais du contenu.**
`test_journeyman_recruitment` est tombé trois fois sur sept — et passe
systématiquement seul : douze tests verts en quinze secondes, deux fois de
suite, sur la base la plus chargée. Même chose pour les deux autres fichiers
touchés, verts seuls en cinquante-deux secondes.

Ce n'est donc ni une dépendance d'ordre, ni un état partagé : c'est le temps de
réponse du serveur sous trois cents tests enchaînés, contre des plafonds
calibrés à vide.

## La cause probable — reste à instrumenter

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

**Ne pas commencer par corriger un test.** Et surtout, ne pas conclure sur un
seul passage : sur sept, deux étaient propres. Un passage vert ne prouve rien,
et c'est précisément ce qui rend cette carte coûteuse.

**La prochaine étape est un enregistrement, pas une correction.** Il faut que la
suite garde la trace de ses échecs sur N passages — le fichier, le test, le
sélecteur attendu et le plafond dépassé — parce que la trace du passage courant
disparaît avec lui, et qu'on ne peut pas décider quels plafonds corriger sans
savoir lesquels cèdent.

Deux tentatives de capture pendant ce raffinage sont revenues vertes : le
message d'échec de `test_le_journalier_recrute_reste_dans_l_effectif` n'a
toujours pas été lu. C'est la première chose à obtenir.
