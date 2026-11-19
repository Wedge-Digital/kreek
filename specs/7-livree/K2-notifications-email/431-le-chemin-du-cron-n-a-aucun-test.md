# Le chemin du cron n'a aucun test, et il porte trois notifications sur quatre

**Priorité : haute** — c'est le critère de clôture de l'épic qui n'est constaté
par rien
**Épic :** E02 — Notifications e-mail de compétition
**Trouvée par :** la revue de déploiement de la branche `demo`
**Fichiers :** `src/app/competitions/use_cases/send_due_notifications_use_case.rs`,
`tests/e2e/test_notification_cron.py` (à créer)

## Le problème

| Composant | Tests |
|---|---|
| `due_today()` — le domaine | 17 |
| `notification_dispatch` — l'expédition | 6 |
| `send_registration_open_use_case` | 1 unitaire + 1 e2e |
| **`send_due_notifications_use_case`** | **aucun** |

Les deux extrémités sont couvertes. **La couture entre elles ne l'est pas** —
environ 180 lignes : `candidates()`, `traiter_saison()`, `expedier()`,
`etiquettes()`, `places_restantes()`.

Ce que cette couture porte : `round_eve`, `round_closing` et
`registration_deadline`. Soit **trois des quatre notifications**.
`registration_open` est la seule à passer par le listener, et la seule vérifiée
de bout en bout (`test_notification_ouverture.py`).

## Le critère de l'épic, que rien ne constate

> Un coach inscrit à une compétition reçoit, la veille d'une journée, un e-mail
> listant ses matchs — sans que personne n'ait lancé de commande à la main, et
> sans qu'une seconde exécution du cron le même jour lui en envoie un second.

Aucun test ne l'observe. L'épic est en `en_cours` avec un critère qu'on ne
saurait pas déclarer atteint.

## Les modes d'échec que cette zone peut avoir en silence

Ce ne sont pas des hypothèses de revue : chacun est nommé par un commentaire du
code lui-même, ce qui veut dire qu'ils ont été vus et laissés sans filet.

**Les deux sources de date doivent s'accorder.** `candidates()` cherche une
journée à `today + n` par SQL, `due_today()` compare avec le même `n` tiré du
domaine. Le commentaire l'écrit :

> Les deux dates doivent sortir de la même source, sans quoi le cron ne trouve
> jamais rien — **sans la moindre erreur pour le signaler**.

Un décalage d'un jour dans `fenetres()` ne casse rien, ne journalise rien, et
n'envoie plus rien. C'est le pire des trois.

**Le dédoublonnage de trois requêtes qui se recouvrent.** Une saison dont une
journée démarre et une autre clôt le même jour sort des trois lectures ;
`candidates()` la réduit par `HashMap`. Rien ne vérifie qu'elle est traitée une
fois et non trois.

**Les étiquettes vides.** `etiquettes()` porte déjà cet avertissement :

> un `String::new()` ici rend « **** t'invite à participer », ce qui est arrivé
> entre la carte 340 et sa correction.

C'est arrivé une fois sur le chemin de l'ouverture, et un test l'y garde
désormais. Le chemin du cron construit ses étiquettes autrement — le nom de
l'administrateur passe par `find_base_info` du dépôt des compétitions — et n'a
pas son équivalent.

**`--dry-run` compte puis s'arrête.** Il incrémente `notifications_due` et
retourne avant `dispatch`. Une inversion de ces deux lignes ferait envoyer une
exécution censée ne rien faire — et c'est justement la commande qu'on lancera en
premier sur la production.

## Conception

### Deux tests, à deux hauteurs

**Unitaire, sur `execute()`** — le seul endroit où les modes d'échec ci-dessus
sont atteignables sans navigateur. Le patron existe : `notification_dispatch` a
six tests avec `#[sqlx::test]` et des doublures de port ; la seule pièce nouvelle
est un `SeasonRepository` réel alimenté par des `INSERT` de fixture.

Les cas qui valent la peine :

| Cas | Ce qu'il garde |
|---|---|
| Journée à `today + 1` | `round_eve` part, les deux autres non |
| Saison sortant des trois requêtes | traitée **une** fois |
| `--dry-run` | `notifications_due` compté, `sent` à zéro, journal vide |
| Réglage décoché | rien ne part, même la journée due |
| Deuxième exécution le même jour | `skipped_already_sent`, pas un second envoi |

**E2E, sur le critère de l'épic** — `test_notification_cron.py`, sur le modèle
exact de `test_notification_ouverture.py` : lire la **table du journal**, pas une
boîte de réception, la suite tournant en `EMAIL__PROVIDER=console`.

La différence avec l'ouverture : le cron ne se déclenche pas tout seul. Le test
doit invoquer la sous-commande. `--date` existe précisément pour viser un jour
choisi, ce qui évite d'avoir à créer une compétition dont une journée tombe
demain.

### Ce qu'on ne cherche pas à tester

Le cron **système**. Qu'une crontab appelle le binaire chaque nuit n'est pas
vérifiable en CI et ne relève pas du code. La commande est documentée
(`README.md:65`) ; c'est le geste d'exploitation qui l'installe.

## Checklist

- [x] Tests unitaires sur `execute()` — les cinq cas du tableau
- [x] `tests/e2e/test_notification_cron.py` — la veille d'une journée, la seconde
      exécution qui n'envoie rien, et le `--dry-run` qui ne réserve rien
- [x] `tests/impact-map.toml` mis à jour dans le même commit
- [x] L'épic E02 peut se clore sur son critère, constaté et non supposé
- [x] `make check-arch`, `make test`, e2e du fichier

## Ce que l'écriture a appris

**Les tests unitaires ne doublent que les deux ports inter-BC.** Saisons,
compétitions, journées et journal sont les **vrais** dépôts sur une vraie base :
c'est précisément la couture entre le SQL et `due_today()` qu'ils existent pour
tenir, et la doubler l'aurait dissoute.

**Le désaccord d'un jour a été falsifié.** En décalant `fenetres()` de `+1` sans
toucher à `veille()`, quatre des cinq tests tombent. C'était le mode d'échec que
le code nommait sans que rien ne le garde.

**Un mode d'échec de plus, découvert en écrivant.** `traiter_saison()` appelle
`find_by_season(…).unwrap_or_default()` : une erreur du dépôt de journées devient
« aucune journée », donc « rien à envoyer », **sans une ligne de journal**. Le
premier jet des tests s'y est fait prendre — un identifiant d'appariement à 27
caractères au lieu de 26 rendait la lecture en erreur, et le rapport annonçait
sereinement `notifications_due: 0`. Mérite sa carte : ce `unwrap_or_default()`
transforme une panne en silence.

**Ce que l'e2e ajoute aux unitaires.** Les doublures des deux ports sont
justement ce que la CLI câble pour de vrai. Un adapter mal branché dans
`src/cli/send_notifications.rs` passerait les cinq tests unitaires — et le
premier jet de l'e2e l'a montré en creux : sans équipe inscrite, le journal
restait vide, parce que la veille de journée ne s'adresse qu'aux inscrits.

## Ce que la carte ne couvre pas

**La vérification en client de messagerie réel** — c'est la 338, qui reste
ouverte pour ça et rien d'autre.
