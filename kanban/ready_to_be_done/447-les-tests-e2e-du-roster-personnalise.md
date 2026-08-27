# Les tests E2E du roster personnalisé

**Épic :** E10 · **Ordre :** 5 · **Dépend de :** 446
**Conception :** `docs/specs/roster-personnalise/editeur-de-roster/07-integration.md`

## Objectif

Prouver dans un navigateur la chaîne que huit couches traversent, et qu'aucun
test unitaire ne voit d'un bout à l'autre.

Fichier : `tests/e2e/test_custom_roster.py`.

## Les scénarios

| Test | Ce qu'il prouve |
|---|---|
| `test_creer_un_roster_minimal` | le chemin heureux |
| `test_le_roster_cree_apparait_a_la_creation_d_equipe` | **le test qui compte** |
| `test_un_roster_sans_journalier_est_refuse` | S2, bout en bout |
| `test_un_roster_utilise_n_est_pas_modifiable` | le verrou, et le bandeau qui dit la cause |
| `test_le_bouton_supprimer_n_existe_pas_sur_un_roster_utilise` | absent, pas grisé |
| `test_un_non_admin_ne_voit_pas_les_actions` | P1 |
| `test_le_roster_d_un_autre_espace_rend_404` | P2, par le résolveur |
| `test_supprimer_un_roster_le_retire_des_tiers` | la chaîne d'événements |

## Celui qui vaut le prix de la suite

**`test_le_roster_cree_apparait_a_la_creation_d_equipe`.**

Il traverse tout : l'écriture en base, le rafraîchissement du cache,
l'aiguillage par préfixe dans `find_team_by_uid`, le port de `team_creation`, et
le sélecteur de rosters.

Et c'est **le seul qui prouve que le cache n'est pas gelé** — sans redémarrage
entre la création et la vérification. C'est exactement le défaut de la carte 362
sur le bundle CSS, et celui qui a coûté une heure de diagnostic sur un roster
Slann en production.

## Le test asynchrone

**`test_supprimer_un_roster_le_retire_des_tiers`** attend que le listener soit
passé.

**Pas de `sleep`.** Une condition sur l'état observable — le roster absent du
sélecteur de la compétition — comme `cliquer_quand_cable` le fait pour htmx. Une
durée fixe n'a aucune marge sur une machine chargée, et c'est exactement là que
la suite échouait.

## Le piège de la fenêtre non câblée

Les trois sélecteurs de l'éditeur s'ouvrent par du JS, mais les panneaux de la
liste arrivent par navigation. Tout clic sur du contenu fraîchement injecté
passe par `cliquer_quand_cable` (`tests/e2e/htmx_helpers.py`).

## Ce que les tests ne couvrent pas

- **`UsageUnavailable`** ne se provoque pas depuis un navigateur : il demande un
  port en panne. La carte 443 le couvre unitairement.
- **Le chargement au démarrage** des rosters de la base : il demande un
  redémarrage au milieu du test. La carte 441 le couvre.

## Checklist

- [ ] Les huit scénarios
- [ ] `cliquer_quand_cable` sur le contenu injecté
- [ ] Aucun `sleep`, y compris dans le test asynchrone
- [ ] `make e2e` vert, serveur de développement lancé par l'utilisateur
