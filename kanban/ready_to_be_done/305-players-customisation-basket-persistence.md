# BC `players` — Persistance du panier de customisation

**Priorité : haute**
**Dépend de :** `304-players-customisation-basket-domain.md`
**Contexte :** `players` — repository / migration

## Objectif

Persister les lignes du panier, avec garde de version optimiste et péremption
à 24 h d'inactivité.

**Spec :** `docs/specs/player-customisation/player-detail/07-integration.md`.

---

## Migration

```sql
CREATE TABLE IF NOT EXISTS players__customisation_baskets (
    player_id  TEXT PRIMARY KEY,
    space_id   TEXT        NOT NULL,
    state      JSONB       NOT NULL,
    version    INT         NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

Clé `player_id` seul : le panier est propre au **joueur**, pas à son auteur.
Deux commissaires le partagent — cas écarté comme improbable au niveau métier
(phase 2).

`state` ne porte **que les lignes**. Joueur, catalogue et caractéristiques de
base sont rechargés à chaque hydratation : c'est ce qui garantit qu'un panier
d'une heure est jugé contre le joueur d'aujourd'hui.

## Port

`ICustomisationBasketRepository` — `load` / `save(expected_version)` /
`delete`, calqué sur `teams::IPhaseBasketRepository`.

`delete` est **idempotent** : un panier déjà absent n'est pas une erreur
(phase 5).

## Péremption

**24 h après la dernière modification** — `updated_at`, pas `created_at` :
c'est l'inactivité qui périme, pas l'ancienneté. Un panier commencé il y a 23 h
mais travaillé à l'instant ne doit pas mourir en pleine session.

La constante vit dans le **domaine du panier**, pas dans un `WHERE` — sans quoi
personne ne la trouvera le jour où il faudra la changer.

La vérification est faite par l'appelant à l'ouverture de la fiche (carte 307),
pas par le repository : celui-ci expose `updated_at`, le domaine décide.

---

## Checklist

- [ ] Migration `players__customisation_baskets`
- [ ] `CustomisationBasketState` + `ICustomisationBasketRepository` dans `ports.rs`
- [ ] Implémentation Pg avec garde de version
- [ ] Constante de péremption dans le domaine du panier
- [ ] Test repository : `save` puis `load` rend les mêmes lignes
- [ ] Test repository : `save` avec une version périmée → `ConcurrentWrite`
- [ ] Test repository : `delete` sur un panier absent réussit
- [ ] Test : un panier dont `updated_at` a plus de 24 h est jugé périmé
