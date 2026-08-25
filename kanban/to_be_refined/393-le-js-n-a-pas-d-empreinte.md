# Le JS n'a pas d'empreinte, et un correctif peut ne jamais atteindre l'utilisateur

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

## Ce qui est à trancher

**Empreindre les scripts comme le CSS.** Le mécanisme existe déjà et
fonctionne : `css_bundle.rs` lit, minifie, empreint, et sert avec `immutable`.
La question est de savoir si le JS rejoint ce bundle ou reçoit le sien.

**Ou, au minimum, poser un `Cache-Control` explicite** sur `/static` —
`no-cache` oblige à revalider à chaque fois, ce qui ferme le trou au prix d'une
requête conditionnelle. C'est le correctif d'une ligne, sans empreinte.

Le second suffit à supprimer le défaut ; le premier supprime aussi les
requêtes. Ne pas confondre les deux enjeux : ici, la correction est **exacte**,
pas la performance.

## Ce que la carte ne couvre pas

**Le correctif de `kreek-select`**, qui est bon et déjà livré. Rien à corriger
dans le composant.

**Les autres ressources de `/static`** — images, polices — qui portent le même
défaut sans la même gravité : une image périmée se voit, un script périmé ne se
voit pas.

## Questions à trancher au raffinement

- Les quatre bibliothèques tierces (`alpine`, `htmx`, `htmx-ext-json-enc`,
  `tom-select`) changent rarement : méritent-elles le même traitement que notre
  script, ou un cache long assumé, leur version étant figée ?
- Un test peut-il constater le défaut ? Un test d'en-têtes HTTP sur `/static/js`
  serait mécanique et bloquant, là où aucun test de navigateur ne peut voir un
  cache.
- Le rendu HTML devra-t-il connaître le nom empreint des scripts, comme il
  connaît déjà celui du bundle CSS ? C'est ce qui décide de l'ampleur du
  chantier.
