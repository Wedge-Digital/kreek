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

## Checklist

- [ ] Le fichier de test, la lecture du jeton par `db_helpers.py`
- [ ] Les sept scénarios
- [ ] `tests/impact-map.toml` : entrée pour le nouveau fichier, **même commit**
- [ ] `make e2e` passe (serveur dev lancé par l'utilisateur)
- [ ] `make lint`, `make check-arch`, `make test`
