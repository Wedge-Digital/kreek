# Les tests E2E du panneau de statistiques

**Épic :** aucune · **Ordre :** 2 · **Dépend de :** 474
**Conception :** `docs/specs/statistiques-de-competition/README.md`

## Objectif

`tests/e2e/test_competition_stats.py`. Vérifier dans un navigateur que les
chiffres affichés sont ceux des rapports publiés — ce qu'aucun test unitaire ne
voit.

## Les scénarios

| Scénario | Ce qu'il prouve |
|---|---|
| `test_l_onglet_stats_affiche_les_quatre_tableaux` | le chemin heureux, après swap HTMX |
| `test_les_valeurs_correspondent_aux_rapports_publies` | **le bout en bout** |
| `test_une_saison_sans_match_affiche_l_etat_vide` | un seul message, pas quatre |

## Celui qui porte le poids

**`test_les_valeurs_correspondent_aux_rapports_publies`** publie deux rapports
dont il connaît les chiffres, puis lit les quatre tableaux et compare.

Il traverse tout : le listener de publication qui remplit
`competition_match_display_proj`, le `UNION ALL` qui recolle domicile et
extérieur, et les quatre tris.

**Il doit inclure une équipe qui joue une fois à domicile et une fois à
l'extérieur** — c'est le seul montage où une moitié d'`UNION` oubliée se voit,
et ce défaut-là donne des totaux exactement divisés par deux, donc plausibles.

Et **une équipe qui inflige beaucoup en encaissant peu**, pour que l'inversion
pour/contre la fasse basculer d'un tableau à l'autre.

## Le piège de la fenêtre non câblée

L'onglet arrive par swap HTMX. Tout clic qui suit tombe dans la fenêtre où
l'élément est peint mais pas encore câblé — le clic s'y perd sans requête, sans
erreur de console, sans rien.

```python
from htmx_helpers import cliquer_quand_cable
cliquer_quand_cable(page, '.tab[data-tab="stats"]')
```

**Aucun `sleep`.** Une durée fixe n'a aucune marge sur une machine chargée — et
c'est exactement là que la suite échoue.

## Checklist

- [ ] `tests/e2e/test_competition_stats.py`, les trois scénarios
- [ ] Une équipe à domicile **et** à l'extérieur dans le montage
- [ ] Une équipe qui inflige beaucoup et encaisse peu
- [ ] `cliquer_quand_cable`, **aucun `sleep`**
- [ ] `make e2e` vert
