# Les coachs sont déconnectés trop souvent

**Priorité : haute** — plusieurs utilisateurs s'en plaignent
**Dépend de :** rien · **Sans épic**
**Remontée par :** l'utilisateur, depuis des retours de coachs

## Ce qui est mesuré

### 1. Le cookie meurt à la fermeture du navigateur

`main.rs:609` construit la couche sans configurer l'expiration :

```rust
let session_layer = SessionManagerLayer::new(DashMapStore::new());
```

Le défaut de `tower-sessions` 0.14 est `expiry: None`, et `service.rs:118` traite
ce cas ainsi :

```rust
Some(Expiry::OnSessionEnd) | None => cookie_builder,   // ni Max-Age ni Expires
```

Un cookie sans `Max-Age` est un **cookie de session** : le navigateur le supprime
dès qu'on le ferme.

**L'asymétrie est frappante.** Côté serveur, `tower-sessions-core` garde la
session **deux semaines** (`DEFAULT_DURATION = Duration::weeks(2)`,
`session.rs:22`). Le serveur se souvient quinze jours d'une session que le
navigateur a jetée le soir même.

### 2. Les sessions vivent dans la mémoire du processus

`src/common/session_store.rs` — un `DashMap`, dont l'en-tête dit lui-même :

> Drop-in replacement for MemoryStore in **dev/staging**.

**Chaque redéploiement déconnecte tout le monde, en même temps.** C'est ce que
le `CLAUDE.md` annonçait — *« Phase 1 : `MemoryStore`. Phase 2 : migration vers
`RedisStore` »* — et la phase 2 n'a pas eu lieu.

Deux effets secondaires : le magasin **ne nettoie jamais** — `load` filtre les
sessions expirées mais ne les supprime pas, donc la mémoire monte lentement — et
il est instancié dans `build_router`, ce qui lie sa durée de vie au routeur.

### 3. `SameSite: Strict` coupe les arrivées par lien externe

C'est le défaut de la crate (`service.rs:135`). Un cookie `Strict` **n'est pas
envoyé** quand on arrive depuis un lien extérieur — un e-mail, un message, un
lien partagé. Le coach atterrit déconnecté, se reconnecte, et conclut que « ça
déconnecte tout le temps ».

`Lax` couvre l'essentiel du CSRF pour un site qui ne fait pas de POST
inter-site — et le projet a déjà son propre middleware CSRF, qui rejette toute
mutation sans `HX-Request: true`.

## Ce qui n'est pas mesuré

**La fréquence des redéploiements.** Il n'y a aucun déploiement dans le dépôt —
`.github/workflows/` ne contient que `ci.yml`. Impossible de dire depuis ici si
la cause 2 pèse une déconnexion par mois ou trois par semaine.

**Laquelle des trois causes domine.** Rien ne le distingue aujourd'hui dans les
journaux : une session absente du magasin et un cookie jamais envoyé produisent
la même page de connexion.

**Ce que la production n'a pas** : `secure: true` n'est pas en cause, la démo
répond bien en HTTPS.

## Ce que cette carte livre, et ce qu'elle ne livre pas

**Elle livre les deux lignes de configuration** — celles qui coûtent le moins et
règlent probablement le plus :

```rust
SessionManagerLayer::new(DashMapStore::new())
    .with_expiry(Expiry::OnInactivity(time::Duration::days(30)))
    .with_same_site(SameSite::Lax)
```

`OnInactivity` et non `AtDateTime` : la fenêtre glisse à chaque requête, donc un
coach qui vient toutes les semaines n'est jamais déconnecté, tandis qu'un compte
inactif trente jours l'est. Une date fixe déconnecterait tout le monde le même
jour.

**Trente jours** parce que c'est un outil de ligue amateur : entre deux matchs il
se passe une à deux semaines, et se reconnecter à chaque journée de championnat
est exactement la plainte qu'on traite.

**Elle ne livre pas le magasin persistant** — c'est la carte 491. On mesure
d'abord ce que ces deux lignes font disparaître : si les plaintes cessent, la
cause 2 pesait peu ; si elles persistent par vagues corrélées aux déploiements,
elle pesait tout.

## Redis est écarté, définitivement

Le `CLAUDE.md` annonçait une « phase 2 : migration vers `RedisStore` ». La
décision est prise et le fichier est corrigé : **Postgres**, déjà là, déjà
sauvegardé, déjà surveillé. Redis ajouterait une infrastructure, un point de
panne et une sauvegarde de plus pour quelques centaines de sessions de ligue
amateur.

## Ce que la carte ne fait pas

**Elle ne touche pas au middleware CSRF.** Il est indépendant du cookie de
session, et `SameSite::Lax` ne l'affaiblit pas : toute mutation exige toujours
`HX-Request: true`.

**Elle ne change pas l'authentification.** `axum-login` et `AuthBackend` restent
tels quels ; c'est la durée et le transport du cookie qui changent, pas la
vérification de l'identité.

**Elle ne fait pas de « se souvenir de moi ».** Une case qui allongerait la
session pour certains est un autre sujet — et corriger d'abord pour tout le monde
évite d'avoir à le décider.

## Tests

Ils lisent l'en-tête **réellement émis** par le routeur de production, via le
harnais qui se connecte pour de vrai — pas la configuration censée le produire.

| Test | Ce qu'il prouve |
|---|---|
| `le_cookie_de_session_dure_au_dela_de_la_fermeture_du_navigateur` | `Max-Age` présent, et supérieur à vingt jours |
| `le_cookie_survit_a_une_arrivee_depuis_un_lien_exterieur` | `SameSite=Lax` |
| `le_cookie_reste_http_only_et_secure` | **la contre-épreuve** — assouplir `SameSite` n'assouplit pas le reste |

Le troisième existe parce qu'un futur `with_secure(false)` posé pour déboguer en
local partirait en production sans que rien ne proteste.

Falsifié quatre fois : `with_expiry` retiré, durée ramenée à un jour, `Strict`
rétabli, `secure` désactivé — chaque mutation rougit le test qui la vise, et lui
seul.

## Le harnais expose l'en-tête brut

`Harnais` ne gardait du `Set-Cookie` que `nom=valeur`, ce qu'il faut pour rejouer
la session. Les attributs — `Max-Age`, `SameSite`, `Secure` — sont précisément ce
qu'il fallait pouvoir lire : sans eux, un test ne distingue pas un cookie de
session d'un cookie qui dure trente jours.

## Checklist

- [x] `.with_expiry(Expiry::OnInactivity(30 jours))`
- [x] `.with_same_site(SameSite::Lax)`
- [x] Le `CLAUDE.md` dit Postgres, et dit pourquoi
- [x] `Harnais::set_cookie_brut()` pour lire les attributs
- [x] Les trois tests, falsifiés quatre fois
- [x] `make lint`, `make test` (1632), `make check-arch` (17 axes), `make e2e` (351, 0 échec)
- [ ] **Mesurer** : les plaintes cessent-elles ? Sinon, carte 491
