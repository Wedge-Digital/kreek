# BC `team_creation` — cloisonnement des espaces

**Priorité : moyenne**
**Dépend de :** `324` (le middleware commun)
**Contexte :** `team_creation` — 25 routes

---

## Non sondé

Un seul fichier de `team_creation/io/web/` mentionne `space_id`, sans
comparaison. La sonde reste à faire.

## Les ressources

`team_drafts` et `team_roster_selections` portent toutes deux `space_id` :
**comparaison directe**, pas de saut.

## Un point de vigilance propre à ce BC

C'est ici que vivent les transactions de la carte 317 —
`team_roster_selections` était la table visible dans la transaction fantôme.
Migrer ces handlers veut dire les relire : si l'un d'eux ouvre une transaction
sur un chemin qui peut être annulé, c'est l'occasion de le noter, même si sa
correction relève de la 317.



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
| `team_drafts` | colonne `space_id` — direct |
| `team_roster_selections` | colonne `space_id` — direct |

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
