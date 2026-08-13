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

- [ ] `ISpaceOwnership` pour ce BC, enregistré dans `main.rs`
- [ ] Tests de handler : matrice d'appartenance, lecture et écriture
- [ ] Un scénario e2e de bout en bout
