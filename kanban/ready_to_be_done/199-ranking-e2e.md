# BC `ranking` — Tests E2E de l'onglet Classement

**Priorité : haute**
**Dépend de :** `198-competitions-host-classement.md`
**Contexte :** `tests/e2e/test_ranking_classement.py` (nouveau)
**Spec :** `docs/specs/ranking/classement/07-integration.md`

## Objectif

Couvrir en navigateur ce qu'aucun test unitaire ne peut garantir : le rendu réel HTML/HTMX du widget, ses 4 états, et l'intégration avec la page hôte `competitions`.

## Scénarios (cf. `07-integration.md`)

1. Aucune équipe inscrite à la saison → "Aucune équipe dans la compétition."
2. Équipes inscrites, aucun rapport de match publié → "Aucun match n'a encore été joué."
3. Règles de classement non configurées pour la saison → état d'erreur affiché
4. Un rapport de match publié → les 2 équipes apparaissent avec MJ=1 et V/N/D/Pts corrects
5. Deux rapports de match publiés pour la même équipe → cumul correct (MJ=2), pas seulement le dernier match
6. Tri strictement décroissant par points (l'équipe la mieux classée en rang 1)

## Checklist

- [ ] `tests/e2e/test_ranking_classement.py` créé
- [ ] Les 6 scénarios ci-dessus couverts
- [ ] `make e2e` passe (nécessite le serveur dev déjà lancé par l'utilisateur)
