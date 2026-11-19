# E15 — Recruter un journalier

**État :** `done` — six cartes sur six, closes le 2026-09-06.
**Conception :** `docs/specs/embaucher-un-journalier/` — quinze décisions issues
du grilling, à lire avant les cartes.
**Maquette :** `assets/rawpages/html/app-team-recruitment.html`

## La fonction

Une équipe incomplète aligne des journaliers pour jouer son match. Aujourd'hui
ils disparaissent avec le rapport : ce qu'ils ont fait sur le terrain ne laisse
aucune trace, et le coach qui aurait voulu garder celui qui a marqué trois fois
n'a aucun moyen de le faire.

Cette épic ouvre ce geste : **un journalier qui a joué peut être recruté
définitivement à la phase de recrutement suivante**, avec l'expérience et les
améliorations gagnées pendant le match. Celui qu'on ne recrute pas est perdu.

## Le renversement qui commande tout

**Un journalier est un joueur dès le début du rapport de match**, pas à son
recrutement. Il naît dans `players` avec un `membership: Journeyman`, joue, gagne
des SPP et prend ses améliorations comme les autres ; le recrutement ne fait que
basculer son `membership` en `Active`.

Sans ce renversement, les hausses de valeur du règlement seraient impossibles à
porter — **un joueur qui n'existe pas ne peut pas s'améliorer**. C'est ce qui
explique l'ordre des cartes, et pourquoi la première ne montre aucun écran.

## Les cartes

| Carte | | Dépend de |
|---|---|---|
| `454` | Un journalier est un joueur — le troisième statut d'appartenance | — |
| `455` | Le journalier naît avec le rapport | 454 |
| `456` | Le journalier disparaît — celui qu'on ne recrute pas quitte l'effectif | 455 |
| `457` | Le panier accueille un journalier — domaine, agrégat, limites | 454 |
| `458` | L'écran affiche les journaliers recrutables | 456, 457 |
| `459` | Les tests e2e du recrutement de journalier | 458 |

## Ce qui commande l'ordre

**La 454 est seule en tête, et c'est la plus risquée.** Elle ouvre un troisième
`membership` et fait cesser **cinq lectures d'exclure les journaliers** — sans
un écran ni un événement. Une carte qui ne montre rien et touche à ce que tout le
monde lit : c'est là que le socle se casse ou tient.

**La 455 et la 457 partent ensuite en parallèle** : l'une fait naître le
journalier avec le rapport, l'autre construit le domaine du recrutement. Elles ne
se croisent pas.

**La 456 ferme la boucle avant l'écran** : sans elle, un journalier non recruté
resterait dans l'effectif, et le panneau de la 458 proposerait de recruter des
joueurs qui n'auraient jamais dû survivre.

**La 459 est à part** parce qu'elle prouve une chaîne qui traverse **trois BCs et
deux bus d'événements** — ce qu'aucun test unitaire ne voit d'un bout à l'autre.

## Ce que l'épic ne couvre pas

**Les journaliers du rapport de match**, qui existent déjà : `init_temp_players`,
le sélecteur de joueurs temporaires, et le décompte de l'étape 2 corrigé par la
carte `495`. Cette épic ne les remplace pas — elle leur donne une suite.

**Les étoiles et les mercenaires**, qui relèvent d'un autre geste de recrutement.

**La valeur d'équipe pendant le match** : ce qu'un journalier y pèse est tranché
ailleurs (`basket_hydration_service.rs` porte déjà le commentaire qui l'annonce).

## Ce que l'épic a appris

### Le compilateur a tenu trois verrous que les `grep` ne voyaient pas

`guard_active` (454), `treasury_movement` (455) et les sept `match` ouverts par
`SquadEngagement` (457) ont chacun **posé une question de conception au moment
d'ajouter une variante**. Aucun des deux `grep` de contrôle de la 454 ne les
aurait attrapés : ils ne s'écrivent ni `membership = 'Active'` ni
`is_active()`.

C'est un argument pratique, éprouvé trois fois : partout où une décision se
prend sur une appartenance ou un événement, un `match` exhaustif vaut mieux
qu'un prédicat.

### Une carte de socle ne peut pas prouver ce qu'elle rend possible

Trois fois d'affilée, un test a dû être légué à la carte suivante — la `454` à
la `455`, la `455` à la `459`, la `456` à la `457` par inversion d'ordre. Ce
n'est pas un défaut d'exécution mais une propriété du découpage : une carte qui
ouvre un statut sans rien qui le produise ne peut pas l'observer.

**Ce qui compte est que la dette soit nommée et rattachée**, pas qu'elle
n'existe pas.

### Deux maillons n'ont jamais été branchés, et rien ne le disait

La carte `459` a trouvé qu'`init_temp_players_use_case` n'émettait pas sur le
bus (`455`), et que `JourneymanRecruited` ne sortait pas du BC (`457`). Dans les
deux cas le code compilait, tous les tests unitaires passaient, et la
fonctionnalité ne marchait pas du tout.

**L'axe 12 de `check-arch` ne voit ni l'un ni l'autre** : il vérifie qu'une
émission passe par `emettre()` ou `publier()`, jamais qu'un événement destiné à
sortir est bien émis, ni qu'un bras de publisher est atteignable. C'est le
piège que l'épic E11 a documenté trois fois — du code qui a l'air branché et ne
l'est pas — et il mérite sa propre carte.

### Le découpage avait une case vide

Personne ne portait la bascule du journalier en `Active` : la `456` fait
disparaître, la `457` tient le domaine, la `458` l'écran. L'épic l'énonçait en
une phrase que trois cartes ont lue sans se l'attribuer.

## Terminé quand

Un coach dont l'équipe a joué avec un journalier ouvre sa phase de recrutement,
**voit ce journalier avec les SPP qu'il a gagnés au match**, le recrute, et le
retrouve dans son effectif au match suivant — avec son expérience.

Et celui qu'il n'a pas recruté n'apparaît plus nulle part.


## Constaté le 2026-09-06

`test_journeyman_recruitment.py`, sept scénarios verts. Un coach dont l'équipe
a joué avec un journalier ouvre sa phase de recrutement, le voit dans un
panneau rendu par `players`, le recrute, et le retrouve dans son effectif —
avec son maillot et sa valeur. Celui qu'il ne recrute pas n'apparaît plus nulle
part.

Suite complète : 1703 tests Rust, 363 e2e.
