# Les tests e2e de l'encart

**Priorité : haute — deux de ces scénarios ne se rencontrent pas à la main**
**Épic :** E16 — Sondage de présence
**Dépend de :** 531
**Fichiers :** `tests/e2e/test_competition_encart_presence.py`, `tests/impact-map.toml`

## Les scénarios

| Scénario | Ce qu'il éprouve |
|---|---|
| coach avec une équipe, campagne ouverte | l'encart apparaît, deux boutons |
| il répond, la page ne bouge pas | l'onglet ouvert reste ouvert, le classement n'est pas rechargé |
| il change d'avis | la bascule, sans rechargement de page |
| coach avec **deux** équipes | deux lignes, une réponse par équipe (R1) |
| coach sans équipe engagée | **rien du tout** — pas d'encart vide, pas de marge |
| campagne close entre l'affichage et le clic | la carte explicative, **pas un fragment vide** |
| après le tirage, il se décommande | R30 — la mention, et **aucun adversaire annoncé** |
| deux campagnes ouvertes | deux cartes, la plus proche échéance en premier |

## Les deux qui valent le plus

**La campagne close entre l'affichage et le clic.** Personne ne rencontre cette
course en développement, et son mode d'échec — un encart qui disparaît
silencieusement — **ressemble à un succès**. C'est la classe de défaut que la
carte 486 a payée : un refus rendant le même `200` qu'un succès, avec une CI
rouge trois runs sur quatre.

**Le coach sans équipe engagée.** Il vérifie une absence, ce qu'on oublie de
tester : que la page est *exactement* celle d'avant — pas seulement qu'aucun
texte n'apparaît, mais qu'aucune marge ne s'est ajoutée. C'est le `outerHTML` de
la carte 531 qui le garantit, et rien d'autre ne le prouverait.

## Le piège habituel

**`cliquer_quand_cable` sur les boutons de l'encart** : il est injecté par htmx,
donc la fenêtre où il est peint mais pas encore câblé s'y présente. Pas de
`sleep`.

## Ce que l'écriture a appris

**Les huit scénarios ne sont pas indépendants, et c'est structurel.** L'encart
agrège *toutes* les campagnes ouvertes de la saison : un scénario qui laisse la
sienne ouverte se voit dans tous les suivants, et celui qui attend deux cartes en
compte trois. Chacun clôt donc la sienne en sortant, et ceux qui dépendent de
l'état posé par le précédent le disent. L'alternative — une compétition par
scénario — aurait coûté huit constructions de fixture pour éviter huit appels à
`close`.

**Le coach connecté est DevCoach, premier de la liste des coachs de l'espace.**
`build_full_competition` distribue ses équipes dans l'ordre des noms : la première
lui revient, et c'est ce qui rend ces scénarios jouables. Le coach « simple » est
le **second** — sur une compétition à quatre équipes il en possède une et verrait
l'encart. D'où une seconde fixture à une seule équipe.

**Le scénario du coach sans équipe a gagné un jumeau.** Une campagne absente, une
route cassée ou un gabarit muet donneraient le même « aucun encart ». Un premier
test montre donc que le propriétaire voit l'encart sur **la même campagne**, et le
second que l'autre ne le voit pas. Séparés, le second ne prouvait rien.

## Un défaut d'échafaudage, trouvé en le faisant échouer

`_lancer` n'assertait que le code HTTP. Or **un refus métier rend `200`** avec un
fragment explicatif : relancer une campagne déjà existante — même close — répond
« une campagne existe déjà » sans rien ouvrir, puisque R2 veut qu'on la *rouvre*.

L'échafaudage était donc vert sur une action sans effet, et le test suivant
accusait l'encart de ne pas s'afficher. Les trois actions vérifient désormais leur
**effet en base** — campagne ouverte, campagne close — et non leur code de retour.

C'est la même famille que le défaut de la carte 486 : un refus qui rend le même
`200` qu'un succès. Ici il était dans le test, pas dans le produit.

## Les deux instruments, éprouvés en les faisant mentir

| Sabotage | Ce qui tombe |
|---|---|
| un `page.reload()` avant l'assertion du témoin | le témoin rend `None` et l'assertion dit « la page a été rechargée » |
| le `{% if !campagnes.is_empty() %}` retiré du gabarit | l'absence attendue devient `1` — le `<div>` vide et sa marge |

Le premier valide **l'instrument** : sans lui, « la page ne bouge pas » ne se
distingue pas d'un rechargement qui redonne le même écran. Le second valide ce que
le `outerHTML` de la carte 531 garantit, et rien d'autre ne le prouverait.

Les deux sabotages ont été retirés, et le `git status` le confirme — aucun fichier
de `src/` ne porte de modification.

## Un piège d'attente, en marge

La boucle de la règle 9 attend un **changement de PID**. Le processus existe alors,
mais n'a pas encore lié son port : la suite lancée dans la foulée s'est heurtée à
un `Connection refused`. Attendre le PID puis **attendre le port** referme le trou
— `until curl -s -o /dev/null --max-time 2 http://localhost:3210/`.

Ce n'est pas dangereux, la `conftest` le détecte et s'arrête proprement. C'est une
minute perdue à chaque fois qu'on l'oublie.

## Checklist

- [x] Le fichier de test
- [x] Les huit scénarios, en neuf tests — celui du coach sans équipe a gagné son
      jumeau, sans lequel il ne prouvait rien
- [x] `cliquer_quand_cable`, aucun `sleep`
- [x] Les trois actions d'échafaudage vérifient leur effet en base, pas leur code
      HTTP
- [x] Les deux instruments éprouvés par sabotage, puis restaurés
- [x] `tests/impact-map.toml`, **même commit**
- [x] `make e2e` passe (serveur dev lancé par l'utilisateur)
- [x] `make lint`, `make check-arch`, `make test`

---

## Suite — deux échecs en CI, et ce qu'ils ont appris (2026-09-14)

La suite locale était verte ; la CI a rendu **deux échecs** au premier passage
qui l'a exécutée. Les deux venaient de ce fichier, et un seul était réel.

### Le vrai : un test dont le verdict dépendait du tirage

`test_j4` cliquait sur `.presence-call .pc-btn--switch` — **la première ligne
venue**. Or DevCoach engage deux équipes dans cette saison, ce qui en fait cinq
présentes, donc deux rencontres et **une exemptée** (R9). Se décommander depuis
l'exemptée ne défait aucune rencontre : pas d'avis R30, et le test tombe.

Il ne tombait pas toujours. Mesuré en rejouant l'ancien sélecteur six fois sur le
fichier complet : **un échec sur six**. Il passait en local par chance de tirage,
quatre fois de suite.

Le test lit désormais les appariements de la journée et clique sur le bouton de
**l'équipe qui y figure**, ciblé par son `hx-vals`. La dépendance au tirage
disparaît par construction, pas par répétition.

### Le faux : l'ombre du premier

`test_j5` attendait deux cartes et en a compté trois. Motif : J4 avait échoué
**avant** sa dernière ligne, celle qui refermait sa campagne — et l'encart agrège
toutes les campagnes ouvertes de la saison.

Le fichier documentait pourtant ce couplage en tête, et la clôture était bien
écrite dans chaque scénario. Elle l'était **en dernière ligne du corps du test**,
c'est-à-dire à l'endroit qui ne s'exécute pas quand le test échoue — donc
exactement quand elle compte.

> Un `try` sans `finally` documente une intention ; il ne l'exécute pas.

Les scénarios passent par un contexte `campagne(...)` dont le `finally` referme
la campagne quoi qu'il arrive. Et `test_j5` porte en plus une **précondition** qui
nomme la cause : « campagnes laissées ouvertes par un scénario précédent : […] »,
plutôt qu'un « 3 au lieu de 2 » qui accuse l'encart.

### Ce que les mesures donnent

| Configuration | Échecs sur 6 passages |
|---|---|
| ancien sélecteur, ancienne clôture (la CI) | 2 par passage rouge — le vrai et son ombre |
| ancien sélecteur, clôture en `finally` | **1** sur 6 — le vrai, seul |
| sélecteur corrigé, clôture en `finally` | **0** sur 6 |

La ligne du milieu est celle qui vaut : elle montre que la correction de clôture
suffit à faire disparaître l'échec en cascade, et donc qu'un test rouge cesse de
salir le suivant.

### Ce que ça coûte de ne pas l'avoir vu en local

Rien de grave ici — mais la leçon est que **quatre passages verts d'affilée ne
disent rien d'un test dont le verdict dépend d'un tirage**. Le sabotage volontaire
avait éprouvé les deux instruments du fichier ; il n'éprouvait pas leur stabilité.
Rejouer un fichier e2e quelques fois avant de le déclarer bon coûte une minute.
