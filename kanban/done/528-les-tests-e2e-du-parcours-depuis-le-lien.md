# Les tests e2e du parcours depuis le lien

**Priorité : haute — c'est le parcours que personne ne verra avant un coach**
**Épic :** E16 — Sondage de présence
**Dépend de :** 525, 527
**Fichiers :** `tests/e2e/test_presence_reponse_publique.py`, `tests/impact-map.toml`

## L'objectif

Le chemin complet, vu du navigateur, depuis le lien jusqu'à l'onglet de
l'organisateur.

Le test ne peut pas partir d'un clic dans une boîte mail : il **lit le jeton en
base** par `db_helpers.py`, comme les autres tests qui ont besoin d'un état que
l'écran ne montre pas.

## Les scénarios

| Scénario | Ce qu'il éprouve |
|---|---|
| ouvrir une campagne, lire un jeton, visiter `/oui` | la page confirme, et l'onglet de l'organisateur voit la présence |
| cliquer le bouton opposé | R4 — le même jeton, l'autre verbe, la réponse bascule |
| visiter `/oui` deux fois | idempotent : un préfetch d'antivirus ne casse rien |
| clore la campagne, revisiter | R27 — motif « échéance » ou « décision », et la dernière réponse est affichée |
| publier un rapport, revisiter | R27 — motif « journée déjà jouée », **alors que l'échéance est à venir** |
| visiter un jeton inventé | R26 — la même page qu'un jeton révoqué, sans un mot de plus |
| visiter sans session, en navigation privée | le routeur public, vu du navigateur |

Le dernier **double le test unitaire du routeur** de la carte 525, et ce n'est
pas de la redondance : l'un vérifie que la route n'est pas sous `require_auth`,
l'autre qu'un navigateur sans cookie voit bien la page. Les deux ont échoué
séparément ailleurs dans ce projet.

L'avant-dernier est celui qui vaut le plus : il éprouve le cas où la campagne est
**ouverte** et la journée pourtant figée — celui qui a fait écrire R27, et que
personne ne rencontre en développement parce qu'il demande un rapport publié
avant l'échéance.

## Pas de `cliquer_quand_cable` ici

La page est un rendu serveur complet, sans HTMX : la fenêtre où un élément est
peint mais pas encore câblé ne s'y présente pas. C'est la seule page du projet
dans ce cas, et le noter évite qu'on ajoute l'attente par habitude.

## Ce qui a été écrit, et les écarts au tableau ci-dessus

**Neuf tests, quatre journées.** Le tableau annonçait sept scénarios ; deux se
sont dédoublés.

*Les deux motifs de clôture au lieu de « échéance **ou** décision ».* Ce sont
deux branches distinctes de `libelle_du_motif`, atteintes par deux mécanismes
différents : la décision par l'action `close`, l'échéance par un `execute_db` qui
recule la date — `ouvrir` et `rouvrir` refusent tous deux une échéance passée,
aucun parcours utilisateur n'y mène. Et c'est l'échéance que le coach rencontre en
vrai, la plupart des campagnes se fermant d'elles-mêmes.

*Le jeton inconnu et le jeton difforme, paramétrés sur le même test.* R26 veut la
**même** page pour les deux, et l'écrire en un seul test avec deux entrées est ce
qui dit que c'est la même page — deux tests séparés auraient laissé la porte à
deux rendus.

**L'assertion qui n'était pas au tableau et qui compte le plus** : après la visite
du lien, l'onglet de l'organisateur montre l'équipe en présents **sans** badge
« saisi par vous ». R6 réserve ce badge à sa saisie ; le chemin du jeton doit
laisser `saisi_par_admin` à `NULL`. Rien d'autre ne le vérifiait, et c'est la seule
chose qui distingue « le coach a dit qu'il venait » de « on a dit pour lui » quand
une rencontre est contestée.

**J4 fait un vrai tirage, et l'ordre de ses six étapes est contraint.**
`etat_de_la_journee` calcule `figee` depuis les **appariements** de la journée qui
portent un rapport publié : un rapport posé sur une journée sans appariement
n'aurait peut-être rien figé, et le test aurait été vert sans éprouver la règle.
D'où présents → clore → tirer → valider → **rouvrir** → publier ; `rouvrir` refuse
une journée figée, donc la réouverture précède la publication.

## Ce que la carte annonçait et qui n'existe pas

> clore la campagne, revisiter | R27 — motif « échéance » ou « décision », **et la
> dernière réponse est affichée**

`PresenceClosedTemplate` ne porte pas la réponse : ses champs sont `css`,
`team_name`, `round_name`, `competition_name` et `motif`. Le récapitulatif dit
l'équipe, la journée et la compétition — jamais ce qui a été enregistré.

**Et R27 ne le demande pas** : la règle porte sur les trois causes de fermeture,
pas sur le rappel de la réponse. C'est donc une supposition de cette carte, pas un
manque par rapport à la spec. Le test a été écrit sur le motif et le
récapitulatif, sans inventer l'assertion manquante.

Reste que le geste est naturel : un coach qui revisite son lien après la clôture
veut savoir ce qui est noté, et n'a aujourd'hui que le téléphone de
l'organisateur pour l'apprendre. À décider séparément — ce n'est pas un défaut de
la 525, c'est une fonction qui n'a jamais été demandée.

## Le contrôle de crédibilité

Neuf tests verts en cinq secondes, sur une suite dont la moyenne est d'une seconde
et demie par test : la rapidité seule ne prouve rien, et un test qui n'éprouve
rien passe aussi vite. Une assertion a donc été **volontairement faussée** puis
relancée, et l'échec attendu est bien tombé — la page rend « C'est noté, tu
viens », le locator la trouve, et le sabotage a été retiré. Sans cette vérification,
« 9 passed » n'aurait été qu'une affirmation.

## Checklist

- [x] Le fichier de test, la lecture du jeton par `db_helpers.py`
- [x] Les sept scénarios, en neuf tests
- [x] `tests/impact-map.toml` : entrée pour le nouveau fichier, **même commit**,
      avec `match_report` en dépendance dure — c'est le seul chemin vers R27
- [x] Une assertion faussée exprès pour vérifier que la suite sait échouer
- [x] `make e2e` passe (serveur dev lancé par l'utilisateur)
- [x] `make lint`, `make check-arch`, `make test`
