# Competitions — Tests E2E de la saisie des bonus

**Priorité : moyenne**
**Dépend de :** `201-bonus-domaine.md`, `202-bonus-formulaire.md`
**Contexte :** `tests/e2e/test_competition_rules_bonus.py` (nouveau)
**Spec :** `docs/specs/ranking-bonus-points/competition-rules-form/07-integration.md`

## Objectif

Couvrir en navigateur ce qu'aucun test unitaire ne garantit : le rendu réel du
formulaire, la sérialisation JS des bonus et leur ré-hydratation à l'édition.

## Scénarios (cf. `07-integration.md`)

1. Créer une compétition, atteindre l'étape 2 (Règles).
2. Activer le **bonus agressif**, saisir points X et seuil Y ; modifier le **seuil
   défensif** (≠ 1).
3. Enregistrer & continuer → étape 5 : le récap affiche les 3 bonus avec les bons
   seuils.
4. Revenir éditer l'étape 2 : ré-hydratation correcte (agressif coché, X/Y et seuil
   défensif restaurés).
5. (Optionnel, fixture) Compétition dont les règles JSONB ne contiennent pas les
   nouveaux champs → étape 2 affiche les défauts (agressif décoché, seuil défensif
   = 1) sans erreur.

## Checklist

- [ ] `tests/e2e/test_competition_rules_bonus.py` créé
- [ ] Scénarios 1-4 couverts (5 si fixture rétro-compat ajoutée)
- [ ] `make e2e` passe (nécessite le serveur dev déjà lancé par l'utilisateur)
