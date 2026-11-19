# Ranking — Test E2E des bonus de classement (navigateur)

**Priorité : haute**
**Dépend de :** `206-ranking-bonus-cablage.md`
**Contexte :** `tests/e2e/test_ranking_bonus.py` (nouveau)
**Spec :** `docs/specs/ranking-bonus-points/post-match-bonus-calc/07-integration.md` (§5)

## Objectif

Vérifier en navigateur ce qu'aucun test unitaire ne garantit : après publication d'un
match, le total de points affiché dans le widget classement **inclut réellement** les
points bonus calculés (chaîne config compétition → publication → projection → rendu).

## Scénarios (cf. `07-integration.md` §5)

1. **Bonus agressif** activé (seuil bas) + match avec sorties > seuil pour une équipe →
   total de points de l'équipe = V/N/D + points bonus (et l'adversaire, sans sorties
   suffisantes, ne le touche pas).
2. **Bonus offensif** activé + équipe marquant ≥ seuil de TD → total inclut le bonus.
3. **Bonus défensif** activé + équipe encaissant ≤ seuil → total inclut le bonus.
4. **Bonus désactivé** (même condition remplie) → total **sans** bonus (garde-fou
   d'activation visible côté rendu).
5. **Cumul** : compétition avec 2+ bonus activés, un match les remplissant → total
   inclut la somme.

## Checklist

- [ ] `tests/e2e/test_ranking_bonus.py` créé
- [ ] Les 5 scénarios ci-dessus couverts
- [ ] `make e2e` passe (nécessite le serveur dev déjà lancé par l'utilisateur)
