# Les actions d'équipe se gardent elles-mêmes

**Priorité : haute — écriture ouverte en production**
**Dépend de :** rien ; se livre avec la carte 500, qui n'a de sens qu'avec elle
**Fichiers :** `src/app/teams/io/web/garde_action_equipe.rs` (nouveau),
`src/app/teams/router.rs`, `src/app/teams/io/web/mod.rs`, `src/main.rs`,
`src/app/teams/io/web/costly_mistakes.rs`,
`src/app/teams/io/web/tests/test_garde_action_equipe.rs` (nouveau)

## Le défaut

Un coach simple, membre du même espace, peut aujourd'hui **recruter des
joueurs sur l'équipe d'un autre** et **clore ses phases de jeu**, en tapant
l'URL. Rien ne l'en empêche.

`grep peut_modifier_effectif src/app/teams` ne rend que deux appels :
`team_detail.rs` (l'affichage) et `costly_mistakes.rs` (le POST du jet).
Toutes les autres actions sont ouvertes :

| Route | Garde actuelle |
|---|---|
| `POST validate-improvement-phase` | aucune — **pas même `AuthSession`** |
| `POST validate-recruitment-phase` | aucune — idem |
| `POST validate-dismissals-phase` | aucune — idem |
| `GET recruitment` + ses 2 widgets | phase seulement |
| `POST recruitment/players/{add,remove}` | phase seulement |
| `POST recruitment/staff/{add,remove}` | phase seulement |
| `GET dismissals` + ses 2 widgets | phase seulement |
| `POST dismissals/players/{mark,unmark}` | phase seulement |
| `POST dismissals/staff/{mark,unmark}` | phase seulement |
| `GET costly-mistakes` | session seulement |
| `POST costly-mistakes/roll` | **complète** — le modèle à recopier |

`space_scope_middleware` ne comble rien : il vérifie qu'une ressource
appartient bien à l'espace de l'URL, pas que le demandeur ait un droit dessus.

C'est la différence avec la carte 389, dont le commentaire de tête de
`roster_edit_access_service` dit « masquer un bouton n'est pas un contrôle
d'accès — l'écriture reste gardée par `can_spend_spp` ». C'était vrai de
l'édition d'effectif. Ça ne l'est d'aucune de ces onze routes.

## La décision : un garde de groupe de routes, pas dix-huit gardes de handler

Poser la garde en tête de chaque handler demanderait dix-huit modifications, et
les handlers de mutation n'offrent même pas de point de passage commun : `add_player`
appelle `basket_mutation::add_player` **avant** de rendre le fragment, donc
avant de toucher `charger()`. Une garde posée dans `charger()` refuserait
l'affichage d'un panier déjà modifié.

Surtout, dix-huit gardes recopiées sont dix-huit occasions d'en oublier une —
et le prochain widget de recrutement naîtrait ouvert. C'est le même
raisonnement que le `retain` de la carte 500 : **on garde par construction, pas
par vigilance.**

### Le middleware

`src/app/teams/io/web/garde_action_equipe.rs`, sur le patron de
`space_scope_middleware` — extraction des paramètres de chemin, décision,
`next.run` :

```rust
pub async fn garde_action_equipe(
    State(state): State<AppState>,
    auth_session: AuthSession,
    request: Request,
    next: Next,
) -> Response
```

Il lit `team_id` dans le chemin, charge l'équipe, appelle
`roster_edit_access_service::peut_modifier_effectif` — **la fonction existante,
inchangée** — et refuse par `403` sinon. Pas une cinquième copie de la règle :
un appelant de plus.

Codes de retour, alignés sur `post_costly_mistakes_roll` :

| Cas | Réponse |
|---|---|
| pas de session | `401` (`require_auth` l'a normalement déjà traité) |
| `team_id` illisible | `400` |
| équipe inconnue | `404` |
| ni propriétaire ni administrateur | `403`, avec une ligne `warn` nommant l'équipe et le coach |

### Le groupe de routes

`teams/router.rs` sépare ce qu'il expose en deux :

```rust
pub fn router(state: AppState) -> Router<AppState> {
    routes_ouvertes().merge(
        routes_d_action().route_layer(from_fn_with_state(state, garde_action_equipe)),
    )
}
```

Dans le groupe gardé : les trois `validate-*-phase`, les deux `costly-mistakes`,
les sept routes de recrutement, les sept routes de renvois.

**Hors du groupe, et c'est délibéré :**

- `TEAM_DETAIL`, `TEAM_TREASURY`, `TEAM_MATCHES` — la fiche se lit par tout le
  monde ; c'est exactement ce que la carte 500 préserve ;
- `DISMISS_TEAM` et les actions d'inscription — règle différente, celle du
  commissaire : `SpacePermissions::is_admin()`, déjà en place, et qui **exclut**
  le propriétaire ;
- les widgets de liste (`my-teams`, `enrolled`, `pending`, `team-selection`) —
  de la lecture.

### Le coût assumé

`router()` prend un paramètre, là où les dix autres BCs exposent un
`router() -> Router<AppState>` uniforme. C'est une ligne dans `main.rs` et une
divergence visible ; `from_fn_with_state` exige une valeur d'état à la
construction, et il n'y a pas de moyen de la lui donner plus tard sans perdre
le groupement par routes qui fait tout l'intérêt du dispositif.

Le middleware charge l'équipe, que le handler rechargera. `post_costly_mistakes_roll`
le fait déjà pour la même raison, et un `find_by_id` sur une projection indexée
ne pèse pas devant les allers-retours que la page fait déjà.

### Ce que `get_costly_mistakes_page` perd

Sa vérification de session en propre devient redondante et part avec — le
middleware la couvre, et deux gardes pour une question invitent à en corriger
une seule. `post_costly_mistakes_roll` perd de même son bloc
`peut_modifier_effectif`, désormais tenu en amont (règle 4 : le comportement
supprimé doit être couvert par le nouveau code avant le commit).

## Ce que la carte ne fait pas

- Elle ne change **aucune** règle de droit : `peut_modifier_effectif` est
  reprise telle quelle, avec ses trois questions et leur ordre.
- Elle ne touche pas aux actions de commissaire, qui relèvent de
  `SpacePermissions` et dont l'exclusion du propriétaire est un choix de
  conception.
- Elle ne touche pas aux BCs voisins : `players` garde `can_spend_spp`,
  `match_report` garde ses propres contrôles.

## Checklist

- [x] `garde_action_equipe.rs`, déclaré dans `io/web/mod.rs`
- [x] `router(state)` scindé en `routes_ouvertes()` / `routes_d_action()`
- [x] `main.rs` passe l'état
- [x] `costly_mistakes.rs` : les deux gardes locales retirées, couvertes en amont
- [x] Les **dix-huit** routes frappées par un membre simple → `403`
- [x] **Contre-épreuve sur chacune** : les mêmes dix-huit frappées par un ayant
      droit → tout sauf `403` et `401`. Sans elle, un `403` dû à une URL fautive
      se lirait comme un refus d'autorisation — le piège que
      `test_roster_edition.py:355` documente déjà.
- [x] Un troisième test tient l'autre moitié : la fiche, sa trésorerie et ses
      matchs restent `200` pour un tiers
- [x] `make lint`, `make check-arch`, `make test` — 1653 tests
- [x] `make e2e` — 356 passés, 7 ignorés : le découpage du routeur n'a fermé
      aucune route à qui y avait droit

## Ce qui a été fait

**Le test a été vu échouer.** Le garde neutralisé, `un_coach_tiers_…` tombe sur
la première route — `GET recruitment`, `200` pour un membre simple : le défaut
que la carte décrit, reproduit.

### Le harnais plutôt que l'e2e, et pourquoi

La carte annonçait « tests unitaires du middleware » plus un e2e par famille.
Les deux ont été remplacés par **trois tests de harnais**
(`io/web/tests/test_garde_action_equipe.rs`), et c'est un meilleur marché :

- Un test unitaire du middleware aurait vérifié la décision, que
  `peut_modifier_effectif` couvre déjà par six tests — lesquels **passaient
  quand le défaut était là**. Ce qui manquait n'était pas la règle, c'était son
  câblage. Le harnais monte le routeur de production et frappe les dix-huit
  routes : un oubli de câblage s'y voit, pas ailleurs.
- L'e2e n'aurait rien ajouté : le harnais existe précisément pour les matrices
  d'autorisation, et sa raison d'être est d'éviter d'y faire monter ce qui n'a
  ni HTMX, ni Alpine, ni CSS à éprouver. Un `403` n'en a aucun.

L'équipe de test est semée **par le dépôt**, non par un `INSERT` : `find_by_id`
hydrate depuis l'event store, et une équipe posée en projection seule aurait
rendu `404` — l'assertion du tiers serait tombée rouge pour une raison
étrangère, et sa contre-épreuve verte sans rien prouver. C'est l'écueil que
`test_space_scope.rs` documente déjà.

L'ayant droit du test est **administrateur de l'espace sans être propriétaire** :
la propriété court-circuite les deux autres questions, et l'exercer n'aurait
rien prouvé du câblage des ports.

## Le test qui compte

Celui qui, pour chaque route gardée, **prouve que la requête aurait abouti**.
Un test qui n'assène qu'un `403` passe aussi bien quand la route n'existe pas,
quand le `space_id` est mal formé, ou quand la phase ne s'y prête pas. Les
trois se sont produits dans ce dépôt.
