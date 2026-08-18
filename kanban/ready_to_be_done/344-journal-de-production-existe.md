# `web` — En production, le journal de requêtes n'existe pas

**Priorité : haute** — c'est un correctif, pas une amélioration
**Dépend de :** rien
**Fichiers :** `src/main.rs`, `src/web/middleware/request_log.rs`

## Le problème

Il y a deux journaux de requêtes dans l'application. Selon le build, il t'en
reste un ou zéro.

**`request_log` est compilé hors du binaire release.** Il n'est branché qu'à
l'intérieur du bloc `#[cfg(debug_assertions)]` de `build_app` (`main.rs:522`),
au milieu du câblage de `tower-livereload` — un voisinage de développement dont
il a hérité la condition, vraisemblablement sans que ce soit voulu.

**`TraceLayer::new_for_http()` (`main.rs:501`) est muet.** Il émet sur la cible
`tower_http::trace`, or le filtre par défaut est `kreek=debug` : une directive
ciblée n'active *que* cette cible. Le `TraceLayer` ne produit donc rien, ni en
développement ni en production.

Conséquence : **en production, chaque ligne du journal est un `error!` isolé** —
sans méthode, sans chemin, sans statut, sans utilisateur. On ne peut même pas
savoir quelle requête l'a provoqué. C'est l'essentiel de la douleur constatée.

Troisième défaut, plus discret, dans `request_log` lui-même :

```rust
let thread = std::thread::current().id();
tracing::info!(?thread, %method, %path, "→ requête reçue");
```

En async Tokio, l'identifiant de thread **ne corrèle rien** : une tâche migre
d'un thread à l'autre entre deux `await`, et des requêtes concurrentes
partagent le même thread. Deux lignes portant le même `thread` n'appartiennent
pas à la même requête, et deux lignes d'une même requête peuvent porter des
`thread` différents. C'est pire qu'une absence d'information : ça en simule
une.

## Ce qu'il faut faire

**Un seul journal de requêtes, le nôtre, toujours actif.**

- Sortir `request_log` du bloc `#[cfg(debug_assertions)]` et le brancher dans
  le chemin commun, pour tous les builds.
- Retirer `TraceLayer::new_for_http()`. Le garder supposerait d'ouvrir
  `tower_http` dans le filtre pour obtenir un doublon de ce que `request_log`
  produit déjà, en moins bien : notre middleware sait ce qu'on veut y mettre.
- Retirer `?thread`. Son remplaçant — un identifiant de requête — arrive avec
  la carte 345 ; d'ici là, mieux vaut aucun champ de corrélation qu'un champ
  qui ment.

Attention à l'ordre des couches : `request_log` doit envelopper le traitement
au plus près de l'extérieur, pour voir aussi les requêtes rejetées par
`require_auth` ou `space_scope_middleware`. Une requête refusée est
précisément celle qu'on cherche à comprendre.

## Ce que cette carte ne fait pas

Elle ne change ni le contenu ni le niveau des lignes. Pas de `rid` (carte 345),
pas de durée (carte 345), pas de reclassement des 198 `error!`. Elle rétablit
l'existence du journal, rien de plus — et c'est déjà l'écart le plus grand du
lot.

## Checklist

- [ ] `request_log` actif en release comme en debug
- [ ] `TraceLayer` retiré, et l'import correspondant avec
- [ ] `?thread` retiré
- [ ] Vérifié sur un build release local : une requête produit bien ses deux
      lignes, avec méthode, chemin et statut
- [ ] Vérifié qu'une requête rejetée par un middleware d'authentification
      apparaît elle aussi
- [ ] `make lint` et `make check-arch` passent
