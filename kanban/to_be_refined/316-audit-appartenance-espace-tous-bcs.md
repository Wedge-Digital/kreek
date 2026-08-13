# Audit — l'appartenance à l'espace est-elle vérifiée dans les autres BCs ?

**Priorité : à trancher après l'audit**
**Contexte :** transverse — les huit BCs exposant `/app/{space_id}/…`

## Point de départ

La carte 315 corrige un défaut **prouvé** dans `players` : le `space_id` du
chemin sert à autoriser, sans qu'on vérifie jamais que la ressource visée
appartient à cet espace. Un admin d'un espace quelconque agissait sur des
joueurs d'un autre.

**Le même patron est possible partout ailleurs. Il n'a pas été mesuré.**

Cette carte est un **inventaire**, pas une correction. Son livrable est une
liste de cas avérés, chacun avec sa preuve — après quoi on décidera du
découpage des corrections.

## Périmètre

Huit BCs exposent des routes `/app/{space_id}/…`, pour 146 déclarations au
total :

`ranking` · `team_creation` · `match_report` · `players` (fait, carte 315) ·
`spaces` · `competitions` · `news` · `teams`

## Ce qu'on sait déjà, sans l'avoir testé

`spaces` expose un extracteur `SpacePermissions`
(`io/web/extractors/space_permissions.rs`) qui résout le rôle de l'appelant
depuis le `space_id` du chemin. **Il souffre du même angle mort** : il vérifie
l'appartenance de l'**utilisateur** à l'espace, jamais celle de la **ressource**
manipulée.

`competitions` l'utilise (`latest_results_view`, `resultats_view`), en lecture.

Ce n'est pas une accusation : un extracteur qui résout un rôle fait son travail.
C'est le fait qu'il ne dise rien de la ressource qui doit être su.

## Méthode

Pour chaque BC, et pour chaque route portant `{space_id}` **et** un
identifiant de ressource :

1. La ressource porte-t-elle son propre `space_id` ?
2. Est-il comparé à celui du chemin, où que ce soit sur le chemin d'exécution ?
3. Si non : **le prouver** par une requête contre le serveur de développement,
   comme la carte 315 l'a fait — un grep ne suffit pas, une vérification peut
   vivre trois appels plus loin.
4. Classer : **écriture** (grave) ou **lecture** (fuite d'information).

Ne rien corriger pendant l'audit. Un correctif glissé dans un inventaire fausse
l'inventaire.

## Livrable

Un tableau BC × route × verdict × preuve, puis une proposition de découpage.

Si le défaut est général, la bonne réponse est probablement un **extracteur
partagé** — « cette ressource est-elle dans cet espace ? » — plutôt que huit
correctifs indépendants qui divergeront. Mais c'est une conclusion à tirer de
l'audit, pas une hypothèse à y projeter.

## Question ouverte

Faut-il un garde-fou dans `check-arch` ? Un axe qui repère une route portant
`{space_id}` et un identifiant de ressource, sans comparaison d'appartenance sur
le chemin, dirait la même chose que cet audit — mais en continu.

L'écrire correctement est un travail en soi, et un axe qui produirait des faux
positifs serait désactivé dans le mois. À évaluer une fois l'étendue connue.
