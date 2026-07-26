# Départages — Tests E2E navigateur

**Priorité : haute**
**Dépend de :** carte 211 (câblage complet)
**Contexte :** `tests/e2e/test_competition_rules_tiebreakers.py` (nouveau)
**Spec :** `docs/specs/ranking/tiebreakers/competition-rules-form/07-integration.md`

## Objectif

Vérifier en navigateur que la saisie des départages fonctionne réellement — case à
cocher, réordonnancement, garde-fou, round-trip de persistance. Un test unitaire ne
couvre pas le rendu HTMX/JS de cette section (cf. CLAUDE.md, « Couverture obligatoire »).

## Conception

Nouveau fichier calqué sur `tests/e2e/test_competition_rules_bonus.py` — le test
round-trip de la phase 2 livré par la feature `ranking-bonus-points`, qui fournit déjà
les helpers de navigation jusqu'au formulaire de règles.

| Scénario | Vérifie |
|---|---|
| Round-trip complet | Décocher deux critères, réordonner par drag & drop, enregistrer, revenir sur la phase 2 → ordre **et** activation restitués (règles 1 à 3) |
| Renumérotation | Décocher le critère de rang 1 → le suivant actif affiche 1, l'inactif affiche « — » |
| Garde-fou | Tout décocher → bouton d'enregistrement désactivé, message inline visible, aucune requête envoyée |
| Catalogue | Les 7 libellés attendus sont présents, `Nombre de cartons rouges` **absent** (règle 10) |

Le drag & drop se pilote avec `drag_to()` sur les `.tiebreak-row`. En cas d'instabilité,
repli sur `dispatch_event` des événements `dragstart` / `drop`.

## Prérequis d'exécution

Serveur dev lancé **par l'utilisateur** (`make dev-demo`, référentiel
`assets/references.example`) et `make seed_e2e` préalable. Ne jamais démarrer le serveur
soi-même (CLAUDE.md règle 8).

## Checklist

- [ ] `tests/e2e/test_competition_rules_tiebreakers.py` créé
- [ ] Les 4 scénarios ci-dessus implémentés
- [ ] Le test passe en ciblé (`make e2e` sur ce fichier) contre le serveur dev
- [ ] La suite e2e complète reste verte
