# BC `news` — cloisonnement des espaces

**Priorité : moyenne** — écriture prouvée, mais bénigne
**Dépend de :** `324` (le middleware commun)
**Contexte :** `news` — 5 routes

---

## Prouvé en écriture

C'est sur ce BC que l'écriture croisée a été démontrée :

```
GET  /app/<espace E2E>/home/articles/<article d'un autre espace>
→ 200, le titre réel est servi

POST …/comments   { content: "sonde-audit-316" }
→ 200, la ligne est écrite en base
```

Le commentaire de sonde a été supprimé, base rendue à son état initial.

## Pourquoi en dernier malgré la preuve

C'est le seul BC dont l'abus ne touche pas les données de jeu : un commentaire
indu se supprime, un prix ou un score customisé se propage.

Le rang dit la gravité, pas la certitude — c'est ici que la preuve est la plus
nette.

## Les ressources, et le saut

| Ressource | Espace |
|---|---|
| `articles` | colonne `space_id` — comparaison directe |
| `comments` | **aucune colonne** → saut par `article_id` |



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
| `articles` | colonne `space_id` — direct |
| `comments` | **saut** par `article_id` → `articles.space_id` |

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
- [x] Suite e2e complète verte — 182 passés

## Réalisé

**Un seul résolveur, et non deux.** La carte prévoyait un saut
`comments` → `articles`, par symétrie avec les saisons. Il est **inutile** :
aucune route de ce BC ne porte d'identifiant de commentaire. Les commentaires
s'atteignent par `/home/articles/{article_id}/comments`, donc contrôler
l'article suffit.

La carte avait sur-spécifié en généralisant depuis `competitions` sans regarder
les routes. Vérifier avant d'écrire aurait coûté une commande.

### La sonde qui a lancé la série, rejouée

```
lecture croisée  → 404   (était 200, titre réel servi)
écriture croisée → 404   (était 200, ligne écrite)
commentaires : 2 avant, 2 après
```

C'est ici que l'audit de la carte 316 avait prouvé l'écriture croisée. Le geste
est désormais refusé, et un test le fige — il vérifie aussi qu'**aucune ligne
n'est écrite**, un `404` seul ne prouvant pas que rien n'a été fait.

### L'écart est prouvable en HTTP

Troisième BC seulement dans ce cas, avec `competitions` et `players` :
l'article se sert depuis sa table, sans event store. `match_report` et `teams`
avaient dû faire prouver l'écart au niveau du résolveur.
