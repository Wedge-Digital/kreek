# Les tests E2E des compétences personnalisées

**Épic :** E10 — Référentiels éditables · **Ordre :** 10
**Dépend de :** 463 à 471
**Conception :** `docs/specs/competences-personnalisees/page-de-gestion/07-integration.md`

## Objectif

`tests/e2e/test_custom_skills.py`. Vérifier dans un navigateur ce qu'aucun test
unitaire ne voit — le rendu HTMX, le cache non gelé, et le chemin qui va jusqu'à
l'argent.

## Les scénarios

| Scénario | Ce qu'il prouve |
|---|---|
| `test_creer_une_competence` | chemin heureux : formulaire vidé, liste rechargée |
| `test_la_competence_creee_apparait_dans_le_selecteur_de_spp` | **le test qui vaut le prix de la suite** |
| `test_un_nom_deja_pris_est_refuse` | C6, corpus compris |
| `test_un_nom_avec_apostrophe_passe` | `TEXTE_SAISI`, bout en bout |
| `test_une_competence_portee_garde_ses_champs_de_libelle_ouverts` | U2 |
| `test_une_competence_portee_affiche_sa_categorie_en_texte` | pas grisée, un fait |
| `test_le_bouton_supprimer_n_existe_pas_sur_une_competence_portee` | absent, pas désactivé |
| `test_corriger_le_nom_d_une_competence_portee_reussit` | **U6, le piège** |
| `test_un_non_admin_ne_voit_pas_la_page` | P1 |
| `test_la_competence_d_un_autre_espace_rend_404` | P2, via le résolveur |
| `test_le_type_elite_coute_dix_kpo_de_plus` | C4, jusqu'au barème |

## Les trois qui portent le poids

**`test_la_competence_creee_apparait_dans_le_selecteur_de_spp`** traverse tout :
l'écriture en base, le rafraîchissement des deux cartes de cache, l'aiguillage
par préfixe dans `find_skill_by_uid`, `list_skills_for_space`, et la route du
sélecteur qui a gagné son `space_id`. **C'est aussi le seul qui prouve que le
cache n'est pas gelé** — sans redémarrage entre la création et la vérification.

**`test_corriger_le_nom_d_une_competence_portee_reussit`** est le pendant e2e du
test unitaire de la carte 464. Sans lui, la suite est verte sur une
fonctionnalité où plus personne ne peut corriger une faute de frappe : les refus
refusent, les créations créent, et le seul chemin cassé est celui qu'on n'a pas
pensé à parcourir.

**`test_le_type_elite_coute_dix_kpo_de_plus`** est le seul qui atteigne l'argent.
Il crée une compétence Élite, la fait acheter en SPP, et vérifie le débit — c'est
ce qui attraperait un `"Elite"` sans accent, qu'aucun test de sérialisation ne
verrait si quelqu'un corrigeait le test plutôt que le type.

## Le piège de la fenêtre non câblée

Les deux widgets se rechargent sur `customSkillsChanged` : **chaque action qui
suit une mutation tombe dans la fenêtre où l'élément est peint mais pas encore
câblé.** Le clic s'y perd sans requête, sans erreur de console, sans rien.

```python
from htmx_helpers import cliquer_quand_cable
cliquer_quand_cable(page, ".cs-btn--danger")
```

**Aucun `sleep`.** Une durée fixe n'a aucune marge sur une machine chargée — et
c'est exactement là que la suite échoue — tout en coûtant son délai aux milliers
d'appels où tout est déjà prêt.

## Le contrôle visuel

`uv run python visual/debordements.py` doit rester muet sur la page des
compétences (carte 469). Un débordement signalé ici veut dire que les teintes
sont restées dans `widgets/players-widget.css`.

## Checklist

- [ ] `tests/e2e/test_custom_skills.py`, les onze scénarios
- [ ] `cliquer_quand_cable` partout après une mutation, **aucun `sleep`**
- [ ] `debordements.py` muet sur la nouvelle page
- [ ] `make e2e` vert
- [ ] `make lint && make test && make check-arch`
