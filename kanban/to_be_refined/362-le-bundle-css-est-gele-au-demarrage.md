# Le bundle CSS est gelé au démarrage, et rien ne le dit

**Priorité : moyenne** — rien n'est cassé en production ; c'est la boucle de
développement qui ment, et elle a déjà coûté une heure et une conclusion fausse
**Dépend de :** rien — la carte 342 est livrée, celle-ci solde son effet de bord
**Fichiers :** `src/web/css_bundle.rs`, `src/main.rs`

## Le problème

Modifier une feuille de style n'a **aucun effet** sur le serveur de
développement qui tourne. Ni au rechargement, ni au vidage du cache. Il faut
redémarrer le serveur. Rien ne le signale — la page se recharge, le CSS servi
est l'ancien, et tout a l'air normal.

La cause est en trois lignes :

```rust
// css_bundle.rs:130
static BUNDLES: OnceLock<HashMap<&'static str, Bundle>> = OnceLock::new();

// css_bundle.rs:149
fn tous() -> &'static HashMap<&'static str, Bundle> {
    BUNDLES.get_or_init(|| { … })       // lit les 59 feuilles sur disque
}
```

`batir()` lit, concatène, minifie et empreinte. Une seule fois, à la première
demande — donc au démarrage, `construire()` étant appelée depuis `run_server`
(`main.rs:615`). Ensuite le contenu vit en mémoire jusqu'à l'arrêt du processus.
Aucune reconstruction, aucune surveillance de fichier, aucun mode développement.

## Ce n'est pas un défaut d'origine, c'est une propriété perdue

Avant la carte 342, `app-layout.html` liait chaque feuille sous `/static/css/…`,
servi par `ServeDir` depuis le disque. Éditer une feuille et rafraîchir
suffisait — comme sur n'importe quel projet web.

`/static` est **toujours** servi par `ServeDir` (`main.rs:588`), et les 59
feuilles y sont toujours accessibles à leur ancienne URL. Elles ne sont
simplement plus celles que les pages chargent. La propriété n'a pas été
supprimée : elle a été contournée, sans que personne le remarque.

C'est ce qui rend le piège efficace. L'habitude acquise reste vraie *ailleurs*
dans le projet — les templates rechargent, les fichiers de `/static` rechargent
— et fausse pour le seul CSS applicatif.

## Ce que le rechargement automatique ne rattrape pas

`main.rs:605` monte un `LiveReloadLayer` sous `#[cfg(debug_assertions)]`. Il ne
surveille pas les fichiers : il fait recharger le **navigateur** quand la
connexion tombe, c'est-à-dire quand le serveur redémarre. Il est donc taillé
pour une boucle `cargo watch`, où toute modification passe par une
recompilation.

Un fichier CSS ne déclenche aucune recompilation. Il tombe exactement dans
l'angle mort de ce dispositif.

## Ce que ça a coûté

Le 22/08/2026, pendant la carte 343 : une réservation de hauteur a été posée
sur `.menu-zone`, un test e2e s'est mis à échouer par intermittence, et la
réservation a été mise en cause. Elle a été retirée, remise, retirée. Le test
suivait, semblait suivre, ne suivait plus.

Le serveur n'avait pas redémarré. Le bundle servi n'a pas changé une seule fois
de toute la séquence. Les trois séries de mesures portaient sur le même CSS, et
la conclusion — « la réservation casse le test » — était sans fondement. Après
redémarrage, réservations effectivement servies et vérifiées dans le bundle par
`grep`, le test passe cinq fois sur cinq.

Le coût réel n'est pas l'heure perdue. C'est qu'une mesure avait l'air d'en
être une.

## L'empreinte est déjà journalisée — et ça n'a pas suffi

```rust
// main.rs:616
tracing::info!(duree_ms = …, app = %bundle("app").chemin, "bundles CSS construits");
```

La ligne existe, elle est juste, elle est au bon endroit. Elle n'a rien évité,
parce qu'elle est émise **au démarrage** alors que la surprise arrive une demi-
heure plus tard, et qu'il faut déjà soupçonner le bundle pour penser à la lire.

À retenir avant de choisir la solution : *journaliser davantage ne réglera pas
ce défaut-ci.* Le piège n'est pas un manque d'information, c'est un écart entre
ce que la boucle de développement promet et ce qu'elle fait.

## Les deux voies

**Reconstruire à chaque demande en `debug_assertions`.** Supprime le piège au
lieu de le signaler. Coût : 59 lectures disque, une concaténation, une
minification par page servie — à mesurer avant de trancher, le `duree_ms` de la
ligne ci-dessus donnant le chiffre dès le prochain démarrage. La minification
peut être sautée en développement si elle domine.

Difficulté à traiter, et c'est la vraie question de conception : `chemin_app()`
rend un `&'static str` et `bundle()` un `&'static Bundle`. Une reconstruction
doit donc soit fuiter (`Box::leak`, environ 300 ko par reconstruction), soit
changer ces signatures — ce qui touche `app-layout.html`, appelé depuis une
soixantaine de templates. Une fuite bornée par le nombre de rechargements d'une
session de développement est peut-être le prix juste ; ça se décide, ça ne se
subit pas.

**Surveiller `assets/static/css/` et invalider.** Plus économe à l'usage, plus
de pièces mobiles — une dépendance de surveillance, un fil, une invalidation
concurrente. Probablement disproportionné pour 59 fichiers dont la lecture
complète se compte en millisecondes.

La première voie est recommandée si la mesure la valide. Elle a la propriété
qui compte : *elle ne peut pas se désynchroniser*, là où une surveillance peut
manquer un événement et ramener le même piège sous une autre forme.

## Piège — ne pas rendre la production paresseuse

La construction au démarrage est **voulue** en production, et pour deux raisons
que la solution ne doit pas emporter : un échec de lecture ou de minification
doit être fatal tôt (`batir()` panique délibérément), et l'URL porte l'empreinte
du contenu, ce qui autorise le `immutable` posé sans réserve sur le cache
(`css_bundle.rs:257`). Un bundle qui change d'empreinte en cours de vie de
processus casserait cette garantie.

D'où le `#[cfg(debug_assertions)]` : deux comportements assumés, pas un
comportement paramétré.

## Vérification

Le contrôle est direct et doit être **vu échouer** avant d'être cru :

1. serveur lancé, relever l'empreinte servie dans le `<link>` d'une page ;
2. modifier une feuille du bundle de façon visible ;
3. recharger ; l'empreinte doit avoir changé et la modification être servie.

Sans le correctif, l'étape 3 rend la même empreinte — c'est l'état actuel, et
c'est la démonstration que le contrôle porte.

## Checklist

- [ ] Coût d'une reconstruction mesuré (`duree_ms` au démarrage), et décision
      documentée sur la minification en développement
- [ ] Voie retenue et justifiée entre reconstruction et surveillance
- [ ] Sort des `&'static` tranché : fuite bornée assumée par écrit, ou
      signatures changées
- [ ] Production inchangée : construction au démarrage, échec fatal, empreinte
      stable pour toute la vie du processus
- [ ] Contrôle des trois étapes vu échouer sans le correctif, puis passer avec
- [ ] `make lint`, `make check-arch`, `make test` passent
