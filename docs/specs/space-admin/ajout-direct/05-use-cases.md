# Onglet Ajout direct — Use cases

**Entrée :** `04-dtos.md` validé.

Deux use cases, dans deux BCs. Le widget des candidats et le journal de session
sont en lecture et n'en ont pas.

## `spaces::add_member_use_case`

```rust
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd:   AddMemberCommand,
    repo:  &dyn ISpaceRepository,
    email: &dyn IEmailService,
    bus:   &EventBus,
) -> Result<MembershipOutcome, AddMemberError>;
```

**Orchestration**

1. `repo.find_by_id(&cmd.space_id)` → `EspaceInconnu` si `None`
2. `space.add_member(&cmd.acteur, &cmd.nouveau, cmd.profil)` — l'agrégat valide
   et rend l'événement
3. `repo.add_member(&cmd.space_id, &cmd.nouveau, &cmd.profil)` — la méthode
   existe déjà au port
4. si `cmd.notifier == Notification::Envoyer` : envoyer la courtoisie
5. `emettre(bus, event.to_enveloppe())`
6. rendre `MembershipOutcome { administrateurs }`

Le compte d'administrateurs sert ici aussi : ajouter un administrateur peut
faire passer l'espace de un à deux, ce qui **dégèle** la ligne du premier.
L'onglet Membres se rafraîchit sur `memberAdded`, et sa liste doit alors rendre
un sélecteur redevenu actif.

### La notification est envoyée par le use case, pas portée par l'événement

`notifier` est une **case à cocher**, c'est-à-dire l'état d'une interface au
moment d'un clic. La faire voyager dans l'événement domaine l'inscrirait pour
toujours au journal — `event_log_feeder` persiste chaque enveloppe — et un
lecteur futur y verrait une propriété du fait « ce coach a été ajouté », qu'elle
n'est pas.

Le use case lit le drapeau de la commande et appelle `IEmailService`. C'est le
précédent d'`auth`, dont `send_reset_password_email` envoie directement plutôt
que d'émettre pour qu'un listener envoie.

**`SpacesContext` gagne donc `email_service: Arc<dyn IEmailService>`**, comme
`AuthContext`. `crate::common::services::email` ne figure pas dans ce qu'un BC
extractible s'interdit — c'est un service de la couche commune, pas un autre BC,
pas l'hôte.

### Un échec d'envoi ne fait pas échouer l'ajout

L'appartenance est posée, l'événement est émis, et un email qui ne part pas est
journalisé en `warn`. L'inverse — refuser l'ajout parce que le serveur de mail
est indisponible — ferait dépendre une règle d'appartenance d'un service qui
n'en gouverne aucune.

**Erreurs**

```rust
pub enum AddMemberError {
    EspaceInconnu,
    Metier(SpaceMembershipError),   // DejaMembre
    Database(String),
}
```

Le use case **ne réinterprète pas** l'erreur du domaine, il la transporte.
`DejaMembre` deviendra un 409 côté contrôleur : la requête est bien formée,
c'est l'état de l'espace qui la refuse.

## `auth::create_account_without_password`

```rust
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd:   CreateAccountWithoutPasswordCommand,
    repo:  &dyn IUserRepository,
    jetons: &dyn IResetTokenRepository,
    email: &dyn IEmailService,
    bus:   &EventBus,
) -> Result<CoachId, CreateAccountError>;
```

**Pourquoi un use case de plus.** `RegisterCommand` exige `password` et
`password_confirm`, et refuse en dessous de huit caractères. Un compte créé par
un administrateur n'en a pas — et lui en inventer un serait pire : un mot de
passe que personne ne connaît et que rien n'oblige à changer.

**Orchestration**

1. mêmes vérifications d'unicité et de format que l'inscription publique —
   pseudo et email
2. créer le compte **sans hachage de mot de passe**
3. engendrer un jeton de réinitialisation et envoyer l'email de définition
4. `emettre(bus, AuthDomainEvent::AccountCreated { … })`

**Le même événement que l'inscription publique.** C'est le même fait : un compte
existe. Le chemin par lequel il a été créé n'intéresse aucun BC d'à côté, et
`spaces::user_created_listener` continue d'alimenter son cache sans rien savoir.

**L'email de définition part toujours** — décidé en phase 2, la case de la
maquette est retirée. Ici l'envoi est donc **une étape, pas une option**, et son
échec fait échouer le use case : un compte sans mot de passe et sans email reçu
est un compte auquel personne ne peut accéder.

C'est l'inverse du choix fait pour la courtoisie de `spaces`, et les deux sont
cohérents : là-bas l'email est un agrément, ici c'est l'unique porte d'entrée.

**Erreurs**

```rust
pub enum CreateAccountError {
    PseudoDejaPris,
    EmailDejaPris,
    PseudoInvalide,
    EmailInvalide,
    EnvoiEmailImpossible(String),
    Database(String),
}
```

Ces variantes **ne remontent jamais jusqu'à `spaces`** : le widget d'`auth` les
rend lui-même, dans son propre fragment. C'est tout le bénéfice du widget
injecté — le jour où `auth` ajoute une vérification, il ajoute une variante et
un message, et `spaces` ne bouge pas.

## L'échec après création, et ce qu'on en dit

Les deux use cases vivent dans deux requêtes. Si la création réussit et que
l'ajout échoue, **le compte existe et l'appartenance non**.

Le message d'erreur le dit explicitement — « le compte a bien été créé, mais
l'ajout à l'espace a échoué ; retrouvez le coach dans la recherche ci-dessus » —
plutôt que de rester générique. Un compte orphelin dont l'administrateur ignore
l'existence, c'est un pseudo et un email pris, et une seconde tentative qui
échouera sur `PseudoDejaPris` sans qu'il comprenne pourquoi.

La reprise ne demande rien de neuf : le coach apparaît dans le panneau de
recherche, où un clic l'ajoute.

## Tests unitaires

### `add_member_use_case`, sur `FakeRepo` et un service d'email factice

| Test | Attendu |
|---|---|
| ajout nominal en Membre | `Ok`, `UserAddedToSpaceByAdmin`, compte inchangé |
| ajout nominal en Admin | `Ok`, compte +1 |
| coach déjà membre | `Metier(DejaMembre)`, **aucune écriture, aucun événement, aucun email** |
| espace inconnu | `EspaceInconnu` |
| `notifier = Envoyer` | l'email factice a reçu un envoi |
| `notifier = Taire` | il n'en a reçu aucun |
| l'envoi d'email échoue | `Ok` quand même, et l'écriture a bien eu lieu |

Le dernier est le seul qui vérifie que la courtoisie ne gouverne pas
l'appartenance.

### `create_account_without_password`

| Test | Attendu |
|---|---|
| création nominale | `Ok(coach_id)`, `AccountCreated` émis, email envoyé |
| pseudo déjà pris | `PseudoDejaPris`, aucun compte créé |
| email déjà pris | `EmailDejaPris`, aucun compte créé |
| l'envoi d'email échoue | `EnvoiEmailImpossible` — **et le compte n'est pas laissé derrière** |

Le dernier est le plus délicat de la liste, et il décide d'une conception :
l'envoi doit précéder le point de non-retour, ou l'échec doit défaire le compte.
Le compte créé ne doit pas rester si son unique porte d'entrée n'a pas été
livrée — sinon on fabrique exactement l'orphelin que la section précédente
s'emploie à rendre visible.

## Question ouverte pour la phase 6

- `Space::add_member` ne porte qu'une règle, `DejaMembre`. Faut-il un plafond de
  membres par espace ? Rien dans la maquette ni dans les règles validées ne le
  suggère, mais c'est le genre d'invariant qui coûte cher à ajouter après coup,
  une fois des espaces au-delà du seuil en production.
