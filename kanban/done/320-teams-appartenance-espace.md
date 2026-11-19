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

- [x] `ISpaceOwnership` pour ce BC, enregistré dans `main.rs`
- [x] Tests de handler : matrice d'appartenance, lecture et écriture
- [x] Sonde d'écriture sur une équipe en phase de recrutement, pour lever le doute du `422`
- [ ] Un scénario e2e de bout en bout — **non écrit**, même raison qu'en cartes 318 et 319

## La sonde a levé le doute — et le résultat est pire que la lecture

L'audit n'avait prouvé qu'une fuite en lecture ; sa sonde d'écriture avait rendu
`422`, mais sur une équipe en `ReadyToPlay`. Refaite sur une équipe **en phase
de recrutement** :

```
POST /app/<espace étranger>/teams/<équipe>/recruitment/players/add
→ 200, ligne écrite
teams__phase_baskets.space_id = l'espace de l'ATTAQUANT
```

Un admin d'un espace quelconque recrutait dans l'équipe d'un autre, touchant
**effectif et trésorerie**. Le `422` venait bien du garde de phase, pas de
l'autorisation — ne jamais conclure d'un refus qu'il vient de celui qu'on
soupçonne.

Troisième preuve d'écriture croisée après `players` et `news`, et la plus
lourde. Panier de sonde supprimé, base rendue à son état.

Après correctif :

```
lecture,  espace étranger → 404      espace réel → 200
écriture, espace étranger → 404      aucun panier créé
```

## Un résolveur qui en couvre deux

`team_proj` porte `space_id` : comparaison directe. Ce résolveur couvre aussi
les **quatre routes de `match_report`** portant `{team_id}` — la liste du
middleware étant plate, un BC bénéficie des résolveurs des autres sans les
connaître.

## Le test d'écart vit au niveau du résolveur

Deuxième BC dans ce cas après `match_report`, et pour la même raison :
`team_detail` charge l'agrégat depuis l'event store, donc une équipe semée en
projection seule rend `404` quel que soit l'espace, et l'assertion nominale en
HTTP serait verte sans rien prouver.

L'écart en HTTP est vérifié par la sonde ci-dessus, sur données réelles.
