# Onglet Ajout direct — Effets de bord

**Entrée :** `06-domaine.md` validé. Conception, pas implémentation.

## Persistance

### Une méthode de recherche, une méthode déjà là

```rust
async fn search_platform_coaches(&self, space_id: &SpaceId, q: &str, limite: i64)
    -> Result<Vec<CandidateRow>, SpaceRepositoryError>;
```

`add_member(&SpaceId, &CoachId, &SpaceProfile)` existe déjà au port et sert
telle quelle.

```sql
SELECT u.id, u.coach_name, u.coach_icon, u.email,
       (m.coach_id IS NOT NULL) AS est_membre
FROM   spaces__user_cache u
LEFT   JOIN spaces__user_space m
       ON m.coach_id = u.id AND m.space_id = $1
WHERE  u.coach_name ILIKE $2 OR u.email ILIKE $2
ORDER  BY u.coach_name
LIMIT  $3
```

**`space_id` est dans la condition de jointure, pas dans le `WHERE`.** L'y
déplacer transformerait la jointure externe en jointure interne et ne rendrait
que les membres — l'exact inverse du besoin, sans erreur, sans exception, avec
une liste qui a l'air d'une liste. C'est le piège de cette requête, et il rend
un résultat plausible.

**Plafond de vingt, seuil de deux caractères**, tous deux en dur dans le
contrôleur : ce sont des décisions du serveur, pas des paramètres. Les exposer
en query permettrait de redemander l'annuaire entier.

`ILIKE` avec `%q%` des deux côtés. Pas d'index à prévoir pour l'instant —
`spaces__user_cache` est petite, et un `LIKE` non ancré n'en tirerait rien sans
extension. À reconsidérer quand l'annuaire grossira, pas avant.

### Aucune migration

Tout existe. `spaces__user_cache` et `spaces__user_space` suffisent.

### Tests d'intégration du dépôt

| Test | Attendu |
|---|---|
| un membre de l'espace est **rendu**, avec `est_membre = true` | la jointure externe tient |
| un non-membre est rendu avec `est_membre = false` | idem |
| un membre d'un **autre** espace est rendu avec `est_membre = false` | **le test qui attrape le piège** — il échoue si `space_id` glisse dans le `WHERE` |
| la recherche par email trouve | les deux colonnes sont bien cherchées |
| vingt-cinq coachs correspondants | vingt rendus |

Le troisième est la raison d'être des cinq.

## Événements

### Ce que `spaces` émet

```
UserAddedToSpaceByAdmin  →  SpacesAppEvent::UserSubscribed
```

Le même app event que `UserSubscribedToSpace`. Le domaine sépare les deux faits,
l'extérieur n'a besoin que de l'effet.

### Ce que `auth` émet

`create_account_without_password` émet `AuthDomainEvent::AccountCreated`, **le
même** que l'inscription publique. `spaces::user_created_listener` continue
d'alimenter `spaces__user_cache` sans rien savoir du chemin.

### Aucun nouveau listener

Contrairement à l'onglet Membres, qui fait réagir `competitions` au retrait.
L'ajout n'a pas de conséquence inter-BC à propager : personne ne tient de liste
de membres à jour.

## Handlers

```rust
// widget des candidats
pub async fn space_admin_candidates_widget(
    perms: SpacePermissions,
    State(ctx): State<SpacesContext>,
    Query(q): Query<CandidateSearchQuery>,
) -> Result<SpaceAdminCandidatesTemplate, StatusCode>;

// action d'ajout
pub async fn add_member_controller(
    auth_session: AuthSession,
    perms: SpacePermissions,
    State(ctx): State<SpacesContext>,
    Form(form): Form<AddMemberForm>,
) -> Result<Response, StatusCode>;

// widget de création de compte — côté auth
pub async fn coach_creation_widget(
    State(ctx): State<AuthContext>,
    Query(prefill): Query<CoachPrefillQuery>,
) -> impl IntoResponse;

pub async fn post_coach_creation(
    State(ctx): State<AuthContext>,
    Form(form): Form<CreateCoachForm>,
) -> Response;
```

`is_admin()` sur les deux endpoints de `spaces`. **Pas sur ceux d'`auth`** : son
routeur entier est public, et `/auth/register` crée déjà des comptes sans
authentification. La garde qui compte est sur l'ajout — créer un compte n'ajoute
personne à un espace.

### Réponses

```
GET  …/candidates       → 200, la liste (ou l'état sous-seuil, ou l'état vide)
POST …/members/add      → 200, la ligne candidate re-rendue en « Déjà membre »
                          HX-Trigger: memberAdded {coach_id, name}
POST /auth/widgets/…    → 200, le widget re-rendu (succès ou erreurs)
                          HX-Trigger: accountCreated {coach_id, name}
```

L'ajout **re-rend la ligne candidate** plutôt que de la retirer : le coach
existe toujours dans l'annuaire, il est simplement devenu membre. La faire
disparaître laisserait croire à une suppression.

### Traduction des erreurs

| Erreur | Statut |
|---|---|
| `DejaMembre` | **409** — la requête est bien formée, l'état la refuse |
| `EspaceInconnu` | 404 |
| `Database` | 500 |

Les erreurs de `create_account_without_password` **ne sortent jamais d'`auth`** :
son widget les rend lui-même, dans son propre fragment. C'est tout le bénéfice
du widget injecté.

## Templates

| Template | BC | Nature |
|---|---|---|
| `widgets/space-admin-candidates.html` | spaces | fragment — liste, état sous-seuil, état vide |
| `widgets/_candidate-row.html` | spaces | ligne, incluse par la liste **et** rendue seule après un ajout |
| `widgets/coach-creation.html` | **auth** | fragment injecté par `coach_creation_widget()` |

### Trois états, pas deux

`sous_seuil` et « aucun résultat » sont distincts. « Tapez au moins deux
caractères » et « aucun coach ne correspond à *xyz* » ne disent pas la même
chose, et **seul le second propose de créer un compte**.

### Le widget d'`auth` apporte son style

`auth` **sert ses propres feuilles** — c'est l'exception documentée de la règle
du bundle, ses pages étant des chargements complets sans swap. Son widget suit
la même règle et n'entre pas dans `FEUILLES_APP`.

Il s'accorde au reste par les **tokens de `common.css`** — `--p1`, `--text-tiny`,
`--radius-*` — qui sont globaux. Il n'utilise **aucune classe de `spaces`** :
c'est ce qui garde le couplage à zéro.

### Le sélecteur de profil reste à `spaces`

La grille Pseudo · Email · Profil de la maquette passe sous **deux
propriétaires**. Les deux premiers champs et le bouton viennent d'`auth`, le
troisième de `spaces`, posé à côté du fragment injecté.

C'est une contrainte sur le dessin, à régler à la maquette avant de coder — pas
une conséquence à découvrir à l'intégration.

## Le journal de session

**Aucun endpoint, aucun VM, aucun template serveur.** Une liste Alpine dans la
page hôte, alimentée par `memberAdded`, perdue au rechargement — c'est le sens
exact de « ajoutés pendant cette session ».

Son bouton « Retirer » appelle `SPACE_ADMIN_MEMBER_REMOVE`, écrite par la carte
371. Retirer quelqu'un qu'on vient d'ajouter et retirer un membre de longue date
sont la même opération.

**Il affiche depuis le payload, jamais d'une relecture** — et c'est ce qui masque
la course du cache : `spaces__user_cache` est alimenté par un app event
asynchrone, donc un compte tout juste créé peut être membre sans encore
apparaître dans la liste des membres. Le journal dit vrai immédiatement.

Le `name` est dans le payload **pour cette seule raison**. Sans lui, le journal
devrait relire — et retomberait dans la course qu'il est censé masquer.

## Tests

### Harnais handler

| Test | Attendu |
|---|---|
| les deux endpoints `spaces`, en `SpaceUser` | 403 |
| les deux, en non-membre | 403 |
| POST ajout d'un coach **déjà membre**, en admin | **409** |
| GET candidats avec `q` d'un caractère | 200, état sous-seuil, **aucune requête de recherche** |

Le dernier vérifie que le seuil est appliqué **avant** la lecture, pas après :
un seuil qui filtre le résultat aurait déjà interrogé l'annuaire entier.

### Tests E2E — `tests/e2e/test_space_admin_direct_add.py`

| Scénario | Vérifie |
|---|---|
| chercher un coach existant | la liste, l'email affiché |
| chercher un membre de l'espace | badge « Déjà membre », pas de bouton |
| ajouter un coach | la ligne passe en « Déjà membre », le compteur monte, la liste des membres le contient |
| l'ajouté apparaît au journal de session | et **immédiatement** — c'est le test de la course |
| le retirer depuis le journal | la ligne disparaît, le compteur redescend |
| chercher un pseudo inexistant | état vide, invitation à créer un compte |
| **créer un compte et ajouter** | le compte existe, le coach est membre, le journal l'affiche |
| taper un seul caractère | état sous-seuil |

**Le septième est le seul filet du contrat `accountCreated`.** Le nom de
l'événement et les clés `coach_id` et `name` ne sont vérifiés par rien d'autre :
ni le compilateur, ni `cargo test`, ni `check-arch`, qui est un `grep` aveugle
aux chaînes littérales et aux attributs HTML. Si `auth` renomme une clé,
**seul ce test le dira**.

Il doit donc vérifier la chaîne complète — compte créé **et** appartenance
posée — pas seulement que le formulaire répond.

### Le piège des tests d'échange HTMX

Un clic sur un élément que sa propre requête remplace peut être **rejoué par
Playwright**. Vécu sur `test_dismissals_phase`. Le remède n'est pas
`dispatch_event`, qui court-circuite l'actionnabilité et clique parfois trop
tôt : c'est d'attendre l'état réel après chaque action.

Ici l'ajout rafraîchit **trois zones** — la ligne candidate, le journal, les
statistiques. L'attente porte sur les trois.

## Question ouverte pour la phase 8

- Le widget de création de compte fait-il une carte à lui seul ? Il vit dans
  `auth`, apporte son use case, son gabarit et sa feuille, et se teste chez lui.
  Mais il n'a aucune valeur tant que `spaces` ne l'affiche pas — et l'afficher
  demande la méthode d'injection, qui est du `spaces`.
