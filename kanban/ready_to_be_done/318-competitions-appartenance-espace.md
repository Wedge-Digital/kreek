# BC `competitions` et `ranking` — cloisonnement des espaces

**Priorité : haute** — fuite prouvée, données de jeu
**Dépend de :** `324` (le middleware commun)
**Contexte :** `competitions` (44 routes), `ranking` (2 routes)

---

## Prouvé

Compétition « Ligue Open » appartenant à l'espace *Bordeaux BBC*, demandée
depuis l'espace *E2E* dont l'appelant est admin :

```
GET /app/<espace E2E>/competitions/<comp de Bordeaux>/<saison>
→ 200, le nom réel de la compétition est servi
GET …/standings
→ 200
```

Aucun fichier de `competitions/io/web/` ne mentionne l'espace d'une ressource.

## Les deux ressources, et le saut

| Ressource | Espace |
|---|---|
| `competitions` | colonne `space_id` — comparaison directe |
| `competition_seasons` | **aucune colonne** → saut par `competition_id` |

Une saison n'a pas d'espace en propre, elle en hérite : le saut est préféré à
l'ajout d'une colonne, qui créerait une seconde source de vérité.

## `ranking` voyage avec

Ses deux routes portent `{competition_id}` et `{season_id}` — même compétition,
même saut, et `ranking_lines` n'a pas non plus de `space_id`. Le migrer
séparément voudrait dire écrire deux fois le même saut.



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
| `competitions` | colonne `space_id` — direct |
| `competition_seasons` | **saut** par `competition_id` → `competitions.space_id`, en une jointure |
| `ranking_lines` | même saut : `ranking` réutilise le résolveur saison, il n'écrit rien |

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
