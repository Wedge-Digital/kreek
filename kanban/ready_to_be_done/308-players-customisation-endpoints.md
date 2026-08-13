# BC `players` — Endpoints de customisation

**Priorité : haute**
**Dépend de :** `306-players-customisation-use-cases.md`, `307-players-customisation-widget.md`
**Contexte :** `players` — controller HTTP

## Objectif

Les sept `POST` : cinq mutations unitaires, la validation, l'annulation.

**Spec :** `04-dtos.md` et `07-integration.md`.

---

## Routes

```
POST .../customisation/skills/add      { skill_id, expected_version }
POST .../customisation/stats/add       { stat, crans, expected_version }
POST .../customisation/price/adjust    { delta_kpo, expected_version }
POST .../customisation/spp/add         { amount, expected_version }
POST .../customisation/lines/remove    { line_id, expected_version }
POST .../customisation/validate
POST .../customisation/cancel
```

`Form` urlencoded : les charges sont des scalaires plats, c'est natif à HTMX et
sans extension, et c'est ce qu'utilisent les endpoints de panier de `teams`.

**`crans` porte le sens en qualité du joueur** (+1 améliore), jamais l'offset
brut : la traduction appartient au domaine, seul détenteur de la table de
directions.

## Réponses

| Résultat | Réponse |
|---|---|
| Mutation acceptée | **200** + panneau re-rendu |
| Refus métier | **200** + panneau portant `RefusalVm` |
| `ConcurrentWrite` | **200** + panneau re-rendu, **sans message d'erreur** |
| Validation | **200** + `HX-Refresh: true` |
| Annulation | **200** + fragment du journal |
| Sans droit | **403** |
| Joueur inconnu | **404** |
| Formulaire malformé | **400** |

Le refus métier répond **200** pour la même raison que l'endpoint d'édition
d'effectif (carte 294) : un 4xx ferait échouer le swap HTMX et laisserait le
commissaire devant un panneau figé.

`ConcurrentWrite` n'est **pas** une erreur d'utilisateur : le panneau re-rendu
porte l'état réel, le commissaire voit que son geste n'a pas pris et le refait.
Un message sur un événement aussi rare qu'invisible ferait plus de bruit que de
bien.

Le refus s'affiche **là où l'on a cliqué**, d'où `RefusalTarget`. Un bandeau en
tête de panneau obligerait à deviner laquelle des quatre actions a échoué.

## Autorisation

**Vérifiée sur chaque endpoint.** Masquer le bouton n'est pas un contrôle
d'accès.

---

## Checklist

- [ ] Les cinq DTOs de formulaire
- [ ] Les sept handlers
- [ ] Autorisation sur chacun
- [ ] `RefusalVm` porté par le panneau, ciblé sur l'action refusée
- [ ] `HX-Refresh` sur validation
- [ ] Retour au journal sur annulation
- [ ] Wiring `router.rs`
- [ ] Test : refus métier → 200 et panneau re-rendu, panier intact
- [ ] Test : membre simple → 403 sur chaque endpoint
- [ ] Test : `expected_version` périmé → panneau re-rendu sans message d'échec
