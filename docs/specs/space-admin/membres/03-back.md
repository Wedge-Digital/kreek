# Page hôte + onglet Membres — Architecture back

**Entrée :** `02-front.md` validé.

## Widgets → BCs

Les deux widgets et les deux actions appartiennent à **`spaces`**. Aucun autre
BC ne fournit de fragment sur cette page.

La seule chose qui vient d'ailleurs est la **réinitialisation de mot de passe**,
et elle ne vient pas sous forme de widget — voir ci-dessous.

## Fichiers

```
src/app/spaces/
├── routes.rs · router.rs                     + 5 routes
├── context.rs                                inchangé
├── domain/
│   ├── space.rs                              invariant + 2 méthodes de commande
│   ├── membership_error.rs                   nouveau — erreurs typées
│   └── domain_event.rs                       + UserDemotedToSpaceUser
│                                             + correction du doublon de type
├── domain/space_repository_port/
│   └── space_repository_port.rs              + 3 méthodes
├── io/repository/
│   ├── space_repository.rs                   + 3 implémentations
│   └── sql/space/
│       ├── list_members_with_profile.sql     nouveau
│       ├── update_member_profile.sql         nouveau
│       └── delete_member.sql                 nouveau
├── io/web/
│   ├── host_layout.rs                        + password_reset_action()
│   ├── controllers/
│   │   ├── space_admin_controller.rs         page hôte
│   │   ├── change_member_role_controller.rs
│   │   ├── remove_member_controller.rs
│   │   └── widgets/
│   │       ├── space_admin_stats_widget.rs
│   │       └── space_admin_members_widget.rs
│   └── templates/
│       ├── space-admin.html
│       └── widgets/
│           ├── space-admin-stats.html
│           └── space-admin-members.html
├── io/app_events/app_event_publisher.rs      + mapping UserUnsubscribed
└── use_cases/
    ├── change_member_role_use_case.rs
    └── remove_member_use_case.rs

src/infrastructure/spaces/
└── host_layout_adapter.rs                    + password_reset_action()

src/app/auth/
├── routes.rs · router.rs                     + 1 route
└── io/web/reset_password_request_controller.rs   nouveau

src/app/competitions/io/app_events/
└── user_unsubscribed_listener.rs             nouveau

assets/static/css/widgets/
├── space-admin-stats.css                     nouveau — à inscrire au bundle
└── space-admin-members.css                   nouveau — à inscrire au bundle

src/web/css_bundle.rs                         + 2 feuilles, ordre imposé
```

## Routes

```rust
// spaces
pub const SPACE_ADMIN:                &str = "/app/{space_id}/admin";
pub const SPACE_ADMIN_STATS_WIDGET:   &str = "/app/{space_id}/admin/widgets/stats";
pub const SPACE_ADMIN_MEMBERS_WIDGET: &str = "/app/{space_id}/admin/widgets/members";
pub const SPACE_ADMIN_MEMBER_ROLE:    &str = "/app/{space_id}/admin/members/{coach_id}/role";
pub const SPACE_ADMIN_MEMBER_REMOVE:  &str = "/app/{space_id}/admin/members/{coach_id}/remove";
```

`spaces` étant extractible, ces routes sont exposées par ses **propres**
`Routes`, jamais par `AppRoutes`, et ses templates les appellent directement.

**Chaque route est gardée séparément** par `SpacePermissions::is_admin()`, la
page comme les deux widgets comme les deux actions. Un widget n'hérite d'aucune
protection de sa page hôte : son endpoint est directement atteignable.

## La réinitialisation de mot de passe — pourquoi ce n'est pas un problème

Le bouton « Réinit. mdp » déclenche une opération qui vit dans `auth`. Deux
constats ferment le sujet.

**L'opération est déjà publique.** `app::auth::router::router()` est fusionné
dans `auth_app` **hors** du routeur `protected` qui porte `require_auth` :
`/auth/forgot-password` est atteignable sans être connecté. N'importe qui peut
déjà demander un email de réinitialisation pour n'importe quel pseudo — l'email
part chez le titulaire légitime, ce qui rend l'opération inoffensive.

Le bouton d'un administrateur **n'ajoute donc aucun privilège**. Ce n'est pas
une opération d'administration qui se trouverait vivre dans `auth` ; c'est
l'opération publique existante, avec un bouton commode.

**La destination est injectée par l'hôte**, comme toute destination sortante
d'un BC extractible :

```rust
// dans ISpacesHostLayout, aux côtés de unauthenticated_redirect()
fn password_reset_action(&self, coach_name: &str) -> String;
```

`spaces` rend son propre bouton, avec ses propres classes CSS, qui poste vers
une destination qu'il n'interprète pas.

**L'URL plutôt que le markup, et c'est délibéré.** Le précédent du BC —
`upload_widget(field) -> String` — injecte un fragment rendu par l'hôte, parce
qu'un widget Cloudinary est une mécanique complexe qui s'appartient. Un bouton
de réinitialisation, non : c'est un `action-btn` du dessin de la ligne. Le
faire rendre par `auth` l'obligerait à connaître les classes CSS de `spaces` —
on déplacerait le couplage au lieu de le supprimer.

**Une route à ajouter côté `auth`.** L'endpoint public actuel rend la page
« consultez vos emails », ce qui n'a pas de sens dans une ligne de tableau. Il
faut une variante qui réponde `HX-Trigger: showToast` sans swap. C'est une
route d'`auth`, décidée et écrite par `auth` ; `spaces` n'en connaît que
l'adresse, et par injection.

**Aucun use case côté `spaces`.** Le BC ne fait que rendre un bouton.

## Dépôt — trois méthodes manquantes

```rust
async fn list_members_with_profile(&self, space_id: &SpaceId)
    -> Result<Vec<SpaceMemberRow>, SpaceRepositoryError>;

async fn update_member_profile(&self, space_id: &SpaceId, coach_id: &CoachId,
                               profile: &SpaceProfile)
    -> Result<(), SpaceRepositoryError>;

async fn delete_member(&self, space_id: &SpaceId, coach_id: &CoachId)
    -> Result<(), SpaceRepositoryError>;
```

`list_members_for_space` existe déjà mais rend des `User` **sans leur profil** —
son SQL ne sélectionne pas `m.profile`. L'onglet a besoin du rôle sur chaque
ligne, d'où une méthode distincte plutôt qu'un élargissement : le sélecteur de
coachs qui consomme l'existante n'a que faire du profil.

`SpaceMemberRow` est un DTO de lecture, donc à primitives — c'est la convention
du projet pour les `find_*`/`list_*` de port.

## Domaine

L'agrégat `Space` porte déjà `coaches: Vec<Coach>`, et `Coach` porte son
`profile`. L'invariant a donc tout ce qu'il lui faut sous la main.

```rust
impl Space {
    pub fn change_member_role(&mut self, actor: &CoachId, target: &CoachId,
                              nouveau: SpaceProfile)
        -> Result<SpacesDomainEvent, SpaceMembershipError>;

    pub fn remove_member(&mut self, actor: &CoachId, target: &CoachId)
        -> Result<SpacesDomainEvent, SpaceMembershipError>;
}
```

**Les deux méthodes reçoivent l'acteur**, parce que deux des règles portent sur
lui et non sur la cible : on ne modifie pas son propre rôle, on ne se retire
pas soi-même. Une signature sans acteur obligerait le use case à trancher ces
deux règles — c'est-à-dire à faire du métier.

```rust
pub enum SpaceMembershipError {
    DernierAdministrateur,   // l'invariant : un espace en a toujours un
    ActeurEstLaCible,        // on n'agit pas sur soi-même
    PasMembre,               // la cible n'appartient pas à l'espace
}
```

`spaces` n'a **pas** de `DomainError` central — ses erreurs vivent aujourd'hui
dans des enums par use case (`RegisterSpaceError`, `JoinSpacesError`). Le
`CLAUDE.md` demande des erreurs domaine typées ; on en crée donc un dans
`domain/`, sans reprendre l'existant.

## Événements domaine

```rust
UserPromotedToSpaceAdmin { … }   // existe, jamais émis — repris ici
UserDemotedToSpaceUser   { … }   // nouveau, symétrique
UserUnsubscribedFromSpace{ … }   // nouveau
```

**Deux événements plutôt qu'un portant le rôle cible** : c'est ce qui se lit
dans un journal. `grep UserDemotedToSpaceUser` répond à une question ;
`grep UserRoleChanged` oblige à lire les charges utiles.

## App events — ce qui traverse, et ce qui ne traverse pas

**Le changement de rôle ne traverse pas.** Le rôle d'espace est lu **en direct**
par `SpacePermissions` à chaque requête, via `find_member_profile`. Aucun BC
n'en cache de copie. Promotion et rétrogradation restent donc des événements
strictement internes — pas de mapping dans `to_app_event()`.

**Le retrait traverse.** `SpacesAppEvent::UserUnsubscribed` **existe déjà** dans
l'enum, avec son type `"UserUnsubscribed"` ; personne ne l'émet ni ne l'écoute.
Il n'y a pas d'app event à créer, il y a un app event à réveiller. Le mapping
s'ajoute dans `to_app_event()`, et le publisher fait le reste — le use case ne
touche jamais l'`app_event_bus`.

### Son unique conséquence réelle

Un coach retiré d'un espace peut être **administrateur d'une compétition de cet
espace** : `competitions_members` est vivante et `competitions.space_id` la
scope. Un listener côté `competitions` l'en retire.

```
src/app/competitions/io/app_events/user_unsubscribed_listener.rs
    init(app_event_bus: &EventBus, …)     ← nommage cross-BC, axe 5 de check-arch
```

La convention de nommage compte : `init(app_event_bus: …)` signale à l'axe 5 de
`check-arch` qu'il s'agit d'un listener cross-BC, exempté de la règle de
transaction unique. Un événement déjà committé ailleurs ne peut pas partager sa
transaction.

### Ce qui n'est pas une conséquence

**Les équipes restent.** `team_proj.coach_id` continue de pointer sur le coach
retiré, l'équipe reste engagée, la compétition se déroule. C'est la règle 3 de
la phase 1.

**La saisie des matchs n'est pas touchée** : `can_report_match()` n'est vraie
que pour `SpaceAdmin`. Un membre ordinaire ne saisissait déjà rien.

L'état accepté est donc : **une équipe dans l'espace dont le propriétaire n'y a
plus accès.** Ça se dit, ça ne se corrige pas.

### Les trois caches de `competitions` n'existent pas

`competitions__space_cache`, `competitions__user_cache` et
`competitions__user_space_cache` apparaissent dans les migrations et **ne sont
pas dans la base** : `20260525000001_drop_competitions_cache.sql` les supprime
en `CASCADE`, dix jours après les avoir créées. Vérifié sur la base de dev,
`pg_tables` ne les connaît pas.

Le `CASCADE` a emporté au passage les clés étrangères que `competitions` et
`competitions_members` portaient vers elles — d'où des `REFERENCES` visibles
dans les migrations de création qui ne correspondent à aucune contrainte
vivante.

Il n'y a donc **rien à synchroniser et rien à nettoyer**. Le seul cache de
membres vivant du dépôt est `spaces__user_cache`, et il appartient à `spaces`.

## Ports inter-BC — aucun

`spaces__user_cache` contient déjà tous les coachs de la plateforme, alimenté
par `user_created_listener`. Le sens de la propagation est **sortant**. Aucun
port, aucun adapter, aucun domain service : les VMs se construisent directement
depuis le domaine et le DTO de lecture du dépôt.

## Ordre imposé

**Le doublon de type d'événement se corrige en premier.**
`USER_INVITED_IN_SPACE` et `USER_SUBSCRIBED_TO_SPACE` valent tous deux
`"UserRegisteredInSpace"`. L'onglet Membres n'émet ni l'un ni l'autre, mais il
touche à ce fichier : c'est le bon moment, et ça évite que l'onglet Invitations
en hérite.

Ensuite : domaine → dépôt → use cases → widgets → actions → listener
`competitions`.

## Questions ouvertes pour la phase 4

- `SpaceMemberRow` doit-il porter l'email ? La maquette l'affiche sous le
  pseudo. C'est une donnée personnelle exposée à tout administrateur d'espace —
  à confirmer plutôt qu'à supposer.
- Le widget stats fait-il une requête ou deux ? Compter les membres et les
  administrateurs se déduit d'une seule lecture ; les invitations en attente
  viendront d'ailleurs.
