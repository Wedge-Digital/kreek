# Les tests e2e de l'onglet Présences

**Priorité : haute — le rendu HTMX ne se vérifie pas autrement**
**Épic :** E16 — Sondage de présence
**Dépend de :** 521
**Fichiers :** `tests/e2e/test_competition_presences.py`, `tests/impact-map.toml`

## Cette carte ne déclare aucune route — et le vérifie

Les douze routes de l'onglet ont été posées par les cartes qui les servent : trois
en 519, une remplie en 520, neuf en 521. Cette carte est la première à les
**traverser toutes** dans un navigateur, donc la première à constater qu'aucune
ne manque ni ne répond à côté.

Son entrée dans `tests/impact-map.toml` va dans le même commit que le test :
l'axe 8 de `check-arch` refuse un test e2e sans entrée, et un test sans entrée est
traité comme `"all"` — donc silencieusement toujours exécuté.

## L'objectif

Le parcours complet dans un navigateur, contre le serveur dev. Aucun test
unitaire ne voit ce que cette carte couvre : le bug du widget coach-search et
celui des pickers de tiers n'ont été trouvés que là.

## Les scénarios

| Scénario | Ce qu'il éprouve |
|---|---|
| lancer une campagne, voir les trois colonnes | R3 — une équipe sans adresse est bien dans « sans réponse » |
| poser une présence à la main, voir le badge | R6 — « saisi par vous » survit au rechargement |
| clore, puis poser encore une réponse | R21 — la clôture ferme le coach, pas l'organisateur |
| tirer avec un seul présent | R15 — le motif s'affiche, le bouton reste inactif |
| tirer, valider, ouvrir le Calendrier | les rencontres y sont, avec leurs projections |
| passer un présent à absent après le tirage | R12 — le panneau de défection propose, il n'agit pas |
| valider la réparation | la rencontre change, **les autres ne bougent pas** |
| vider la journée au Calendrier, revenir en Présences | R24 — l'onglet dit « clos », pas « appariée » |

## Le piège à ne pas rouvrir

**`cliquer_quand_cable` sur tout ce qui vient d'être injecté**
(`tests/e2e/htmx_helpers.py`). Le panneau est remplacé à chaque action : c'est
exactement la fenêtre où un bouton est peint, visible, et **inerte**. Mesuré :
six éléments non câblés à `t=0`, zéro à `t=50ms`.

**Pas de `sleep`.** Une durée fixe n'a aucune marge sur une machine chargée — et
c'est là que la suite échoue — tout en coûtant son délai aux milliers d'appels où
tout est déjà prêt.

## La carte d'impact

`tests/impact-map.toml` est mis à jour **dans le même commit** que le test. Une
carte tests↔BC incomplète fait sauter en silence le test qu'on vient d'écrire —
c'est ce que la carte 480 a trouvé, cinq tests e2e sans entrée que ni le local ni
la CI ne voyaient.

## Checklist

- [x] Le fichier de test, cinq journées allouées selon la dépendance réelle
- [x] Les huit scénarios — 8/8 en 6,5 s
- [x] `cliquer_quand_cable` partout, aucun `sleep`
- [x] `tests/impact-map.toml` : entrée dans le même commit
- [x] `make lint`, `make check-arch`, `make test` — 1878/1878

## Les trois défauts que cette carte a trouvés

### 1. `json-enc` ne transporte pas un tableau depuis `hx-vals`

Symptôme : les rencontres n'étaient pas écrites après un clic sur « Valider » — le
genre de piste qui envoie chercher dans le use case.

**C'est la surveillance de la console par le harnais e2e qui a donné la cause**,
pas une assertion : `TypeError: Converting circular structure to JSON`. Sans elle,
la recherche serait partie côté serveur.

La mécanique, lue dans l'extension et non devinée. htmx aplatit `hx-vals` dans un
`FormData`, où un tableau devient N entrées de même clé. `json-enc` les recompose
ainsi :

```js
if (Array.isArray(object[cleanKey])) object[cleanKey].push(typedValue)
```

`object[cleanKey]` vaut déjà le tableau entier — `getExpressionVars` rend la valeur
typée d'origine — et `typedValue` aussi : **le tableau se pousse dans lui-même**.
D'où « index 2 closes the circle » avec deux couples.

**Le premier correctif était faux** : croyant la construction en JavaScript
fautive, la liste a d'abord été sérialisée côté serveur — et l'erreur a persisté à
l'identique. La cause n'était pas la construction, c'était le tableau.

Contournement retenu : les listes voyagent en **chaîne JSON dans un champ
scalaire**, désérialisées par le handler. Et `serde_json::to_string` sur la chaîne
produit le littéral échappé qu'un attribut HTML exige — l'échapper à la main se
serait cassé sur le premier nom d'équipe portant une apostrophe. Documenté dans
`presences_actions.rs` : le prochain qui voudra passer une liste tombera dessus.

### 2. `etat_du_panneau` lisait l'appariement sur la campagne — R24

```rust
if matches!(survey.appariement(), Appariement::Fait { .. }) { … }
```

Le Calendrier vide une journée sans rien savoir de la campagne : `Appariement::Fait`
y restait **indéfiniment**, et le panneau annonçait « Journée appariée » sur une
journée vide.

**C'est le défaut que R24 a été écrite pour empêcher**, et le commit de la carte 520
affirmait « l'appariement se lit sur la journée, jamais sur la campagne » : vrai de
`desaccord`, faux d'`etat_du_panneau`. La règle était comprise et à moitié
appliquée.

**Aucun test unitaire ne pouvait le voir** : le vidage passe par l'autre onglet, et
seul un e2e traverse les deux. C'est l'argument même de la règle de couverture
obligatoire.

### 3. Un test unitaire qui fixait ce défaut comme règle

En corrigeant, `une_journee_appariee_donne_appariee` s'est mis à échouer : il
passait une **journée vide** en attendant « appariée ». Il décrivait le
comportement observé au lieu de la règle voulue, et c'est pour ça qu'il n'a jamais
rien signalé.

Il a fallu deux corrections avant qu'il dise vrai. Avec des rencontres mais des
équipes silencieuses, il rendait `Defection` — et c'était **juste** : une rencontre
dont les camps n'ont pas confirmé leur présence *est* un désaccord. Il faut donc
deux équipes présentes **et** appariées.

**La leçon, écrite dans sa docstring** : un test rédigé d'après ce que le code fait
protège le défaut au lieu du contrat.

## Deux erreurs de rédaction, corrigées en lançant le fichier seul

`competition_pairings` n'existe pas — la table s'appelle
`competition_match_day_pairings`. Le nom avait été inventé au lieu d'être lu.

`.team-card:first-child` ne matche rien : le premier enfant d'une colonne est son
en-tête. Et `cliquer_quand_cable` prend déjà `.first`, donc la précision était
inutile autant que fausse.

Et une assertion trop grossière : `not_to_contain_text("01")` pour vérifier qu'un
ULID ne s'affiche pas échouait sur un nom **juste** — les noms de la fixture
portent un horodatage. Remplacée par une expression sur la **forme** d'un ULID nu.

## Ce que les huit scénarios couvrent

| Journée | Vérifié à l'écran |
|---|---|
| J1 | lancement → quatre équipes en « sans réponse » (R3) · le badge « saisi par vous » survit au rechargement (R6) |
| J2 | clore n'arrête pas l'organisateur (R21) |
| J3 | un seul présent → bouton inactif **et son motif** (R15) |
| J4 | tirer → valider → les rencontres sont en base · défection → le panneau propose sans rien écrire (R12) · réparer → une rencontre change, les autres non |
| J5 | vider au Calendrier → l'onglet repasse à « clos » (R24) |

Huit des dix actions de la carte 521 sont désormais traversées dans un navigateur.
Les deux restantes : `remind`, dont l'expédition est provisoire et n'offre rien à
observer, et `undo-draw`, que la réparation couvre partiellement.

**Le fichier tient en 6,5 secondes**, contre les une à deux minutes estimées au
plan : les cinq journées allouées selon la dépendance réelle évitent de relancer
huit campagnes.
