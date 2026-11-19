# Les tests E2E de l'onglet Matchs

**Épic :** E06 — La fiche d'équipe complétée · **Ordre :** 3 · **Dépend de :** 477
**Conception :** `docs/specs/matchs-d-une-equipe/README.md`

## Objectif

`tests/e2e/test_team_matches.py`. Vérifier dans un navigateur ce qu'aucun test
unitaire ne voit — le rendu du composant partagé sur sa nouvelle page, et le
lien de rapport présent ou absent selon qui regarde.

## Les scénarios

| Scénario | Ce qu'il prouve |
|---|---|
| `test_l_onglet_matchs_liste_les_matchs_de_l_equipe` | le chemin heureux |
| `test_le_prochain_match_apparait_en_tete` | l'ordre chronologique centré |
| `test_le_coach_de_l_equipe_peut_ouvrir_le_rapport` | le contrôle d'accès |
| `test_un_visiteur_ne_voit_pas_le_lien_du_rapport` | son autre moitié |
| `test_une_equipe_sans_match_affiche_l_etat_vide` | l'état vide |
| `test_le_bloc_de_match_reste_correct_sur_la_page_competition` | **la non-régression** |

## Celui qui porte le poids

**`test_le_bloc_de_match_reste_correct_sur_la_page_competition`** n'est pas une
politesse. La carte 476 déplace 93 règles CSS hors de
`pages/competition-detail.css` et retire leur préfixe de portée : c'est le seul
geste du chantier qui touche une page qui marche aujourd'hui.

Il vérifie le rendu de l'onglet Résultats de compétition — scores, blessures,
badge « en cours », lien cliquable — après le déplacement. Un composant extrait
qui casse sa page d'origine est le mode de panne classique de cette
manœuvre.

À compléter par le contrôle visuel : `uv run python visual/debordements.py`
doit rester muet sur la fiche d'équipe.

## Le piège de la fenêtre non câblée

L'onglet arrive par swap HTMX — le mécanisme d'aiguillage de la carte 434. Tout
clic qui suit tombe dans la fenêtre où l'élément est peint mais pas encore
câblé : le clic s'y perd sans requête, sans erreur de console, sans rien.

```python
from htmx_helpers import cliquer_quand_cable
cliquer_quand_cable(page, '.tab[data-tab="matchs"]')
```

**Aucun `sleep`.** Une durée fixe n'a aucune marge sur une machine chargée — et
c'est exactement là que la suite échoue.

## Checklist

- [x] `tests/e2e/test_team_matches.py`, les six scénarios
- [x] Un match à venir **et** un match joué dans le montage
- [x] Deux comptes : le coach de l'équipe, et un membre quelconque de l'espace
- [x] `debordements.py` — la fiche d'équipe n'y figure pas
- [x] `cliquer_quand_cable`, **aucun `sleep`**
- [x] `make e2e` vert — 330 passés, 7 ignorés

---

# Ce que la réalisation a appris

## Le montage repose sur une coïncidence utile, qu'il faut nommer

`build_full_competition` distribue les équipes aux coachs de l'espace **triés
par nom** : `DevCoach` d'abord — administrateur — puis `E2E Coach 01`, qui est
justement le coach que `bypass_auth` connecte sur
`X-Bypass-Auth-Profile: simple`, et qui est `SpaceUser`.

C'est ce qui rend les deux moitiés du contrôle d'accès observables sur la même
base : sa propre équipe pour le lien présent, une équipe de l'autre appariement
pour le lien absent. Le montage l'assert plutôt que de le supposer — si la
distribution changeait, les deux tests passeraient sans rien prouver.

## Le test de l'ordre ne prouvait rien, comme son jumeau unitaire

Écrit d'abord en jouant la **première** journée et en laissant la seconde à
venir — l'ordre naturel. Le match à venir avait alors la position la plus haute,
et un `round_position DESC` nu produisait exactement l'ordre attendu. Vérifié :
la falsification a supprimé tout le `CASE` sur le statut **sans faire rougir le
test**.

Le montage joue donc la seconde journée et laisse la première à venir. C'est le
second test de ce chantier à tomber dans ce piège — après
`un_match_en_cours_passe_devant_les_a_venir` de la carte 477, à quelques heures
d'intervalle. La leçon n'est pas « attention aux positions » : c'est qu'**un test
d'ordre doit être construit pour que le tri naïf donne le mauvais résultat**,
sinon il n'observe rien.

## `team_proj.coach_name` est vide sur toute équipe construite en HTTP

Le constructeur poste un nom vide et laisse l'agrégat résoudre par
l'identifiant. S'y fier rendait une liste vide, et le montage échouait sur un
`IndexError` qui ne disait rien de sa cause. La résolution passe par une
jointure sur `coach_id`.

C'est aussi ce qui explique la colonne « COACH » vide et le « Roster · » sans
suite sur les fiches des équipes de test — un artefact du constructeur, pas un
défaut de l'écran.

## Ce que le contrôle de débordement ne peut pas voir

Il reste les **7 débordements préexistants** de la carte 476, et la fiche
d'équipe n'y figure pas. Mais l'onglet Matchs n'est pas non plus *visité* : le
collecteur connaît `equipe-detail`, pas `/matchs`.

L'y ajouter donnerait un faux confort — le contrôle compare à un relevé de
référence qui dit ce que chaque page chargeait **avant** la fusion des feuilles,
et une page absente de ce relevé est **sautée en silence**. C'est la même
asymétrie que pour une feuille de composant neuve, contournée dans la 476 en la
scopant. Le mécanisme protège l'existant et ne voit pas ce qui naît.

## Falsification

| Mutation | Constaté |
|---|---|
| L'onglet Matchs redevient inerte | `…liste_les_matchs_de_l_equipe` rouge |
| L'ordre redevient celui de la compétition | **passait**, puis rouge après correction du montage |
| L'autorisation devient permissive | `…un_visiteur_ne_voit_pas_le_lien…` rouge |
| L'autorisation refuse tout le monde | `…le_coach_de_l_equipe_peut_ouvrir…` rouge |
| L'état vide disparaît | `…affiche_l_etat_vide` rouge |
| La page compétition hérite pastille et libellé | `…reste_correct_sur_la_page_competition` rouge |

## L'axe 8 a servi

`check-arch` a signalé `test_team_matches.py` comme absent de la carte d'impact
**avant** qu'on y pense — première prise du verrou réparé par la carte 480, le
jour même.
