# Onglet Ajout direct — Contrats de données

**Entrée :** `03-back.md` validé.

Chaque type porte son **émetteur** et son **consommateur**.

## DTOs d'entrée

### La recherche

```rust
#[derive(Deserialize)]
pub struct CandidateSearchQuery {
    #[serde(default)]
    pub q: String,
}
```

Le plafond de vingt et le seuil de deux caractères **ne sont pas des paramètres**
— ce sont des décisions du serveur, en dur dans le contrôleur. Les exposer en
query permettrait à n'importe quel appelant de demander l'annuaire entier.

`space_id` vient de `SpacePermissions`, déjà validé.

### L'ajout

```rust
#[derive(Deserialize)]
pub struct AddMemberForm {
    pub coach_id: String,
    pub profile:  String,        // "SpaceAdmin" | "SpaceUser"
    #[serde(default)]
    pub notifier: bool,          // « Prévenir le coach qu'il a rejoint l'espace »
}
```

Primitives assumées : frontière HTTP. Le contrôleur convertit par
`CoachId::try_new()` et `SpaceProfile::try_from(&str)`.

`notifier` est `#[serde(default)]` parce qu'une case décochée **n'est pas
envoyée** par un formulaire HTML. Absent vaut donc faux, ce qui est le
comportement voulu.

| DTO | Émetteur | Consommateur |
|---|---|---|
| `CandidateSearchQuery` | champ de recherche, `hx-get` débouncé | `space_admin_candidates_widget` |
| `AddMemberForm` | bouton « Ajouter » d'une ligne candidate, **et** la réaction à `accountCreated` | `add_member_controller` |

Les deux chemins d'ajout postent **le même formulaire**. Le second le remplit
depuis le payload de l'événement DOM et le sélecteur de profil de `spaces` ; le
contrôleur ne les distingue pas, et n'a aucune raison de le faire.

## Commande applicative

```rust
pub struct AddMemberCommand {
    pub space_id: SpaceId,
    pub acteur:   CoachId,
    pub nouveau:  CoachId,
    pub profil:   SpaceProfile,
    pub notifier: Notification,
}
```

`Notification` est un value object, pas un `bool` nu — règle CQRS. Un booléen
dans une commande est un drapeau dont on ne sait plus, six mois après, ce que
`true` veut dire ; `Notification::Envoyer` / `Notification::Taire` se lit à
l'appel comme dans le journal.

`acteur` vient d'`AuthSession`. Il ne sert **aucune règle** — rien n'interdit à
un administrateur d'ajouter qui il veut — il sert la **trace** : l'ajout se
passe du consentement du coach, et une opération sans consentement doit dire qui
l'a ordonnée.

| DTO | Émetteur | Consommateur |
|---|---|---|
| `AddMemberCommand` | `add_member_controller` | `add_member_use_case` |

## DTO de lecture du dépôt

```rust
pub struct CandidateRow {
    pub coach_id:   String,
    pub coach_name: String,
    pub email:      String,
    pub icon:       Option<String>,
    pub est_membre: bool,
}
```

`est_membre` vient du `LEFT JOIN` — les membres sont **rendus, pas exclus**, pour
ne pas laisser croire qu'un coach n'existe pas alors qu'il est déjà là.

**L'email est affiché, et c'est une décision d'exposition.** Contrairement à
l'onglet Membres, qui montre les emails des membres d'un espace, celui-ci rend
consultable **tout coach de la plateforme**, par pseudo comme par email, depuis
n'importe quel espace dont on est administrateur. Trois garde-fous bornent la
portée, et ils sont là pour ça : le seuil de deux caractères, le plafond de
vingt résultats, et `is_admin()`. Aucun ne se retire sans rouvrir la question.

| DTO | Émetteur | Consommateur |
|---|---|---|
| `CandidateRow` | `ISpaceRepository::search_platform_coaches` | `builders.rs` du widget, **jamais un template** |

## View models

```rust
pub struct CandidateRowVm {
    pub coach_id:   String,
    pub name:       String,
    pub email:      String,
    pub initials:   String,
    pub est_membre: bool,
}

#[derive(Template)]
#[template(path = "widgets/space-admin-candidates.html")]
pub struct SpaceAdminCandidatesTemplate {
    pub routes:     Routes,
    pub space_id:   String,
    pub candidats:  Vec<CandidateRowVm>,
    pub requete:    String,      // pour l'état vide et le pré-remplissage
    pub sous_seuil: bool,        // moins de deux caractères
}
```

`sous_seuil` est distinct d'une liste vide : « tapez au moins deux caractères »
et « aucun coach ne correspond à *xyz* » sont deux états, et le second seul
propose de créer un compte.

`CandidateRowVm` n'a pas de `role_locked` : le sélecteur de profil d'une ligne
candidate est toujours actif. `est_membre` suffit — la ligne rend alors un badge
au lieu du sélecteur et du bouton.

| VM | Émetteur | Consommateur |
|---|---|---|
| `CandidateRowVm` | `builders.rs` du widget candidats | `_candidate-row.html` |
| `SpaceAdminCandidatesTemplate` | `space_admin_candidates_widget` | Askama |

## Les contrats qui traversent le DOM

Ceux-là ne sont pas des structs Rust, et c'est précisément pourquoi ils doivent
être écrits : rien ne les vérifie à la compilation.

L'onglet Membres avait déjà des événements DOM — `memberRemoved`,
`memberRoleChanged`. Deux choses changent ici.

**La charge utile est lue.** Là-bas, les événements servaient de **signaux** :
le widget des statistiques les recevait et **relisait le serveur**, sans
toucher au payload. Un nom mal orthographié donnait un compteur qui ne se
rafraîchit pas — visible, gênant, réparable. Ici `coach_id` est extrait et
reposté : une clé mal nommée ne donne pas un écran périmé, elle donne une
requête avec un identifiant vide.

**Et le contrat franchit une frontière de BC.** Partout ailleurs, un BC parle à
un autre par un trait, un DTO, un enum d'app event — des types que le
compilateur vérifie. `accountCreated` n'a de type sur aucun des deux bords.

### `accountCreated` — d'`auth` vers `spaces`

```json
{ "coach_id": "01J…", "name": "NurgleFan" }
```

Posé par `auth` en `HX-Trigger` après création réussie. Écouté par l'onglet, qui
poste `AddMemberForm` en y joignant le profil choisi dans **son** sélecteur.

C'est le seul point de contact entre les deux BCs sur cet onglet, et il est
**textuel**. Ni le compilateur, ni `check-arch` ne le vérifieront : seul un test
e2e peut dire qu'il tient. C'est le prix du widget injecté, et il est assumé —
la contrepartie étant qu'`auth` garde ses règles et ses erreurs chez lui.

### `memberAdded` — de `spaces` vers ses propres widgets

```json
{ "coach_id": "01J…", "name": "NurgleFan" }
```

Posé par `add_member_controller`. Écouté par les statistiques, la liste des
membres, et le journal de session.

**Le journal se sert du payload, pas d'une relecture.** C'est ce qui masque la
course du cache d'utilisateurs : `spaces__user_cache` est alimenté par un app
event asynchrone, donc un compte tout juste créé peut être membre sans encore
apparaître dans la liste. Le journal, lui, affiche depuis ce qu'il tient déjà et
dit vrai immédiatement.

Le `name` est dans le payload **pour cette seule raison**. Sans lui, le journal
devrait relire — et retomberait dans la course qu'il est censé masquer.

## Ce que l'hôte injecte

```rust
pub struct CoachPrefill<'a> {
    pub pseudo: Option<&'a str>,
    pub email:  Option<&'a str>,
}
```

Deux champs ciblés plutôt qu'une chaîne à répartir : **la répartition est une
décision de `spaces`**, qui sait ce que l'utilisateur cherchait. Faire trancher
`auth` sur la présence d'un `@` lui ferait deviner une intention qu'il n'observe
pas.

| DTO | Émetteur | Consommateur |
|---|---|---|
| `CoachPrefill` | l'onglet, via `host_layout.coach_creation_widget()` | le widget d'`auth` |

## Ce qui n'existe pas ici

**Aucun DTO de port inter-BC**, et donc aucun domain service. `auth` n'est pas
consulté par du Rust : il est affiché, et il répond par un événement DOM.

**Aucun VM pour le journal de session.** Il vit au client, construit en
JavaScript depuis le payload de `memberAdded`. Sa seule dépendance serveur est
la route de retrait, écrite par la carte 371.

## Question ouverte pour la phase 5

- L'échec de l'ajout **après** une création de compte réussie laisse un compte
  orphelin, non rattaché. La phase 2 l'accepte et compte sur le panneau de
  recherche pour rattraper. Le message d'erreur doit-il le dire explicitement —
  « le compte est créé, l'ajout a échoué, cherchez-le ci-dessus » — ou rester
  générique ?
