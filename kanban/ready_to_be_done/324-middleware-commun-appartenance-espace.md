# Middleware commun — une ressource n'est atteignable que depuis son espace

**Priorité : haute** — prérequis des cartes 318 à 322
**Contexte :** `src/web/middleware/`, un port par BC

## L'objectif, en une phrase

Empêcher qu'un admin de l'espace A lise ou modifie une ressource de l'espace B
en mettant son propre espace dans l'URL.

C'est une comparaison — *la ressource demandée appartient-elle à l'espace du
chemin ?* Non → `404`. Tout le reste est de la plomberie, et cette carte existe
pour que la plomberie ne soit écrite qu'une fois.

## Pourquoi commun, et pourquoi maintenant

**Un seul BC contrôle quoi que ce soit aujourd'hui** : `players`, six sites
d'appel, issus de la carte 315. Les sept autres ne contrôlent rien.

Rien n'est donc figé. Mettre le mécanisme en commun maintenant convertit **un**
BC ; le faire après les cartes 318-322 en convertirait six.

Ce qui varie d'un BC à l'autre est mince — quel paramètre de chemin désigne une
ressource, et comment remonter à son espace. Tout le reste est identique.
Écrite cinq fois, cette sémantique divergerait : un `403` ici, un `404` là, un
identifiant mal formé traité en `500` ailleurs. Un garde qui diverge est un
garde inutile, et on ne s'en aperçoit pas.

## La forme

```rust
// src/web/middleware/space_scope.rs — le mécanisme, une seule fois
#[async_trait]
pub trait ISpaceOwnership: Send + Sync {
    /// Le paramètre de chemin que ce résolveur sait traiter.
    fn param(&self) -> &'static str;          // "competition_id", "team_id", …
    /// À quel espace appartient cette ressource ? `None` si elle n'existe pas.
    async fn space_of(&self, id: &str) -> Option<SpaceId>;
}
```

Le middleware lit les paramètres du chemin, retient ceux qu'un résolveur
connaît, pose la question, compare. Chaque BC répond **sur ses propres
ressources, via son propre repository** — la souveraineté des données est
préservée, ce qu'un middleware interrogeant les tables directement violerait.

`main.rs` enregistre les résolveurs. Ajouter un BC devient : implémenter un
trait, l'enregistrer. Pas un middleware de plus.

### Lire les paramètres depuis un middleware

Technique déjà employée par
`spaces/io/web/extractors/space_permissions.rs` :

```rust
let Path(params) = Path::<HashMap<String, String>>::from_request_parts(parts, state).await?;
```

## Sémantique, à figer ici et nulle part ailleurs

| Situation | Réponse |
|---|---|
| Espace du chemin mal formé | `400` |
| Ressource inexistante | `404` |
| Ressource d'un autre espace | `404` |
| Aucun paramètre connu dans le chemin | laisse passer |

**`404` et non `403`** pour l'espace étranger : un `403` confirmerait
l'existence de la ressource à qui l'énumère. Pour lui, elle n'existe pas.

**Le contrôle vient avant l'autorisation** : il ne s'agit pas de savoir qui a le
droit, mais de quoi on parle.

## Pourquoi un middleware plutôt qu'un appel dans chaque handler

`competitions` compte 44 routes réparties sur 21 fichiers, dont **huit
handlers qui reçoivent l'espace sous un nom souligné** — donc explicitement
ignoré, comme `player_debug_controller` avant la carte 315. Le compilateur ne
dira jamais rien sur un paramètre qu'on choisit d'ignorer.

Un appel par handler, c'est 21 fichiers à toucher et un oubli possible à chaque
handler ajouté. Le middleware couvre les routes futures sans qu'on y pense.

**Le prix, à dire** : le contrôle n'est plus lisible dans le handler. La parade
est l'axe de la carte 323 — vérifier qu'un BC exposant
`/app/{space_id}/…/{ressource_id}` a bien un résolveur enregistré, ce qui se
grep autrement mieux qu'un appel dans chaque handler.

## `players` migre, et sert de premier cas

Son garde sur mesure — `players/io/web/space_scope.rs`, six sites d'appel —
disparaît au profit d'un `ISpaceOwnership`.

C'est du code qui marche et qui est testé, donc un retouchage gratuit en
apparence. Il vaut le coup : **deux mécanismes concurrents pour la même règle,
c'est ce qui produit les angles morts**. Et ses tests de handler existants
(`players/io/web/tests/test_space_scope.rs`) doivent passer sans modification —
c'est la meilleure preuve que la migration ne change rien au comportement.

---

## Checklist

- [ ] `ISpaceOwnership` et le middleware, dans `src/web/middleware/space_scope.rs`
- [ ] Sémantique figée : `400` / `404` / laisser passer
- [ ] Enregistrement des résolveurs dans `main.rs`
- [ ] Résolveur `players`, et suppression de son garde sur mesure
- [ ] **Les tests de `test_space_scope.rs` passent sans être modifiés**
- [ ] Tests de handler du mécanisme lui-même : paramètre inconnu → passe,
      ressource absente → `404`, espace mal formé → `400`

## Point de vigilance

Le middleware doit être posé **là où `bypass_auth` l'est** — en `route_layer`
sur le routeur protégé, donc à l'intérieur d'`AuthManagerLayer`. La carte 311 a
appris ça à ses dépens : une couche posée par-dessus le routeur s'exécute
*avant* l'authentification et ne voit pas la session.

Ici la session n'est pas nécessaire — le contrôle ne dépend pas de l'identité —
mais l'ordre reste à choisir consciemment, et non par défaut.
