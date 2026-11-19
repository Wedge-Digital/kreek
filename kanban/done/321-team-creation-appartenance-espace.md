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

- [x] `ISpaceOwnership` pour ce BC, enregistré dans `main.rs`
- [x] Tests de handler : matrice d'appartenance, lecture et écriture
- [x] Sonde de lecture préalable, pour confirmer la déduction du code
- [x] Suite e2e complète verte — 183 passés, c'est elle qui a compté ici

## Cette carte a d'abord réparé une régression que la 320 avait causée

`{team_id}` est revendiqué par **deux BCs pour deux choses différentes** :
`teams` y voit une équipe enrôlée dans `team_proj`, `team_creation` un brouillon
dans `team_drafts`. Un brouillon n'entre dans la projection qu'à sa soumission.

Le résolveur de la carte 320 ne lisait que la projection : **tous les brouillons
non soumis rendaient `404`**, et la création d'équipe était cassée — 47
brouillons concernés sur la base de développement.

`make test` passait (997 ✓) : aucun test unitaire ne construit d'équipe par les
routes. **Seule la suite e2e l'aurait vu**, et elle n'avait pas été lancée après
la 320. C'est la vérification sautée, pas le code, qui a laissé passer.

### Le correctif, et le garde-fou qui va avec

`TeamSpaceOwnership` consulte désormais **les deux sources**, projection puis
brouillons.

Et le middleware **refuse au démarrage** que deux résolveurs revendiquent le
même paramètre : le second ne serait jamais consulté, et le premier déciderait
pour un BC qui l'ignore. Un doublon devient une erreur de câblage bruyante au
lieu d'un arbitrage silencieux.

C'est le revers, non vu à la carte 324, de la liste plate par ressource : elle
fait qu'un BC bénéficie des résolveurs des autres — et qu'il en subit les
lacunes.

## Une fixture e2e qui testait une compétition fantôme

`competition_rules_url` repliait `competition_id` et `season_id` sur le
`space_id`. L'URL désignait donc une compétition qui n'existait nulle part, et
la page se rendait quand même : les cinq tests de `test_phase2_pickers`
vérifiaient le rendu des pickers **sur une compétition inexistante**.

Le contrôle d'appartenance a mis fin à cette tolérance, à juste titre. La
fixture crée maintenant un vrai brouillon, et le supprime après.

**Ici le test avait tort et le middleware raison** — contrairement aux
brouillons d'équipe, où c'était l'inverse. La distinction valait d'être faite
avant de corriger : en base, zéro saison orpheline, donc le parcours réel
travaille toujours sur une compétition qui existe.
