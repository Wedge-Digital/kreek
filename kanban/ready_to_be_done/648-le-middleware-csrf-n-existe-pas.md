# Le middleware CSRF n'existe pas

**Épic :** aucune · **Dépend de :** rien
**Priorité : haute** — `CLAUDE.md` décrit une garde que personne n'a écrite, et
tout le code est relu en la croyant en place.
**Trouvée par :** le cadrage de E23, `docs/specs/saisie-mobile/E23-cadrage.md` (§ 1 « L'authentification d'une requête longue », § 12).

## Le constat

`CLAUDE.md`, « Middleware — ordre d'exécution », décrit :

```
Request → TraceLayer → SessionLayer → AuthLayer → CsrfMiddleware → login_required! → Handler
```

et ajoute que « le middleware CSRF rejette les POST/PUT/DELETE/PATCH sans
header `HX-Request: true`, sauf `/login` et `/logout` ».

**Aucun fichier ne le définit.** `src/web/middleware/` contient
`bypass_auth.rs`, `panic_response.rs`, `request_log.rs`, `require_auth.rs`,
`space_scope.rs` ; `build_router` (`src/main.rs:632-741`) n'en pose aucun.
Seuls deux commentaires en parlent, au présent : `src/main.rs:656` (« la garde
qui compte ici ») et `src/web/test_harness.rs:134-138` (`post_htmx`, « l'en-tête
`HX-Request` que le middleware CSRF exige »). `TraceLayer` n'est pas branché
non plus (carte 344).

**Ce qui protège réellement les mutations** : le cookie de session en
`SameSite=Lax` (`main.rs:658-660`, carte 490). Un navigateur ne l'envoie pas sur
un `POST` venu d'un autre site ; il l'envoie sur une navigation `GET` de premier
niveau.

## Ce que le code envoie aujourd'hui

**Le JS suit déjà la convention.** Les quinze `fetch` en écriture envoient
`HX-Request: true` : `finalize-team.html:161`, `:171`,
`spp-spending-panel.html:52`, `new-competition-phase-2.html:420`,
`phase-3.html:292`, `phase-4.html:266`, `admin/groups.html:79`,
`settings-tiers.html:137`, `settings-ranking.html:226`,
`schedule-round-detail.html:31`, `:70`, `:83`, `:146`, `:183`,
`teams-costly-mistakes.html:51`. Les `htmx.ajax('POST', …)`
(`action-panel-widget.html:81`, `:107`, `space-admin-create-coach.html:21`) et
les `hx-post` le posent d'eux-mêmes. Les formulaires d'`auth` sont en `hx-post`
(`auth-login.html:10`, `auth-register.html:8`…).

**Six formulaires classiques ne l'envoient pas**, tous dans `match_report` :
`inducements.html:115`, `pre-match.html:94`, `match-selection.html:44`,
`match-mode.html:20`, `recap.html:216` (la publication), `step5.html:67`. Ce
sont des `<form method="post">` soumis par le navigateur, suivis d'un `303`.

**Les tests e2e** : sur 187 appels `requests.post|put|delete`, environ quatre-vingts
n'envoient pas l'en-tête (par exemple `test_cuistot_halfling.py:42`, qui poste
l'avant-match comme le formulaire).

**La déconnexion est un `GET`** (`src/app/auth/router.rs:34`,
`app-menu.html:19`) : `SameSite=Lax` l'envoie depuis n'importe quel lien
externe. L'exception « sauf `/logout` » de `CLAUDE.md` vise une route qui n'est
pas un `POST`.

**E23 ajoute un `POST`** — la présence (carte 644), envoyé par `fetch` avec
`HX-Request: true`.

## La décision à prendre

1. **Écrire le middleware tel que `CLAUDE.md` le décrit** (`HX-Request: true`
   exigé). Casse les six formulaires de `match_report` — à convertir en
   `hx-post` — et environ quatre-vingts appels e2e. L'en-tête est un faible signal :
   il ne prouve rien d'autre qu'une requête non « simple », ce que `SameSite`
   couvre déjà pour un navigateur.
2. **Écrire un middleware sur `Origin` / `Sec-Fetch-Site`** : refuser une
   mutation dont `Sec-Fetch-Site` vaut `cross-site`, ou dont l'`Origin` est
   présent et différent de l'hôte ; laisser passer une requête sans ces
   en-têtes, qui ne vient pas d'un navigateur. Ne casse ni les formulaires ni
   les e2e, couvre les sous-domaines que `SameSite` ne distingue pas.
3. **Ne rien écrire et corriger `CLAUDE.md`** : la garde réelle est
   `SameSite=Lax`, et le texte, comme les deux commentaires, le dit.

**Recommandation : 2, et `CLAUDE.md` corrigé pour décrire ce qui existe** —
une défense en profondeur qui ne casse rien, là où la première forme coûterait
une conversion de six formulaires pour une garantie plus faible. La
déconnexion en `GET` est à trancher dans la même carte : la passer en `POST`,
ou l'assumer par écrit.

## Ce qu'il faut faire

Une fois la décision prise :

- **Option 2** : `src/web/middleware/same_origin.rs`, posé en `route_layer` à
  côté de `bypass_auth` ; méthodes `POST`, `PUT`, `PATCH`, `DELETE` ; refus en
  `403` journalisé en `warn` sous `kreek::web::middleware` (méthode, chemin,
  `Sec-Fetch-Site`, `Origin`). L'hôte attendu vient de `cfg.app_url()`.
- **Dans tous les cas** : `CLAUDE.md` (« Middleware — ordre d'exécution »,
  « Sessions ») et les commentaires de `main.rs:656` et `test_harness.rs:134-138`
  décrivent ce qui existe.

## Fichiers touchés

Option 2 : `src/web/middleware/same_origin.rs` (nouveau), `src/web/middleware/mod.rs`,
`src/main.rs`, `src/web/tests/`. Dans tous les cas : `CLAUDE.md`, `src/main.rs`,
`src/web/test_harness.rs`.

## Tests

- **Unitaires**, par le harnais : un `POST` sans `Origin` passe ; un `POST`
  d'`Origin` étranger est refusé en `403` ; `Sec-Fetch-Site: cross-site` est
  refusé, `same-origin` passe ; un `GET` n'est jamais regardé.
- **E2E** (`tests/e2e/test_garde_inter_sites.py`) : une page servie depuis une
  autre origine qui soumet un formulaire vers `/app/…/recap/publish` est
  refusée ; les six formulaires classiques de `match_report` fonctionnent
  toujours.
- `tests/impact-map.toml` mis à jour.

## Hors périmètre

- Un jeton CSRF par formulaire.
- La persistance des sessions : carte 491.

## Conséquences sur les données de production

Aucune donnée. Option 2 : une requête légitime d'un navigateur ne change pas ;
un client qui enverrait un `Origin` d'une autre adresse que `bloodbowlclub.com`
— derrière un mandataire qui réécrirait l'hôte — serait refusé : vérifier
que nginx transmet `Host` tel quel (`include proxy_params`).

## Terminé quand

- [ ] La décision est prise et écrite dans cette carte
- [ ] `CLAUDE.md` et les deux commentaires décrivent la garde qui existe réellement
- [ ] Option 2 : un formulaire soumis depuis une autre origine est refusé, les formulaires de `match_report` passent
- [ ] `make lint`, `make check-arch`, `make test`, e2e impactés
