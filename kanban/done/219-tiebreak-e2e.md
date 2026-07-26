# Départages — Tests E2E du classement ordonné

**Priorité : haute**
**Dépend de :** carte 218 (câblage du classement ordonné)
**Contexte :** `tests/e2e/test_ranking_tiebreak.py` (nouveau)
**Spec :** `docs/specs/ranking/tiebreakers/tiebreak-calc/07-integration.md`

## Objectif

Vérifier en navigateur que le départage ordonne réellement le classement affiché, et que
l'ex æquo se voit. Aucun test unitaire ne couvre la chaîne complète configuration →
publication de matchs → widget rendu.

## Conception

Nouveau fichier calqué sur `tests/e2e/test_ranking_bonus.py`, qui fournit déjà la création
d'une compétition à règles personnalisées, l'activation en phase 2 et la publication de
rapports de match par API (`_record_action_api` gère les types d'action).

| Scénario | Vérifie |
|---|---|
| Départage par le 1ᵉʳ critère | Compétition avec `diff_td` seul actif ; deux équipes finissent à égalité de points avec des différences de TD distinctes → l'ordre des lignes du widget suit la différence de TD |
| Ex æquo total | Deux équipes égales sur les points **et** sur tous les critères actifs → le widget affiche **deux fois le même rang** |
| Numérotation standard | Après deux ex æquo au rang 2, l'équipe suivante affiche le rang **4** (règle 20) |

Le troisième scénario peut se greffer sur le second : un jeu de résultats produisant
1, 2, 2, 4 couvre les deux d'un coup si c'est plus simple à mettre en place.

## Prérequis d'exécution

Serveur dev lancé **par l'utilisateur** (`make dev-demo`) et `make seed_e2e` préalable.

**`make reset_db` est nécessaire ici** : les lignes de classement antérieures à la carte 216
ont leurs compteurs à 0 et fausseraient les comparaisons. Le demander à l'utilisateur, ne
jamais réinitialiser sa base de sa propre initiative (CLAUDE.md règle 8).

## Checklist

- [ ] `tests/e2e/test_ranking_tiebreak.py` créé
- [ ] Les 3 scénarios implémentés
- [ ] Le test passe en ciblé contre le serveur dev, base réinitialisée
- [ ] La suite e2e complète reste verte
