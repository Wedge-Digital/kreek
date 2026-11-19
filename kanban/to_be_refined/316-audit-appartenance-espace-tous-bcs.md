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

## Méthode retenue — inventaire renversé

L'inventaire exhaustif des 146 routes a été **écarté** : quatre BCs sur sept
n'ont aucune mention de l'espace d'une ressource dans leur couche web, donc
146 lignes auraient dit la même chose. Sondage de représentants, verdict, puis
conception du correctif.

### Méthode d'origine

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


---

# RÉSULTAT DE L'AUDIT

## Verdict : systémique, et prouvé en écriture

Sondes contre le serveur de développement, depuis un espace dont l'appelant est
admin, vers des ressources d'un **autre** espace :

| BC | Sonde | Résultat |
|---|---|---|
| `competitions` | lecture de « Ligue Open » (Bordeaux) via l'espace E2E | **200, contenu réel servi** |
| `teams` | lecture de « Les gros bourrins » | **200, contenu réel servi** |
| `news` | lecture d'un article d'un autre espace | **200, titre réel servi** |
| `news` | **commentaire posté** sur cet article | **200, ligne écrite en base** |
| `players` | *(carte 315)* | corrigé |

L'écriture croisée est démontrée **deux fois** — `players` avant correctif, et
`news`. Ce n'est pas une fuite d'information théorique.

Le commentaire de sonde a été supprimé, base rendue à son état initial.

### Une sonde non concluante, qui ne compte pas comme preuve

L'ajout au panier de recrutement d'une équipe étrangère a rendu `422` — mais
l'équipe était en `ReadyToPlay`, donc c'est vraisemblablement le garde de phase
qui a parlé, pas l'autorisation. **`teams` n'est prouvé qu'en lecture.**

## Conception retenue : un middleware commun, un résolveur par BC

**Corrigé en préparant la carte 318.** Cette section affirmait d'abord qu'un
mécanisme partagé imposerait de toucher 146 signatures de handler. **C'était
faux** : un middleware n'en touche aucune — il lit les paramètres du chemin dans
la requête, comme le fait déjà
`spaces/io/web/extractors/space_permissions.rs`.

Le découpage « un garde par BC » avait été tranché sur cette erreur. La forme
retenue est finalement un **middleware commun** (carte 324) alimenté par un
`ISpaceOwnership` par BC : chaque BC répond sur ses propres ressources, via son
propre repository, ce qui préserve la souveraineté des données.

Ce qui suit décrit l'ancienne forme, conservé pour mémoire — les deux familles
de ressources, elles, restent exactes et alimentent les résolveurs.

Chaque BC reçoit `io/web/space_scope.rs`, sur le modèle de la carte 315 :

```rust
pub async fn charger_<ressource>_de_l_espace(
    state: &AppState, space_id: &str, id: &str,
) -> Result<Ressource, Response>
```

`404` et non `403` : rien ne doit confirmer l'existence d'une ressource d'un
autre espace à qui l'énumère.

**La règle qui fait la solidité** : le garde devient le *seul* moyen d'obtenir
la ressource depuis la couche web du BC. C'est ce qui a permis, en 315, de
vérifier qu'aucun chemin ne le contourne.

### Deux familles, imposées par les données

**Comparaison directe** — la ressource porte son espace :

`competitions` · `team_proj` · `articles` · `match_report_proj` ·
`team_drafts` · `team_roster_selections`

**Un saut** — la ressource hérite de son parent :

| Table | Remonte par |
|---|---|
| `competition_seasons` | `competition_id` → `competitions.space_id` |
| `comments` | `article_id` → `articles.space_id` |
| `ranking_lines` | `competition_id` → `competitions.space_id` |

**Le saut est préféré à l'ajout d'une colonne `space_id`.** Une saison n'a pas
d'espace en propre : elle en hérite. Dénormaliser créerait une seconde source de
vérité qui finirait par diverger — la carte 313 vient de rappeler ce que ça
coûte.

## Cartes de migration, par exposition décroissante

| Carte | BC | Routes | Motif du rang |
|---|---|---|---|
| 318 | `competitions` + `ranking` | 44 + 2 | fuite prouvée, données de jeu, plus gros volume |
| 319 | `match_report` | 23 | aucune vérification, données de jeu |
| 320 | `teams` | 29 | fuite prouvée en lecture |
| 321 | `team_creation` | 25 | — |
| 322 | `news` | 5 | écriture prouvée, mais un commentaire |
| 323 | axe `check-arch` | — | **après** la première migration |

`news` en dernier malgré la preuve d'écriture : c'est le seul dont l'abus ne
touche pas les données de jeu.

L'axe `check-arch` vient après la première migration et non avant : tant que
six BCs sont en infraction, il naîtrait avec une baseline de six exceptions — et
une baseline de cette taille ne se résorbe jamais.

## Cette carte est close

Son livrable était un verdict et un découpage. Les deux sont ci-dessus. Le
travail passe aux cartes 318 à 323.
