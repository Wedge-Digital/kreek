# Onglet Ajout direct — Architecture back

**Entrée :** `02-front.md` validé.

## Widgets → BCs

| Widget | BC |
|---|---|
| liste des candidats | `spaces` |
| formulaire de création de compte | **`auth`**, injecté par l'hôte |
| journal de session | `spaces`, sans endpoint — il vit au client |

C'est le premier endroit de l'application où un BC extractible affiche un
fragment d'un autre. Le mécanisme n'est pas neuf pour autant : `upload_widget()`
fait déjà exactement cela pour le widget Cloudinary.

## Fichiers

```
src/app/spaces/
├── routes.rs · router.rs                     + 2 routes
├── domain/space.rs                           + add_member()
├── domain/membership_error.rs                + DejaMembre
├── domain/domain_event.rs                    + UserAddedToSpaceByAdmin
├── domain/space_repository_port/…            + 1 méthode de recherche
├── io/repository/
│   ├── space_repository.rs                   + 1 implémentation
│   └── sql/space/search_platform_coaches.sql nouveau
├── io/web/
│   ├── host_layout.rs                        + coach_creation_widget()
│   ├── controllers/
│   │   ├── add_member_controller.rs          nouveau
│   │   └── widgets/space_admin_candidates_widget.rs
│   └── templates/widgets/
│       ├── space-admin-candidates.html
│       └── _candidate-row.html
└── use_cases/add_member_use_case.rs          nouveau

src/infrastructure/spaces/
└── host_layout_adapter.rs                    + coach_creation_widget()

src/app/auth/
├── routes.rs · router.rs                     + 1 route
├── use_cases/create_account_without_password.rs  nouveau
├── io/web/coach_creation_widget.rs           nouveau
└── io/web/templates/widgets/coach-creation.html  nouveau

assets/static/css/widgets/space-admin-candidates.css   + bundle
```

## Routes

```rust
// spaces
pub const SPACE_ADMIN_CANDIDATES_WIDGET: &str = "/app/{space_id}/admin/widgets/candidates";
pub const SPACE_ADMIN_MEMBER_ADD:        &str = "/app/{space_id}/admin/members/add";

// auth — publique comme le reste de ses routes, cf. plus bas
pub const COACH_CREATION_WIDGET: &str = "/auth/widgets/coach-creation";
```

Le retrait depuis le journal de session **n'ajoute aucune route** : il appelle
`SPACE_ADMIN_MEMBER_REMOVE`, écrite par la carte 371.

## Le formulaire de création de compte

### Ce qu'`auth` doit gagner

`RegisterCommand` exige `password` et `password_confirm`, et refuse en dessous
de huit caractères. Un compte créé par un administrateur n'en a pas.

D'où `create_account_without_password` : mêmes vérifications d'unicité et de
format que l'inscription, pas de mot de passe, et l'email de définition envoyé
dans la foulée — la case de la maquette est retirée, l'email part toujours.

Le use case émet `AuthDomainEvent::AccountCreated`, **le même** que
l'inscription publique. C'est le même fait : un compte existe. Le chemin par
lequel il a été créé n'intéresse pas les BCs d'à côté, et
`spaces::user_created_listener` continue d'alimenter son cache sans rien savoir.

### Le widget

```rust
// dans ISpacesHostLayout, aux côtés d'upload_widget()
fn coach_creation_widget(&self, prefill: CoachPrefill<'_>) -> String;
```

Le fragment rend Pseudo, Email et le bouton, valide, et affiche **ses** erreurs
chez lui. En cas de succès il pose
`HX-Trigger: {"accountCreated": {"coach_id": "…", "name": "…"}}`.

`spaces` écoute cet événement et poste l'appartenance sur sa propre route. Les
deux BCs ne se parlent que par le DOM — c'est la règle 2 des conventions
widgets, appliquée entre BCs au lieu de widgets.

### La route d'`auth` est publique, et ce n'est pas un oubli

Toutes les routes d'`auth` le sont : son routeur est fusionné dans `auth_app`
**hors** du routeur `protected` qui porte `require_auth`. L'inscription publique
crée déjà des comptes sans authentification.

Cette route n'ouvre donc **aucune capacité nouvelle** : elle fait ce que
`/auth/register` fait déjà, en rendant un fragment au lieu d'une page, et sans
mot de passe. La garde qui compte est ailleurs — sur
`SPACE_ADMIN_MEMBER_ADD`, côté `spaces`, qui exige `is_admin()`. Créer un compte
n'ajoute personne à un espace.

### Le sélecteur de profil reste à `spaces`

`SpaceProfile` est son concept. La grille de la maquette passe sous deux
propriétaires : deux champs et le bouton viennent d'`auth`, le troisième champ
de `spaces`. Le contrôleur d'ajout lit le profil dans le formulaire de `spaces`,
au moment où `accountCreated` arrive.

## Domaine

```rust
impl Space {
    pub fn add_member(&mut self, acteur: &CoachId, nouveau: &CoachId,
                      profil: SpaceProfile)
        -> Result<ChangementDAppartenance, SpaceMembershipError>;
}
```

Vérifications, dans l'ordre :

1. `nouveau` est déjà dans `self.coaches` → `DejaMembre`
2. ajouter à `self.coaches`, produire `UserAddedToSpaceByAdmin`

`DejaMembre` s'ajoute à `SpaceMembershipError`. **Le badge « Déjà membre » de la
liste des candidats est une politesse** — comme `role_locked` de l'onglet
Membres — et un POST direct doit être refusé par le domaine. Sans cette
vérification, le doublon serait refusé par la clé primaire composite de
`spaces__user_space`, sous forme d'erreur SQL brute : une règle métier rendue par
une contrainte d'intégrité, illisible et intraduisible en 409.

`acteur` est présent pour l'événement, pas pour une règle : rien n'interdit à un
administrateur d'ajouter qui il veut. Il est là parce que **l'ajout se passe du
consentement**, et qu'une opération sans consentement doit dire qui l'a
ordonnée.

Le retour réutilise `ChangementDAppartenance` : ajouter un administrateur change
le compte, et la liste des membres doit se re-rendre en conséquence.

## Événements

```
UserAddedToSpaceByAdmin { event_id, user_id, space_id, profile, added_by }
    → SpacesAppEvent::UserSubscribed
```

**Un événement domaine distinct, un app event partagé.** Le domaine distingue
l'adhésion spontanée de l'ajout par un administrateur : ce sont deux faits,
et le journal doit les séparer par un `grep`, pas par la lecture des charges
utiles. L'extérieur, lui, n'a besoin que de l'effet — un coach est membre —
d'où le même `SpacesAppEvent::UserSubscribed` pour les deux.

C'est exactement la distinction que le `CLAUDE.md` pose entre événements
domaine et app events : les premiers enregistrent ce qui s'est passé, les
seconds franchissent une frontière.

## Dépôt

```rust
async fn search_platform_coaches(&self, space_id: &SpaceId, q: &str, limite: i64)
    -> Result<Vec<CandidateRow>, SpaceRepositoryError>;
```

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

**Les membres sont rendus, pas exclus**, avec `est_membre` qui porte le badge.
Les exclure laisserait croire qu'un coach n'existe pas alors qu'il est déjà là —
et l'administrateur chercherait à créer un compte qui existe.

`LEFT JOIN` sur `spaces__user_space` **avec `space_id` dans la condition de
jointure**, pas dans le `WHERE` : le mettre dans le `WHERE` transformerait la
jointure externe en jointure interne et ne rendrait que les membres. C'est le
piège classique de cette requête, et il donne un résultat plausible.

**Plafond de vingt résultats**, et **rien en dessous de deux caractères** :
afficher les vingt premiers de l'annuaire entier serait un échantillon arbitraire
présenté comme une réponse.

`add_member` existe déjà au port et sert telle quelle.

## Le use case

```rust
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(cmd: AddMemberCommand, repo: &dyn ISpaceRepository,
                     bus: &EventBus)
    -> Result<MembershipOutcome, AddMemberError>;
```

Même forme que les deux de la carte 367 : charger, appeler, persister, émettre,
rendre le compte. La notification par email — « Prévenir le coach qu'il a
rejoint l'espace », restée optionnelle — est un effet du use case, déclenché sur
le drapeau de la commande.

## La course du cache d'utilisateurs

`spaces__user_cache` est alimenté par `user_created_listener`, en réaction à
l'app event `AccountCreated`. La liste des membres lit `spaces__user_space`
**jointe** à ce cache. Un compte tout juste créé peut donc être membre sans
apparaître dans la liste, le temps que l'événement atterrisse.

**Retenu : le journal de session masque la course sans la corriger.** Il affiche
la ligne depuis le payload de `memberAdded` qu'il tient déjà, sans rien relire.
L'écran dit vrai immédiatement ; la liste des membres se rattrape au
rafraîchissement suivant.

Deux autres issues, écartées :

- **L'adapter écrit le cache en synchrone.** Le cache aurait alors deux
  alimenteurs, dont un qui court-circuite l'événement — et la question « qui a
  écrit cette ligne ? » n'aurait plus de réponse unique.
- **Le use case attend.** Attendre un listener asynchrone depuis un use case,
  c'est transformer une propagation en appel synchrone déguisé, avec un délai
  d'attente à choisir et un échec à traiter.

La course reste donc réelle et bornée à quelques dizaines de millisecondes, sur
un écran qui n'en dépend pas.

## Questions ouvertes pour la phase 4

- `CandidateRow` porte-t-il l'email ? Il est affiché dans la maquette, et il
  sert à chercher. Cohérent avec la décision de l'onglet Membres, mais il s'agit
  ici de **tous les coachs de la plateforme**, pas des seuls membres de l'espace
  — l'exposition n'est pas de même ampleur.
- `CoachPrefill` porte-t-il un pseudo, un email, ou les deux ? Le front décide
  selon la présence d'un `@` ; reste à savoir si le widget d'`auth` reçoit un
  champ ciblé ou une chaîne à répartir lui-même.
