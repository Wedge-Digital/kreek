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

- [ ] `tests/e2e/test_team_matches.py`, les six scénarios
- [ ] Un match à venir **et** un match joué dans le montage
- [ ] Deux comptes : le coach de l'équipe, et un membre quelconque de l'espace
- [ ] `debordements.py` muet sur la fiche d'équipe
- [ ] `cliquer_quand_cable`, **aucun `sleep`**
- [ ] `make e2e` vert
