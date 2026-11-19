# Tests E2E — widget "Derniers résultats" sur l'accueil

**Priorité : haute**
**Dépend de :** `282-news-home-latest-results-integration.md`
**Contexte :** `tests/e2e/`

## Objectif

Couvrir en navigateur le comportement du widget, notamment ce qu'aucun test
unitaire ne peut vérifier (rendu réel du fragment HTMX, visibilité
conditionnelle du lien). Spec complète :
`docs/specs/accueil-derniers-resultats/widget-derniers-resultats/07-integration.md`.

---

## Scénarios

| Scénario | Vérification |
|---|---|
| Espace avec ≥ 4 matchs `completed` | Widget affiche exactement 4 résultats, du plus récent au plus ancien |
| Espace avec 1-3 matchs `completed` | Widget affiche le nombre exact de résultats disponibles |
| Espace sans match `completed` | Message "Aucun résultat pour le moment." |
| Match nul (scores égaux) | Aucun des deux scores n'a la classe `winner` |
| Coach d'une des deux équipes du match | Résultat cliquable, navigue vers le rapport |
| Coach d'aucune équipe, non admin | Résultat affiché mais non cliquable (pas de `<a>`) |
| Admin d'espace | Tous les résultats cliquables, quelle que soit la compétition |
| Résultats de compétitions différentes | `competition_name` correct affiché pour chaque ligne |

## Checklist

- [ ] Fixtures : au moins 2 compétitions/saisons différentes du même espace, avec matchs `completed` à des `published_at` distincts
- [ ] Les 8 scénarios ci-dessus passent
- [ ] `make e2e` (ou `test-impacted` ciblé sur ce test) vert
- [ ] Mise à jour de la carte d'impact tests↔BCs (skill `test-impact`) si un nouveau test e2e est ajouté
