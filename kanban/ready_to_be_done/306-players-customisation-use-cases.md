# BC `players` — Use cases de customisation

**Priorité : haute**
**Dépend de :** `303-players-stat-deltas-projection.md`, `305-players-customisation-basket-persistence.md`
**Contexte :** `players` — use cases

## Objectif

Les cinq mutations de panier, la validation et l'annulation.

**Spec :** `docs/specs/player-customisation/player-detail/05-use-cases.md`.

---

## Les cinq mutations

Même forme, et **elles ne décident de rien** : charger le joueur, charger les
lignes, hydrater, appeler la méthode domaine, persister sous garde de version.

**Aucune ne rend l'agrégat muté.** C'est la leçon de la carte 264 : `save` rend
la nouvelle version sans la reposer sur l'agrégat, et un appelant qui la
cuirait dans ses `hx-vals` ferait échouer chaque second clic. Le handler relit.

Les commandes portent `expected_version`, qui vient du formulaire — le panneau
étant re-rendu après chaque mutation, la version qu'il porte est fraîche.

**La première mutation crée le panier.** Pas d'endpoint d'ouverture.

## Validation

1. Charger joueur et panier — absent ou vide → `NothingToApply`
2. Hydrater et **revalider intégralement** : une ligne valide à l'ajout peut ne
   plus l'être
3. Produire un événement par ligne
4. **Supprimer le panier**
5. Appendre le lot (`append_batch`)
6. Émettre sur le bus interne

**L'ordre des étapes 4 et 5 est délibéré.** Les deux tables sont écrites par
deux transactions ; une panne entre elles perd la saisie sans rien appliquer.
L'ordre inverse écrirait deux fois des customisations sur des données de jeu,
ce qui ne se découvrirait que bien plus tard.

**Tout ou rien** : si une ligne est refusée à la revalidation, aucune n'est
appliquée.

## Annulation

Supprime le panier. Ni joueur chargé, ni domaine appelé. Idempotente.

---

## Checklist

- [ ] `customisation_basket_hydration_service`
- [ ] Les cinq mutations dans `customisation_basket_mutation.rs`
- [ ] `validate_customisation_use_case` — suppression **avant** append
- [ ] `cancel`
- [ ] Les commandes dans `commands.rs`, avec `expected_version`
- [ ] Test : une mutation refusée ne touche pas les lignes déjà présentes
- [ ] Test : `expected_version` périmé → `ConcurrentWrite`
- [ ] Test : la validation produit **un événement par ligne**
- [ ] Test : une ligne devenue invalide fait tout échouer, rien n'est appliqué
- [ ] Test : la validation supprime le panier
- [ ] Test : l'annulation sur panier absent réussit
