# Page hôte + onglet Membres — Effets de bord

**Entrée :** `06-domaine.md` validé. Conception, pas implémentation.

## Persistance

### Méthodes de dépôt — trois nouvelles

```rust
async fn list_members_with_profile(&self, space_id: &SpaceId)
    -> Result<Vec<SpaceMemberRow>, SpaceRepositoryError>;

async fn update_member_profile(&self, space_id: &SpaceId, coach_id: &CoachId,
                               profile: &SpaceProfile)
    -> Result<(), SpaceRepositoryError>;

async fn delete_member(&self, space_id: &SpaceId, coach_id: &CoachId)
    -> Result<(), SpaceRepositoryError>;
```

Le SQL vit dans des fichiers dédiés sous `io/repository/sql/space/`, comme le
reste du BC.

```sql
-- list_members_with_profile.sql
SELECT u.id, u.coach_name, u.coach_icon, u.email, m.profile
FROM   spaces__user_cache u
JOIN   spaces__user_space m ON m.coach_id = u.id
WHERE  m.space_id = $1
ORDER BY u.coach_name
```

C'est `list_members_for_space.sql` **plus `m.profile`**. Une méthode distincte
plutôt qu'un élargissement de l'existante : le sélecteur de coachs qui consomme
celle-ci n'a que faire du profil, et lui ajouter une colonne inutilisée ferait
porter à deux appelants le besoin d'un seul.

```sql
-- update_member_profile.sql
UPDATE spaces__user_space SET profile = $3 WHERE space_id = $1 AND coach_id = $2

-- delete_member.sql
DELETE FROM spaces__user_space WHERE space_id = $1 AND coach_id = $2
```

`spaces__user_space` a pour clé primaire `(space_id, coach_id)` — les deux
requêtes touchent donc une ligne au plus, sans qu'il faille s'en assurer.

### Pas de transaction, et pas de migration

`spaces` n'est pas event-sourcé : pas d'append à rendre atomique avec une
projection. La règle de transaction unique du `CLAUDE.md` vise les projections
event-sourcées ; elle est sans objet ici.

**Aucune migration.** Tout ce dont l'onglet a besoin existe :
`spaces__user_space(space_id, coach_id, profile)` et `spaces__user_cache`. La
visibilité, qui en demanderait une, est hors périmètre.

### Tests d'intégration du dépôt

Sur une vraie `PgPool`, comme le reste du projet — pas de mock sqlx.

| Test | Attendu |
|---|---|
| `list_members_with_profile` rend le profil de chaque membre | les deux profils distincts sont lus |
| l'ordre est celui du pseudo | tri stable, indépendant de l'insertion |
| `update_member_profile` sur un membre d'un **autre** espace | zéro ligne touchée |
| `delete_member` sur un membre d'un **autre** espace | zéro ligne touchée |

Les deux derniers ne sont pas de la paranoïa : la clé primaire est composite, et
une requête qui oublierait `space_id` passerait tous les tests d'un espace
unique.

## Événements

### Émission — bus interne

Les deux use cases émettent par `emettre(bus, evenement.to_enveloppe())`. Jamais
de `.send(` direct : l'axe 12 de `check-arch` le refuse, et `to_enveloppe()`
engendre un identifiant que seul le helper voit passer.

### Conversion — le publisher

`io/app_events/app_event_publisher.rs` gagne un cas :

```
UserUnsubscribedFromSpace  →  SpacesAppEvent::UserUnsubscribed
UserPromotedToSpaceAdmin   →  None
UserDemotedToSpaceUser     →  None
```

`SpacesAppEvent::UserUnsubscribed` **existe déjà** dans l'enum, avec son type
`"UserUnsubscribed"` ; personne ne l'émet ni ne l'écoute. Il n'y a pas d'app
event à créer, il y a un app event à réveiller.

Les deux changements de rôle ne traversent pas : `SpacePermissions` relit le
profil en base à chaque requête, aucun BC n'en cache de copie.

### Réception — le listener de `competitions`

```
src/app/competitions/io/app_events/user_unsubscribed_listener.rs
    pub fn init(app_event_bus: &EventBus, repo: Arc<dyn …>)
```

Le nom du paramètre **est** la convention : `init(app_event_bus: …)` signale à
l'axe 5 de `check-arch` qu'il s'agit d'un listener cross-BC, exempté de la règle
de transaction unique. Un événement déjà committé ailleurs ne peut pas partager
sa transaction.

Le listener suit le patron des autres du dépôt — `spawn_listener(module_path!(),
…)`, filtre sur `event_type`, `tokio::spawn` sous un
`tracing::info_span!("app_event", event, event_id)`.

**Son effet** : retirer le coach de `competitions_members` pour toutes les
compétitions de l'espace.

```sql
DELETE FROM competitions_members
WHERE  coach_id = $1
AND    competition_id IN (SELECT id FROM competitions WHERE space_id = $2)
```

C'est du SQL de `competitions` sur des tables de `competitions` — la
souveraineté est respectée. L'espace n'est qu'un critère porté par l'événement.

**Ce que le listener ne fait pas** : toucher aux équipes. `team_proj.coach_id`
continue de pointer sur le coach retiré, l'équipe reste engagée, la compétition
se déroule. C'est la règle 5, et l'état accepté est une équipe dans l'espace
dont le propriétaire n'y a plus accès.

## Handlers

### Signatures

```rust
// page hôte
pub async fn space_admin_controller(
    perms: SpacePermissions,
    State(ctx): State<SpacesContext>,
) -> Result<Response, StatusCode>;

// widgets
pub async fn space_admin_stats_widget(
    perms: SpacePermissions,
    State(ctx): State<SpacesContext>,
) -> Result<SpaceAdminStatsTemplate, StatusCode>;

pub async fn space_admin_members_widget(
    auth_session: AuthSession,
    perms: SpacePermissions,
    State(ctx): State<SpacesContext>,
) -> Result<SpaceAdminMembersTemplate, StatusCode>;

// actions
pub async fn change_member_role_controller(
    auth_session: AuthSession,
    perms: SpacePermissions,
    Path(coach_id): Path<String>,
    State(ctx): State<SpacesContext>,
    Form(form): Form<ChangeRoleForm>,
) -> Result<Response, StatusCode>;

pub async fn remove_member_controller(
    auth_session: AuthSession,
    perms: SpacePermissions,
    Path(coach_id): Path<String>,
    State(ctx): State<SpacesContext>,
) -> Result<Response, StatusCode>;
```

`SpacePermissions` porte `space_id` déjà validé : aucun handler ne l'extrait une
seconde fois.

Le widget des membres reçoit `AuthSession` pour marquer `is_self` sur la bonne
ligne. Le widget des stats n'en a pas besoin.

**Chaque handler commence par `if !perms.is_admin() { return Err(FORBIDDEN) }`.**
Les cinq, sans exception — un widget n'hérite d'aucune protection de sa page
hôte, son endpoint étant directement atteignable.

### Chaque handler tient en moins de vingt lignes

Règle du `CLAUDE.md`. Le découpage est le même partout : construire la commande,
appeler le use case, bâtir la réponse. Le calcul des VMs vit dans `builders.rs`.

### Traduction des erreurs domaine

| Erreur | Statut | Corps |
|---|---|---|
| `ActeurEstLaCible` | 403 | fragment d'erreur |
| `DernierAdministrateur` | 409 | fragment d'erreur — l'état du serveur interdit l'opération |
| `PasMembre` | 404 | fragment d'erreur |
| `EspaceInconnu` | 404 | — |
| `Database` | 500 | — |

409 et non 400 pour `DernierAdministrateur` : la requête est bien formée, c'est
l'état de l'espace qui la refuse, et il peut changer.

Les handlers HTMX reçoivent un **fragment HTML d'erreur**, pas du JSON.

### Réponses en cas de succès

```
POST …/role    → 200, la ligne re-rendue     HX-Trigger: memberRoleChanged
POST …/remove  → 200, corps vide             HX-Trigger: memberRemoved
```

**Le repost du rôle courant re-rend la ligne**, comme tout autre succès. Aucun
événement domaine n'a été émis — rien ne s'est passé — mais la réponse est
uniforme : un 204 sur une action réussie se lit comme un trou dans un journal, et
force le client à distinguer deux formes de succès pour rien.

Le `HX-Trigger` est posé même dans ce cas. Le widget des stats se rafraîchit pour
des compteurs identiques : une requête sans effet, contre une branche
conditionnelle dans le handler et une asymétrie à documenter.

Le retrait rend un **corps vide** avec `hx-swap="outerHTML"` sur la ligne, qui
disparaît. Pas de re-rendu de la liste entière : la ligne sait se supprimer.

## Templates

| Template | VM consommé | Nature |
|---|---|---|
| `space-admin.html` | `SpaceAdminPageTemplate` | page, enveloppée par `host_layout.wrap_page()` |
| `widgets/space-admin-stats.html` | `SpaceAdminStatsTemplate` | fragment |
| `widgets/space-admin-members.html` | `SpaceAdminMembersTemplate` | fragment |
| `widgets/_member-row.html` | `MemberRowVm` | fragment de ligne, inclus par le précédent **et** rendu seul par l'action de rôle |

`_member-row.html` existe parce que deux appelants en ont besoin : la liste
complète et le re-rendu d'une ligne. Le préfixe `_` suit `_coach-result-rows.html`,
déjà en place dans le BC.

### Conventions

- Racine de chaque widget en `hx-disinherit="*"`.
- **Aucun `<link rel="stylesheet">`** : `space-admin-stats.css` et
  `space-admin-members.css` s'inscrivent dans `FEUILLES_APP` de
  `src/web/css_bundle.rs`, section widgets. L'axe 14 de `check-arch` refuse
  toute feuille orpheline.
- Chaque feuille est **nommée d'après la racine de son widget** et ne style rien
  au-delà : `scripts/check-css-collisions.sh` vérifie la portée,
  `tests/e2e/visual/debordements.py` vérifie qu'elle ne trouve pas de markup
  ailleurs.
- **Aucun `style="…"`** : la maquette en contient, ils ne se transcrivent pas.
- Le sélecteur de rôle est un **`<kreek-select>`**.
- Scripts scopés par `document.currentScript.previousElementSibling`.
- Les routes sont celles de `spaces`, jamais `AppRoutes`.

### La réservation de hauteur

Les onglets sont chargés en différé : la zone d'onglet **poussera le contenu** en
arrivant, exactement le défaut des cartes 343 et 361.

Le plancher vient d'une règle, pas d'une estimation : **un espace a toujours au
moins un administrateur**, donc la liste a toujours au moins une ligne. La zone
réserve la hauteur d'une ligne plus la barre de statistiques, en `min-height` et
jamais `height`, vérifiée sous 768 px comme au-dessus.

`tests/e2e/visual/decalages.py` mesure le résultat. Attendu : **0 px** sur la
page d'administration, en desktop et en mobile.

## Tests

### Harnais handler — `src/web/test_harness.rs`

Le niveau qui convient à une matrice d'autorisation : cinq endpoints × trois
profils, en millisecondes.

| Test | Attendu |
|---|---|
| les cinq endpoints, en `SpaceUser` | 403 |
| les cinq endpoints, en non-membre | 403 |
| les cinq endpoints, en `SpaceAdmin` | 200 |
| POST rôle sur le dernier administrateur, en admin | 409 |
| POST retrait sur soi-même, en admin | 403 |

Les deux derniers sont la contrepartie de « le front grise, le domaine
refuse » : ils frappent l'endpoint **sans passer par l'interface**, et prouvent
que le grisage n'est pas la garde.

### Tests E2E — `tests/e2e/test_space_admin.py`

| Scénario | Vérifie |
|---|---|
| un administrateur ouvre la page | les quatre onglets, l'onglet Membres actif |
| la liste affiche les membres avec pseudo, email et rôle | le rendu, donc le VM |
| promouvoir un membre | la ligne se re-rend en Admin, le compteur passe de 1 à 2 |
| rétrograder, deux administrateurs | la ligne se re-rend en Membre, compteur à 1 |
| le dernier administrateur a son sélecteur figé | `role_locked`, après rétrogradation de l'autre |
| retirer un membre | la ligne disparaît, le compteur décroît |
| sa propre ligne | sélecteur désactivé, pas de bouton de retrait |
| la recherche filtre la liste | filtre Alpine, sans requête |
| un `SpaceUser` ouvre l'URL | 403 |
| aucun décalage au chargement | `decalages.py` rend 0 px |

**Le cinquième scénario est le plus utile de la liste.** Il enchaîne deux
opérations — rétrograder l'un, constater que l'autre se fige — et c'est le seul
qui vérifie que le re-rendu de ligne transporte bien le compte postérieur. C'est
précisément ce qui a motivé le retour `ChangementDAppartenance` en phase 5.

Le dernier scénario ne se teste pas dans le même fichier : il vit dans l'outil
visuel, avec l'URL de la page ajoutée à `tests/e2e/visual/urls.py` et sa classe
de portée dans `CLASSE_ATTENDUE`.

## Question ouverte pour la phase 8

- Le listener de `competitions` fait-il une carte à lui seul, ou vient-il avec
  celle du retrait ? Il est court, mais il touche un autre BC et se teste
  autrement — un test d'intégration sur `competitions_members`, pas un test
  d'agrégat.
