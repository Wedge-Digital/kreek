# Les tests E2E des points de classement manuels

**Ordre :** 4 · **Dépend de :** 452
**Conception :** `docs/specs/points-classement-manuels/page-de-gestion/07-integration.md`

## Objectif

Prouver dans un navigateur la chaîne qui va de l'attribution au rang affiché.

Fichier : `tests/e2e/test_manual_ranking_points.py`.

## Les scénarios

| Test | Ce qu'il prouve |
|---|---|
| `test_attribuer_des_points_a_une_equipe` | le chemin heureux |
| `test_le_classement_affiche_les_points_manuels` | la colonne, dans les deux vues |
| `test_le_classement_est_reordonne_sans_rechargement_du_serveur` | **le test qui compte** |
| `test_une_penalite_fait_descendre_l_equipe` | le sens négatif, bout en bout |
| `test_supprimer_une_ligne_la_retire_du_classement` | le retour en arrière |
| `test_un_non_admin_voit_la_page_sans_les_actions` | public en lecture, réservé en écriture |
| `test_la_liste_se_recharge_apres_attribution` | `HX-Trigger` |

## Celui qui vaut le prix de la suite

**`test_le_classement_est_reordonne_sans_rechargement_du_serveur`.**

Il attribue assez de points pour changer l'ordre de deux équipes, recharge la
page de classement, et vérifie que les rangs ont suivi — **sans redémarrage du
serveur**.

C'est lui qui prouve que l'architecture est juste : le classement n'étant stocké
nulle part, `build_ordered_standings` recalcule l'ordre à chaque affichage, et
aucune propagation n'est due. Si ce test échouait, c'est toute la phase 3 qui
serait fausse.

## La non-régression, qui compte autant

**Les trois suites existantes doivent rester vertes sans une modification** :

```
tests/e2e/test_detailed_standings.py
tests/e2e/test_ranking_bonus.py
tests/e2e/test_ranking_tiebreak.py
```

Elles n'attribuent aucun point manuel. Elles mesurent donc que le classement
ordinaire s'ordonne **exactement comme avant** — ce qui est la vraie garantie,
puisque la fonctionnalité touche `compare`, la fonction qui ordonne tous les
classements de l'application.

Si l'une d'elles doit être modifiée pour passer, c'est le signe que la carte 449
a changé un comportement qu'elle ne devait pas toucher.

## Le piège de la fenêtre non câblée

La liste arrive par `hx-get` et se recharge sur `HX-Trigger`. Tout clic sur du
contenu fraîchement injecté — déplier une équipe, supprimer une ligne — passe
par `cliquer_quand_cable` (`tests/e2e/htmx_helpers.py`).

**Pas de `sleep`.** Une durée fixe n'a aucune marge sur une machine chargée, et
c'est exactement là que la suite échouait.

## Ce que les tests ne couvrent pas

- **Les bornes des value objects** — zéro, ±100 : la carte 449 les couvre
  unitairement, et les provoquer depuis un navigateur demanderait de contourner
  le formulaire.
- **`TeamNotEnrolled`** : il demande de poster un identifiant d'équipe que
  l'écran ne propose pas. Carte 450.

## Checklist

- [ ] Les sept scénarios
- [ ] `cliquer_quand_cable` sur le contenu injecté
- [ ] Aucun `sleep`
- [ ] Les trois suites existantes vertes **sans modification**
- [ ] `make e2e`, serveur de développement lancé par l'utilisateur
