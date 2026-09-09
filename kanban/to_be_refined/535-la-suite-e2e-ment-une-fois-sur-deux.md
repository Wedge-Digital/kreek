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

## Observation du 2026-09-08 — un cas de plus, et un détail neuf

Relevé pendant la carte 541, sur `make test-impacted` (65 fichiers, 353 tests) :

| Passage | Résultat |
|---|---|
| 1 | `test_dismissals_banner_leads_to_the_dismissals_page` — échec |
| 2 | 346 passés, 0 échec — **même sélection, même code** |
| le fichier seul, base au gabarit | 4 passés, 1 sauté |

Même profil que le reste : attente plafonnée, dépendant de la charge, verte en
isolation.

**Le détail neuf, et il oriente le diagnostic** : l'élément attendu était
**présent dans le DOM avec le bon texte**, et rapporté `hidden` quatre fois de
suite.

```
expect(page.locator(".cm-verdict-title")).to_be_visible(timeout=10000)
  4 × locator resolved to <div class="cm-verdict-title">Incident majeur</div>
    - unexpected value "hidden"
```

Ce n'est donc **pas** une donnée qui tarde à arriver — l'hypothèse portée
jusqu'ici, « ce que ces écrans attendent n'arrive pas par la réponse HTTP mais
par un app event ». Le contenu était là. Ce qui manquait était sa **visibilité** :
un `x-show`, une classe, une transition — quelque chose côté client qui n'avait
pas encore basculé.

Ça déplace une partie du soupçon du serveur vers le navigateur, et ça rejoint la
fenêtre déjà documentée dans le `CLAUDE.md` : *« HTMX câble le contenu qu'il
insère quelques dizaines de millisecondes après l'avoir rendu visible »*. Ici
c'est le symétrique — le contenu est peint avant d'être révélé.

**À vérifier au raffinage** : combien des échecs observés portent sur un élément
absent du DOM, et combien sur un élément présent mais masqué. Les deux
familles n'ont pas le même remède, et les confondre ferait chercher côté serveur
un défaut qui vit dans la vue.

## Relevé du 2026-09-09 — et ce qu'il contredit

Trois passages consécutifs pendant la carte 517, **sur le même serveur et le
même binaire** :

| # | Sélection | Durée | Échecs |
|---|---|---|---|
| 1 | 65 fichiers | 468 s | 5 |
| 2 | 65 fichiers, **code identique** | 525 s | 18 |
| 3 | **68 fichiers** — suite complète | **479 s** | **0** |

Aucun des cinq échecs du premier ne reparaît au second : les tests tombés
diffèrent entièrement d'un passage à l'autre. Les échecs ne sont donc pas
déterministes par rapport au code, ce qui était déjà su — mais ici c'est mesuré
sur deux passages strictement identiques.

**Ce que ce relevé contredit.** Devant la dégradation 468 → 525 s, l'hypothèse
formée sur le moment était que le serveur se détériore à survivre aux recréations
successives de sa base : `e2e_db.sh` la détruit et la recrée à chaque
`make test-impacted`, en tuant ses connexions par `pg_terminate_backend`.

Le troisième passage la ruine : il est **le quatrième sous le même processus**,
il porte **plus de tests**, il est **plus rapide que le deuxième**, et il est
vert. La dégradation n'était pas cumulative.

**Deux hypothèses écartées, donc, et la même qui reste.** Ni l'accumulation dans
la base (déjà éliminée par la carte 536), ni l'usure du serveur : c'est bien la
**charge de la machine** au moment du passage. La moyenne relevée entre les
passages 2 et 3 était de 3,85 / 4,84 / 7,29 — quelque chose d'autre tournait, et
c'est la seule variable qui ait changé.

**Une piste écartée en cours de route, à ne pas refaire** : les neuf processus
« chromium » que `pgrep` remontait ne sont pas des navigateurs Playwright
oubliés, mais les serveurs CEF de RustRover et PyCharm, en place depuis plus d'un
jour. Chercher une fuite de navigateurs est une impasse.

## Outillage posé le 2026-09-09 — et la ligne de base du pool

Deux instruments, parce que trois passages rouges ont été perdus le même jour
faute de les avoir gardés.

**`make e2e` conserve sa sortie** dans `.e2e-out/suite-<horodatage>.log`. Un
rouge est désormais analysable sans relancer huit minutes — c'est exactement la
preuve que ce raffinage attend, et elle avait été jetée trois fois.

Un piège évité au passage : derrière un `tee`, le code de sortie lu est celui de
`tee` et non celui de pytest. La cible aurait été **verte quel que soit le
verdict**, comme le `make lint` qui affichait jadis une étape d'audit jamais
exécutée. D'où `SHELL := /bin/bash` sur cette cible seule, pour `PIPESTATUS`.

**`scripts/e2e_pool_watch.sh` échantillonne le pool** toutes les cinq secondes,
en parallèle de la suite, dans `.e2e-out/pool-<horodatage>.tsv`.

### Pourquoi le pool, et ce que la mesure vaut

La page qui expire à 30 s dans les passages rouges répond en **3 ms au repos** —
un facteur dix mille. Le serveur n'est donc pas lent : il est **bloqué**. Le pool
(20 connexions) était le suspect naturel, une fuite faisant attendre les tests
tardifs, empirant au fil d'un passage et se réparant après que sqlx a moissonné
les inactives.

Cette mesure **invalide aussi un levier qu'on allait proposer** : recompiler le
serveur e2e en `--release`. À 3 ms, il n'y a rien à optimiser.

### La ligne de base, sur un passage vert (368 tests, 424 s)

85 échantillons, pool configuré à 20 :

| Mesure | Valeur |
|---|---|
| plage de travail | **9 à 13 connexions** |
| maximum | **13**, atteint deux minutes après le départ, puis stable |
| `idle in transaction` | **1** sur 85 échantillons |
| verrous en attente | **0** |
| connexions **actives** | **1**, en permanence |

Une seule connexion active à tout instant : c'est la signature d'une suite
sérielle. Pour atteindre 20 il faudrait donc une **fuite**, pas de la
concurrence.

### Ce que la mesure ne peut pas dire

`pg_stat_activity` montre l'état **côté Postgres**, pas côté sqlx. Une connexion
empruntée par un handle fuité apparaît `idle` exactement comme une connexion
disponible dans le pool : le détail actives/inactives ne les distingue pas.

Ce qui reste décisif est binaire : **si le total atteint 20, le pool est plein ;
en dessous, il n'y a pas de famine.** Le prochain rouge tranchera d'un coup
d'œil, et l'hypothèse tombera ou tiendra sans discussion.

## Parallélisation — écartée comme remède, envisageable comme gain de vitesse

Question posée le 2026-09-09 : huit à seize workers amélioreraient-ils la
stabilité ? **Non**, et trois obstacles structurels l'expliquent.

**Aucune isolation.** `space_id` est une fixture `scope="session"` qui résout un
**seul** espace, partagé par les 368 tests. Deux workers y créant des
compétitions se marchent dessus : listes, compteurs, et les assertions
d'unicité de nom — `test_un_nom_deja_pris_s_affiche_sous_le_champ` existe.

**L'ordre intra-fichier compte.** Constaté en tentant d'isoler un test :
`test_dismissals_banner_leads_to_the_dismissals_page` dépend du test qui le
précède pour faire avancer la phase de l'équipe. `--dist loadfile` préserverait
ça, pas le partage d'état entre fichiers.

**La charge est la cause diagnostiquée.** Seize Chromium et autant de requêtes
concurrentes sur un serveur unique en build debug, c'est multiplier la variable
responsable. Côté base la marge existe — une active sur vingt — donc le goulot
serait le serveur, pas Postgres.

Elle reste jouable **après** correction de la cause, et comme gain de vitesse
seulement : il faudrait une base, un serveur et un espace **par worker**. La
carte 536 a rendu le clonage d'une base instantané ; N serveurs coûtent des
ports, de la RAM et une orchestration. Le faire avant de connaître la cause
industrialiserait le défaut en huit exemplaires.

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
