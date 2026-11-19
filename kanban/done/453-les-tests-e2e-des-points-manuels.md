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

- [x] Les sept scénarios — **huit**
- [x] `cliquer_quand_cable` sur le contenu injecté, **et sur lui seul**
- [x] Aucun `sleep`
- [~] Les trois suites existantes vertes — **deux sans modification**, cf. ci-dessous
- [x] `make e2e`, serveur de développement lancé par l'utilisateur

## Une suite a bien été modifiée, et par quoi

La carte exige les trois suites vertes « sans une modification », en posant que
toute retouche signalerait que **la carte 449 a changé un comportement qu'elle ne
devait pas toucher**.

`test_detailed_standings.py` a été modifié — mais par la carte **451**, et sur
`FIXED_COLUMNS`, le nombre de colonnes qui précèdent le bloc des départages. La
451 ajoute une colonne : c'est un changement voulu et annoncé, et la constante
devait le suivre. Rien de l'**ordre** n'a bougé.

`test_ranking_bonus.py` et `test_ranking_tiebreak.py` sont **intacts** depuis
`1753430` et passent. C'est là qu'est la garantie que la carte cherchait : ce
sont eux qui mesurent l'ordre, et ils ne mesurent que lui.

## Un huitième test, pour une limite mesurée

`test_le_total_affiche_est_celui_qui_ordonne`. La carte 451 avait mesuré que
faire afficher `row.points` au lieu de `row.total` laissait **1503 tests
unitaires verts**, alors que le classement aurait contredit son propre ordre.

Vérifié ici par mutation : sous le même changement, **1520 unitaires restent
verts** et ce test rougit sur « le total affiché contredit l'ordre ». C'est la
seule chose qui relie le rang au nombre affiché.

## Deux pièges rencontrés dans les tests eux-mêmes

### `cliquer_quand_cable` sur un élément Alpine

La ligne de groupe de l'accordéon porte un `@click` Alpine, pas d'attribut htmx :
`cliquer_quand_cable` y attendait un câblage qui n'arrive jamais, et expirait au
bout de dix secondes. Le ✕, lui, porte `hx-delete` — c'est là que la fenêtre non
câblée existe pour de bon, et c'est là seulement que le helper sert.

**Le helper ne s'applique pas à tout contenu injecté**, seulement à ce qu'htmx
câble. Un élément Alpine se clique directement.

### Une attente satisfaite par des options périmées

`kreek-select` journalisait « Failed to fetch » : la navigation abandonnait sa
requête en vol, et `console_errors` le comptait comme un échec — à juste titre.

Une première correction guettait la **présence d'options**. Elle était satisfaite
par celles du montage précédent : après un enregistrement, le formulaire est
ré-échangé et le sélecteur se remonte, mais les anciennes options sont encore là.
La suite passait huit fois sur dix et journalisait l'erreur les deux autres —
**pire qu'un échec franc**.

L'attente porte désormais sur la **réponse réseau** (`page.expect_response`), qui
ne peut pas être satisfaite par un état antérieur. Trois passages consécutifs,
huit verts, zéro erreur de console.

Ce n'est pas un défaut du produit : aucun humain ne quitte la page trente
millisecondes après l'avoir ouverte. C'est le même genre d'artefact que la
fenêtre non câblée, et il se règle de la même façon — une condition précise,
jamais une durée.

## Falsification

| Mutation | Constaté |
|---|---|
| Le classement ignore la carte des points manuels | **le test central rouge** : « l'ordre n'a pas suivi », plus 3 autres |
| Le gabarit affiche `row.points` au lieu de `row.total` | 1520 unitaires **verts**, e2e rouge : « le total affiché contredit l'ordre » |

## La fixture crée sa propre compétition

Et non celle qui porte des lignes en base de développement : un test qui dépend
d'un état posé à la main rougit chez quelqu'un d'autre. Une fixture `autouse`
vide les points manuels **avant et après** chaque test — sans quoi l'ordre des
tests deviendrait significatif, et un échec porterait sur sa prémisse plutôt que
sur son assertion.
