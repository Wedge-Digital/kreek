# La correction ouverte au coach lit la page trop tôt

**Priorité : haute** — la suite e2e doit passer à chaque fois (décision du 2026-11-11)
**Épic :** aucune · **Dépend de :** rien
**Trouvée par :** la CI GitHub du 2026-11-11 :
`test_deplacer_un_match.py::test_un_coach_voit_la_correction_mais_pas_le_deplacement`
trouve le bouton de correction désactivé.

## Le constat

**Une course.** Journal CI `43011210712` : publication à `06.350`, réponse du GET du
récapitulatif à `06.407`, passage de `autre` en `PlayerImprovement` à `06.487`.
`correction_eligibility_service` lit la phase des équipes ;
`is_in_post_match_improvement` répond faux, le verdict est « bloqué »
(`PhaseAdvanced`). Le listener `teams` est asynchrone : 6 ms de médiane et 171 ms
au plus sur la base e2e — gagné en local, perdu en CI.

**Un décor bancal.** `equipe` (`teams[1]`) est déjà en `PlayerImprovement` depuis
`match_publie` ; `teams` **refuse** la seconde séquence post-match (« phase de jeu
incorrecte »). Le test passe pour `equipe` par accident.

## Ce qui change (tests seulement)

- `match_publie` se joue entre `teams[0]` et `teams[2]` : `teams[1]` reste prête à
  jouer.
- Après `_publier_un_match`, le test attend par `attendre_que` que les deux équipes
  soient en `PlayerImprovement` avant le GET.

Les deux défauts produit vus en passant sont portés par la carte 665.

## Terminé quand

- [ ] un GET fait **sans** l'attente, juste après la publication, échoue avant le
      correctif du décor — preuve que la course est la bonne ;
- [ ] le test passe **20 fois de suite** ;
- [ ] la suite complète passe.

## Conséquences sur les données de production

Aucune : seul un test change.
