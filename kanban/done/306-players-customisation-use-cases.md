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

- [x] `customisation_basket_hydration_service`
- [x] Les cinq mutations dans `customisation_basket_mutation.rs`
- [x] `validate_customisation_use_case` — suppression **avant** append
- [x] `cancel`
- [x] Les commandes dans `commands.rs`, avec `expected_version`
- [x] Test : une mutation refusée ne touche pas les lignes déjà présentes
- [x] Test : `expected_version` périmé → `ConcurrentWrite`
- [x] Test : la validation produit **un événement par ligne**
- [x] Test : une ligne devenue invalide fait tout échouer, rien n'est appliqué
- [x] Test : la validation supprime le panier
- [x] Test : l'annulation sur panier absent réussit

---

## Notes d'implémentation

**`list_all_skills()` a été avancé ici**, alors qu'il était prévu en carte 307 :
l'hydratation en dépend. `references` savait déjà lister ses compétences,
l'adapter n'a fait que traduire.

**Le `CustomisationId` est porté par la commande**, engendré par le handler. Ni
le domaine ni le use case ne doivent tirer d'aléatoire, sous peine de devenir
intestables.

**Les compétences possédées incluent base et acquises**, sans distinction
d'origine : une compétence obtenue en SPP bloque son ajout par customisation, et
réciproquement. Lecture littérale de la règle de phase 1, commentée sur place
pour qu'on ne la « corrige » pas.

**Un panier absent donne un panier vide en version zéro** — c'est ce qui
l'ouvre au premier geste, sans endpoint dédié.

## Deux tests qui gardent des décisions

`le_panier_est_supprime_avant_l_append` fait échouer l'append volontairement.
L'ordre décidé en phase 5 ne casse aucun test fonctionnel s'il est inversé : il
ouvre seulement la porte à une double écriture, que rien d'autre n'attraperait.

`une_ligne_devenue_invalide_fait_tout_echouer` simule l'acquisition de la
compétence par une autre voie entre l'ajout et la validation — le cas même que
la revalidation existe pour attraper.
