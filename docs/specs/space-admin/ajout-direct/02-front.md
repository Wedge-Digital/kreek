# Onglet Ajout direct — Architecture front

**Maquette :** `assets/rawpages/html/app-space-admin.html`, bloc `#tab-direct-add`.

L'onglet ajoute un coach à l'espace **sans son consentement** — c'est ce que dit
son bandeau d'avertissement, et c'est ce qui explique plusieurs décisions
ci-dessous. Deux chemins : le coach a déjà un compte sur la plateforme, ou il
n'en a pas et on le lui crée.

## Les widgets

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| `space-admin-candidates` | spaces | `GET …/admin/widgets/candidates?q=` | frappe (debounce), `memberRemoved from:body` | `memberAdded` | mutation |
| `coach-creation` | **auth** | injecté par `ISpacesHostLayout` | rendu avec l'onglet | `accountCreated` | mutation |
| `space-admin-session-journal` | spaces | aucun — client | `memberAdded from:body` | — | lecture |

## Événements

```
memberAdded     { coach_id, name }   émis par : candidats, et par la réaction à accountCreated
                                     écouté par : stats, liste des membres, journal de session

memberRemoved   { coach_id }         émis par : liste des membres, journal de session
                                     écouté par : stats, candidats

accountCreated  { coach_id, name }   émis par : le widget d'auth
                                     écouté par : l'onglet, qui pose l'appartenance
```

**Le contrat posé pour l'onglet Membres se paie ici.** `memberAdded` y était déjà
spécifié comme émis par cet onglet, `memberRemoved` déjà prévu pour lever le
badge « Déjà membre ». Rien à renégocier — c'est ce qu'on attendait d'avoir
défini les événements avant d'avoir les deux émetteurs.

## Actions

```
POST …/admin/members/add          { coach_id, profile, notifier }
     → remplace la ligne candidate    HX-Trigger: memberAdded

POST …/admin/members/{id}/remove  ← celle de la carte 371, réutilisée telle quelle
     → retire la ligne du journal     HX-Trigger: memberRemoved
```

Le journal de session **n'a pas d'action à lui**. Son bouton « Retirer » appelle
l'action de retrait de l'onglet Membres, déjà écrite. Retirer quelqu'un qu'on
vient d'ajouter et retirer un membre de longue date sont la même opération.

## Le formulaire de création appartient à `auth`

`ISpacesHostLayout::coach_creation_widget(prefill) -> String`, sur le modèle
d'`upload_widget()` déjà en place. L'hôte rend un fragment d'`auth` et le donne à
`spaces` sous forme de chaîne ; `spaces` ne connaît pas `auth`.

**Pourquoi un widget et non un port.** Les règles de création de compte —
unicité du pseudo, unicité de l'email, format, ce que « sans mot de passe »
implique — appartiennent à `auth` et **changeront là-bas**. Un port obligerait
`spaces` à les ré-exprimer dans un enum d'erreurs et à les rendre dans son
propre gabarit ; le jour où `auth` ajoute une vérification, le port bouge et
`spaces` suit. Avec le widget, `auth` valide et affiche ses erreurs chez lui.

**Pourquoi le markup ici et l'URL pour le bouton de réinitialisation.** Un
bouton unique dans une ligne de tableau est un élément du dessin de la ligne :
le faire rendre par `auth` l'obligerait à connaître les classes CSS de `spaces`.
Un panneau de création de compte est une mécanique autonome, comme le widget
Cloudinary qu'`upload_widget()` injecte déjà. Et `auth` **sert ses propres
feuilles** — l'exception documentée de la règle du bundle — donc un widget qui
apporte son style est cohérent avec ce que ce BC fait de toute façon. Les tokens
de `common.css` suffisent à l'accorder au reste.

### Le sélecteur de profil reste à `spaces`

`SpaceProfile` est un concept de `spaces` ; `auth` ne peut pas le connaître. La
grille à trois colonnes de la maquette — Pseudo, Email, Profil — passe donc sous
**deux propriétaires** : les deux premiers champs et le bouton viennent d'`auth`,
le troisième de `spaces`.

C'est une contrainte sur le dessin, pas un détail d'implémentation, et elle est à
traiter à la maquette avant de coder.

### Deux allers-retours, et ce qui arrive si le second échoue

```
[widget auth] créer le compte  ──►  accountCreated { coach_id }
                                          │
[spaces] POST …/members/add ◄─────────────┘
```

Si le second échoue, **le compte existe et l'appartenance non**. Ça se dégrade
proprement : le coach apparaît alors dans le panneau de recherche juste au-
dessus, où un clic l'ajoute. Ce n'est pas une corruption, mais ce n'est plus
« une action, deux effets » — et c'est écrit ici plutôt que découvert.

## Front contre back

**Au serveur, la recherche de candidats.** Elle porte sur l'annuaire de la
plateforme, dont la taille croît sans rapport avec l'espace. C'est l'inverse du
choix fait pour l'onglet Membres, dont la liste tient dans un écran — les deux
choix sont justes pour leur liste.

**Au client, le journal de session.** Une liste Alpine alimentée par
`memberAdded`, perdue au rechargement : c'est le sens exact de « ajoutés pendant
cette session ». Aucun stockage serveur, aucune notion de session à inventer.

**Au client, le pré-remplissage.** Quand la recherche ne rend rien, ce qui a été
tapé part dans le champ Pseudo ou Email du widget d'`auth` selon qu'il contient
un `@`. Le widget étant rendu par l'hôte, le pré-remplissage passe par un
paramètre de son endpoint.

## La course du cache d'utilisateurs

`spaces__user_cache` est alimenté **de façon asynchrone**, par
`user_created_listener` réagissant à l'app event `AccountCreated` d'`auth`. Or la
liste des membres lit `spaces__user_space` **jointe** à ce cache.

Donc : compte créé, appartenance posée, et la ligne **n'apparaît pas dans la
liste des membres** tant que l'app event n'a pas atterri. Pas de crash —
`spaces__user_space` n'a aucune clé étrangère — mais un membre invisible pendant
quelques dizaines de millisecondes, sur l'action où l'administrateur regarde
justement si ça a marché.

**Le journal de session masque la course sans la corriger** : il affiche la ligne
depuis le payload de `memberAdded`, qu'il tient déjà, sans rien relire. L'écran
dit vrai immédiatement ; la liste des membres se rattrape au rafraîchissement
suivant.

À confirmer en phase 3, avec les deux autres issues envisagées et écartées :
l'adapter écrivant le cache en synchrone, ou le use case attendant.

## Règles métier de cet onglet

1. **On ne peut pas ajouter un coach déjà membre.** Le domaine refuse, avec une
   erreur `DejaMembre`. Le badge « Déjà membre » de la liste des candidats est
   une **politesse**, comme `role_locked` de l'onglet Membres — la règle qui fait
   foi vit dans l'agrégat, et un POST direct doit être refusé.
2. **L'ajout par un administrateur est un fait distinct de l'adhésion
   spontanée**, et il est tracé comme tel : `UserAddedToSpaceByAdmin { user_id,
   space_id, profile, added_by }`. Un événement à lui plutôt qu'un champ ajouté à
   `UserSubscribedToSpace`, qui est déjà émis par l'adhésion spontanée où
   `added_by` vaudrait le coach lui-même — un champ qui ne veut pas dire la même
   chose selon l'émetteur ne se lit pas.
3. **L'email de définition du mot de passe part toujours.** La case de la
   maquette est retirée : offrir à un administrateur de créer un compte sans
   aucun moyen d'y accéder, c'est lui offrir de créer un problème silencieux.
4. **La notification « Prévenir le coach qu'il a rejoint l'espace » reste
   optionnelle.** Elle porte sur un coach qui a déjà un compte et déjà accès ;
   c'est une courtoisie, pas une condition d'usage. La case est conservée.
5. **Les deux profils sont attribuables à l'ajout**, Membre comme Admin.
6. **Seul un administrateur accède à l'onglet et à ses endpoints** —
   `SpacePermissions::is_admin()`, sur chacun.

## Widgets existants — ce qui ne se réutilise pas

`coach-search` cherche **parmi les membres de l'espace**
(`list_members_for_space`), ce qui est l'exact inverse du besoin : cet onglet
cherche parmi ceux qui n'y sont **pas**, en marquant ceux qui y sont. Son
gabarit de résultats peut servir de modèle, pas de code.

`find_all_users()` existe au port du cache et rend tous les coachs de la
plateforme. C'est la lecture qu'il faut, sans franchir aucune frontière de BC.

## Questions ouvertes pour la phase 3

- La recherche de candidats doit-elle **exclure** les membres, ou les rendre avec
  un badge ? La maquette les rend — ce qui évite de laisser croire qu'un coach
  n'existe pas alors qu'il est déjà là. À confirmer, car ça change la requête.
- `find_all_users()` rend **tout** l'annuaire. Faut-il une limite au nombre de
  résultats, et que faire d'une recherche vide — tout afficher, ou rien ?
