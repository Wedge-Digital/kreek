# BC `match_report` — cloisonnement des espaces

**Priorité : haute** — aucune vérification, données de jeu
**Dépend de :** `324` (le middleware commun)
**Contexte :** `match_report` — 23 routes

---

## Non sondé, et pourtant classé haut

Aucune sonde n'a été tentée sur ce BC : un rapport de match est une donnée de
jeu, et une sonde d'écriture y aurait laissé des traces difficiles à défaire.

Le classement en priorité haute repose sur la lecture du code :
**aucun fichier de `match_report/io/web/` ne mentionne l'espace d'une
ressource**, exactement comme `competitions`, où la fuite a été prouvée.

C'est une déduction, pas une preuve. La sonde reste à faire au début de la
carte, sur une lecture — sans risque — pour confirmer avant de corriger.

## La ressource

`match_report_proj` porte `space_id` : **comparaison directe**, pas de saut.



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
| `match_report_proj` | colonne `space_id` — direct |

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
- [ ] Sonde de lecture préalable, pour confirmer la déduction du code
- [ ] Un scénario e2e de bout en bout
