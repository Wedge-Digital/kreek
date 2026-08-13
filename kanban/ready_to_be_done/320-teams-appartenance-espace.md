# BC `teams` — cloisonnement des espaces

**Priorité : haute** — fuite prouvée en lecture
**Dépend de :** `324` (le middleware commun)
**Contexte :** `teams` — 29 routes

---

## Prouvé en lecture

Équipe « Les gros bourrins » appartenant à l'espace *Bordeaux BBC*, demandée
depuis l'espace *E2E* :

```
GET /app/<espace E2E>/teams/<équipe de Bordeaux>
→ 200, le nom réel de l'équipe est servi
```

## L'écriture reste non prouvée — et c'est à retenir

L'ajout au panier de recrutement de cette même équipe a rendu `422`. Mais
l'équipe était en phase `ReadyToPlay`, donc **c'est vraisemblablement le garde
de phase qui a parlé, pas l'autorisation**.

La sonde est donc à refaire sur une équipe réellement en phase de recrutement.
Ne pas conclure de ce `422` que l'écriture est protégée : rien dans le code ne
le suggère.

## La ressource

`team_proj` porte `space_id` : **comparaison directe**.

Trois fichiers de `teams/io/web/` mentionnent `space_id`, mais **aucune
comparaison** — vérifié. Ce sont des passages de paramètre, pas des contrôles.



## Ce que cette carte apporte

Le mécanisme est commun (carte **324**) : un middleware unique lit les
paramètres du chemin, interroge un résolveur, compare, et rend `404` sur
divergence. Cette carte n'apporte que **la réponse de ce BC sur ses propres
ressources**.

```rust
impl ISpaceOwnership for … {
    fn param(&self) -> &'static str { "…_id" }
    async fn space_of(&self, id: &str) -> Option<SpaceId> { … }
}
```

Chaque BC répond via **son propre repository** : le middleware n'interroge
aucune table, ce qui préserve la souveraineté des données.

### Les ressources, et leurs sauts

| Ressource | Espace |
|---|---|
| `team_proj` | colonne `space_id` — direct |

Un saut est préféré à l'ajout d'une colonne `space_id` : la ressource n'a pas
d'espace en propre, elle en hérite, et dénormaliser créerait une seconde source
de vérité qui divergerait.

## Tests — plus de report

Le harnais de la carte 311 existe. Le patron est
`players/io/web/tests/test_space_scope.rs`.

- lecture croisée → `404`, **et** lecture nominale → `200`. C'est l'écart qui
  prouve : une assertion qui ne vérifierait que le refus passerait tout aussi
  bien si la ressource n'existait pas ;
- écriture croisée → `404`, écriture nominale → autre chose que `404` ;
- identifiant d'espace mal formé → `400`.

La sémantique elle-même est testée une fois, en carte 324 — pas ici.

## Checklist

- [ ] `ISpaceOwnership` pour ce BC, enregistré dans `main.rs`
- [ ] Tests de handler : matrice d'appartenance, lecture et écriture
- [ ] Sonde d'écriture sur une équipe en phase de recrutement, pour lever le doute du `422`
- [ ] Un scénario e2e de bout en bout
