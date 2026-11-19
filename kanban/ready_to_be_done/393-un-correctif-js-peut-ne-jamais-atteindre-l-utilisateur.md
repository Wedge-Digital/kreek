# Un correctif JS peut ne jamais atteindre l'utilisateur

> **⚠️ Cette carte demande ton attention.** Ce n'est pas une optimisation de
> cache : c'est un défaut de **livraison**. Un correctif JavaScript déployé peut
> rester sans effet pour qui est déjà venu, sans que rien ne le signale — ni
> côté serveur, ni côté navigateur, ni dans les tests.

**Priorité : haute** — silencieux, en production, et il a déjà mordu
**Dépend de :** rien
**Trouvée par :** un changement de rôle qui « ne se validait pas » dans
l'administration d'espace

## Le constat

Deux régimes coexistent pour les ressources statiques :

| Ressource | Nom servi | En-têtes |
|---|---|---|
| Bundle CSS | `kreek-app.a0661443f26e20bf.css` — empreinte du contenu | `cache-control: public, max-age=31536000, immutable` |
| Scripts | `/static/js/kreek-select.js` — **nom nu** | **aucun `cache-control`**, seulement `last-modified` |

La carte 342 a donné au CSS une empreinte, et c'est elle qui autorise
`immutable` : le nom change exactement quand le contenu change, donc le
navigateur reprend le fichier de lui-même.

Les cinq scripts sont restés sur `nest_service("/static", ServeDir::new(…))`
(`main.rs:643`), qui ne pose aucune directive de cache. Sans `Cache-Control`,
un navigateur applique sa **propre heuristique** à partir de `Last-Modified` :
il peut garder le fichier longtemps sans jamais revalider.

```
/static/js/alpine.min.js
/static/js/htmx.min.js
/static/js/htmx-ext-json-enc.js
/static/js/tom-select.complete.min.js
/static/js/kreek-select.js      ← le nôtre
```

## Ce que ça a coûté — mesuré, pas supposé

Le changement de rôle de l'administration d'espace était signalé comme
inopérant. Interrogé dans la page, avant tout rechargement :

```js
customElements.get('kreek-select').prototype._selectItem.toString()
  → ne contient pas dispatchEvent
```

La classe **chargée en mémoire** n'avait pas l'émission du `change`, alors que
le fichier servi par le serveur, lui, l'avait : le navigateur exécutait une
copie mise en cache d'avant le correctif.

Conséquence exacte : au clic, le champ caché passait bien à `SpaceAdmin` et le
libellé affichait « Admin » — **aucune requête ne partait**, le compteur
d'administrateurs ne bougeait pas. L'écran affichait un changement qui n'avait
pas eu lieu.

Après rechargement forcé, même geste :

```
trafic: ["200 …/role", "200 widgets/stats"]      administrateurs : 1 → 2
```

**Ce qui rend le défaut dangereux, c'est que `kreek-select` met son affichage à
jour localement.** La panne est donc invisible : l'utilisateur voit le rôle
changer. C'est le piège que le commentaire de `_selectItem` décrit déjà — il a
mordu une seconde fois, par un chemin que personne n'avait prévu.

## Le correctif retenu — un en-tête, et rien d'autre

```rust
.nest_service(
    "/static",
    ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ))
        .service(ServeDir::new("assets/static")),
)
```

**`no-cache`, et non `no-store`.** `no-cache` n'interdit pas de garder une
copie : il **oblige à revalider** avant de la servir. Le navigateur envoie sa
requête conditionnelle, le serveur répond `304 Not Modified` — quelques octets
— et la copie locale est réutilisée. `no-store` interdirait toute copie et
retéléchargerait `tom-select` en entier à chaque page, pour rien.

**Ce que ça coûte** : une requête conditionnelle par ressource et par
chargement complet de page. En navigation HTMX il n'y en a aucune, les scripts
n'étant pas rechargés. C'est le prix d'un correctif dont on est sûr qu'il
arrive.

**Ce que ça couvre** : les cinq scripts, mais aussi les images et les polices de
`/static`, qui portent le même défaut — moins grave, puisqu'une image périmée se
voit et qu'un script périmé, non.

**Une feature à activer** : `Cargo.toml:17` déclare
`tower-http = { version = "0.6.2", features = ["fs", "catch-panic"] }`.
`SetResponseHeaderLayer` demande **`set-header`**. À défaut, un
`axum::middleware::from_fn` de cinq lignes pose le même en-tête sans toucher aux
dépendances — le choix se fait à l'implémentation, il n'engage rien.

## Ce que l'instruction a répondu aux questions du raffinage

**Les quatre bibliothèques tierces sont couvertes gratuitement** par l'en-tête,
sans qu'on ait à décider si elles « changent assez souvent ». C'était la seule
réponse qui n'oblige personne à s'en souvenir le jour où htmx passe en 2.1.

**Un test peut constater le défaut, et il sera mécanique.**
`src/web/test_harness.rs` expose `Reponse::entete(nom)` : demander
`/static/js/kreek-select.js` et exiger un `cache-control` tient en cinq lignes,
tourne dans `make test`, et bloque la régression. Aucun test de navigateur ne
peut voir un cache ; celui-ci le peut.

**Le rendu HTML n'a rien à connaître.** Les scripts restent référencés par leur
nom nu, dans les **trois seuls layouts** qui en chargent :
`web/templates/app-layout.html` (cinq scripts), `widget-tester-layout.html`
(quatre), `auth/io/web/templates/auth-layout.html` (htmx seul). Aucun autre
`<script src>` dans les 144 templates.

## Pourquoi l'empreinte n'est pas dans cette carte

Elle reste souhaitable — le mécanisme existe, `css_bundle.rs` lit, minifie,
empreint et sert avec `immutable`, et le template l'appelle par une fonction
libre (`{{ crate::web::css_bundle::chemin_app() }}`), donc sans toucher une
seule struct. Onze références seulement seraient à changer.

Mais elle est écartée ici pour trois raisons :

1. **Ce n'est pas le même enjeu.** L'en-tête corrige un défaut de livraison ;
   l'empreinte économise du trafic. Les mêler ferait dépendre une correction
   d'une optimisation.
2. **Elle traînerait la carte 362.** Un JS servi depuis la mémoire serait
   « gelé au démarrage » comme le bundle CSS l'est déjà — et sans que rien ne
   le dise.
3. **`cargo watch` ne surveille pas `assets/static/js`** (`Makefile:84` et
   `:92` : `src`, `Cargo.toml`, `assets/templates`, `assets/static/css`).
   Aujourd'hui, éditer un script et rafraîchir suffit, `ServeDir` relisant le
   disque. Avec l'empreinte en mémoire, l'édition n'aurait plus aucun effet,
   sans redémarrage ni signal — on remplacerait un piège de production par un
   piège de développement.

À reprendre dans une carte propre, avec la 362.

## Ce que la carte ne couvre pas

**Le correctif de `kreek-select`**, qui est bon et déjà livré. Rien à corriger
dans le composant.

**L'empreinte des ressources statiques**, pour les raisons ci-dessus.

## Checklist

- [ ] `cache-control: no-cache` posé sur `/static`, par `SetResponseHeaderLayer`
      (feature `set-header`) ou par un `from_fn` maison
- [ ] Test d'en-têtes dans `src/web/tests/` : `/static/js/kreek-select.js` rend
      un `cache-control`, et il vaut `no-cache`
- [ ] Le même test sur une ressource non-JS — une image — pour verrouiller que
      la directive couvre tout `/static`
- [ ] Vérifié à la main sur le serveur de développement : deuxième chargement
      d'une page, le script part en `304` et non en `200 (from disk cache)`
- [ ] `make lint`, `make check-arch`, `make test`
