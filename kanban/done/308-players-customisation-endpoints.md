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

- [x] Les cinq DTOs de formulaire — tous avec `expected_version`, que la phase 4 avait omis
- [x] Les sept handlers
- [x] Autorisation sur chacun — `customisation_access::garde`, partagé avec le panneau
- [x] `RefusalVm` porté par le panneau, ciblé sur l'action refusée — **réparti par le VM**, pas par le template
- [x] `HX-Refresh` sur validation
- [x] Retour au journal sur annulation
- [x] Wiring `router.rs` — les 404 laissés par la 307 sont levés
- [ ] Test : refus métier → 200 et panneau re-rendu, panier intact — **reporté en 309**
- [ ] Test : membre simple → 403 sur chaque endpoint — **reporté en 309**
- [ ] Test : `expected_version` périmé → panneau re-rendu sans message d'échec — **reporté en 309**

---

## Correction apportée à la carte 306

`ValidateCustomisationCommand` **gagne `expected_version`**, et le use case
refuse un panier dont la version a bougé.

Sans cette garde, la validation était le seul endpoint sans protection de
concurrence — et surtout, le comptage d'identifiants devenait fragile. Le
handler doit savoir combien de lignes le panier porte pour engendrer autant de
`CustomisationId` ; il les compte sur une lecture **antérieure** à celle du use
case. Un panier modifié entre les deux faisait diverger les comptes et sortir
`IdentifiantsManquants` — un code dont la documentation dit qu'il signale un
bug d'appelant, pour ce qui n'est qu'une écriture concurrente.

Avec la garde, le contenu ne peut plus changer sans que la version change : la
course retombe sur `ConcurrentWrite`, chemin déjà silencieux, et
`IdentifiantsManquants` redevient ce qu'il prétend être.

Le panneau de la carte 307 envoyait **déjà** `expected_version` sur son bouton
d'enregistrement : c'est la table des routes de cette carte qui était
incomplète, pas le template.

## Les trois tests de comportement partent en 309

« Membre simple → 403 », « refus métier → 200 » et « version périmée → panneau
re-rendu » sont des contrats **HTTP**. Le projet n'a aucun harnais de test au
niveau handler — ni `oneshot`, ni équivalent : tout comportement passe par la
suite e2e, c'est-à-dire la carte 309, qui possède déjà les scénarios.

La carte 311 propose de combler ce manque ; ces trois vérifications sont son
meilleur premier cas.

**Le harnais existe depuis la carte 311** — `web::test_harness::Harnais`. Les
trois vérifications restent à écrire, mais elles ont désormais un foyer, et
n'ont plus à passer par le navigateur.

## Ce qui est couvert ici, en tests unitaires

- `parse_stat` accepte exactement les cinq clés, et **celles que le panneau
  rend** — un test lie les deux listes, une divergence donnerait un `400` sur
  un bouton du panneau
- un identifiant par ligne, tous distincts
- la répartition du refus : un refus de caractéristique ne touche qu'une carte,
  un refus de prix ne déborde sur aucune autre zone
- la garde de version à la validation : conflit, et **rien n'est consommé**
