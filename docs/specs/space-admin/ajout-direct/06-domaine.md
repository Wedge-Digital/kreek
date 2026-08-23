# Onglet Ajout direct — Domaine

**Entrée :** `05-use-cases.md` validé.

## Récapitulatif exhaustif des règles métier

| # | Règle | Origine | Où elle vit |
|---|---|---|---|
| 1 | **On ne peut pas ajouter un coach déjà membre** | phase 2 | agrégat — `DejaMembre` |
| 2 | **L'ajout par un administrateur est un fait distinct de l'adhésion spontanée**, et trace qui a ajouté qui | phase 2 | événement `UserAddedToSpaceByAdmin` |
| 3 | **L'email de définition du mot de passe part toujours** | phase 2 | `auth`, use case |
| 4 | **La notification « il a rejoint l'espace » reste optionnelle** | phase 2 | use case `spaces` |
| 5 | **Les deux profils sont attribuables à l'ajout** | phase 2 | aucune garde — tout profil est valide |
| 6 | **Aucun plafond de membres par espace** | phase 5 | aucune garde — décidé, pas oublié |
| 7 | **Seul un administrateur accède à l'onglet et à ses endpoints** | phase 2 | couche web |

Les règles 5 et 6 sont des **non-règles**, écrites pour qu'on ne les invente
pas. Un plafond ajouté plus tard, une fois des espaces au-delà du seuil en
production, coûte bien plus qu'un plafond posé d'emblée — le noter évite qu'on
croie à un oubli.

## L'agrégat, dans sa forme complète

Les deux onglets touchent le même agrégat. Le voici **après les deux**, plutôt
que par la seule addition de celui-ci : un agrégat se conçoit d'un bloc, et la
troisième méthode révèle souvent que les deux premières avaient la mauvaise
signature.

```rust
pub struct Space {
    id:      SpaceId,
    name:    SpaceName,
    logo:    CloudinaryImage,
    coaches: Vec<Coach>,
}

impl Space {
    pub fn new(id: SpaceId, name: SpaceName, logo: CloudinaryImage,
               coaches: Vec<Coach>) -> Self;

    pub fn id(&self)      -> &SpaceId;
    pub fn name(&self)    -> &SpaceName;
    pub fn logo(&self)    -> &CloudinaryImage;
    pub fn coaches(&self) -> &[Coach];

    pub fn add_member(&mut self, acteur: &CoachId, nouveau: &CoachId,
                      profil: SpaceProfile)
        -> Result<ChangementDAppartenance, SpaceMembershipError>;

    pub fn change_member_role(&mut self, acteur: &CoachId, cible: &CoachId,
                              nouveau: SpaceProfile)
        -> Result<ChangementDAppartenance, SpaceMembershipError>;

    pub fn remove_member(&mut self, acteur: &CoachId, cible: &CoachId)
        -> Result<ChangementDAppartenance, SpaceMembershipError>;

    fn membre(&self, id: &CoachId) -> Option<&Coach>;
    fn nombre_d_administrateurs(&self) -> usize;
    fn est_dernier_administrateur(&self, id: &CoachId) -> bool;
}
```

### Les champs deviennent privés

Ils sont `pub` aujourd'hui, ce que le `CLAUDE.md` interdit — « les agrégats
n'exposent pas de référence mutable vers leur état interne ».

Trois méthodes qui gardent un invariant ne servent à rien tant que
`space.coaches.push(…)` compile. Le coût est nul : un seul test lit ce champ
dans tout le dépôt.

`coaches()` rend une **tranche**, pas un `Vec` : les VMs et les tests lisent,
personne ne mute hors des trois commandes.

### Les trois aides restent privées

C'est la conclusion de la phase 5 de l'onglet Membres. Le nombre
d'administrateurs n'existe que comme **produit d'une opération réussie**, jamais
comme quelque chose qu'on interroge pour décider soi-même — un getter public
serait une invitation à reprendre au-dehors une décision qui appartient à
l'agrégat.

## `add_member` — ordre de vérification

1. `nouveau` est déjà dans `coaches` → `DejaMembre`
2. l'ajouter, produire `UserAddedToSpaceByAdmin`

`acteur` **ne sert aucune règle** : rien n'interdit à un administrateur
d'ajouter qui il veut. Il est là pour la trace, l'ajout se passant du
consentement du coach.

### Pourquoi `DejaMembre` n'est pas une redondance

Sans cette vérification, le doublon serait refusé par la clé primaire composite
de `spaces__user_space` — une règle métier rendue par une erreur SQL brute,
illisible et intraduisible en 409. Le badge « Déjà membre » de la liste des
candidats est une **politesse** ; un POST direct doit être refusé par le
domaine.

`SpaceMembershipError` gagne donc une quatrième variante, aux côtés de
`DernierAdministrateur`, `ActeurEstLaCible` et `PasMembre`.

## L'événement

```rust
UserAddedToSpaceByAdmin {
    event_id: EventId,
    user_id:  CoachId,
    space_id: SpaceId,
    profile:  SpaceProfile,
    added_by: CoachId,
}
```

**Un événement domaine distinct, un app event partagé** avec
`UserSubscribedToSpace` : tous deux vers `SpacesAppEvent::UserSubscribed`.

Le domaine sépare les deux faits — le journal doit les distinguer d'un `grep`,
pas par la lecture des charges utiles. L'extérieur n'a besoin que de l'effet :
un coach est membre.

Aucune primitive nue : quatre value objects. `notifier` **n'y figure pas** —
c'est l'état d'une case à cocher au moment d'un clic, et `event_log_feeder`
persiste chaque enveloppe pour toujours.

## Un préalable — carte 375

L'agrégat n'est chargé par personne aujourd'hui : `find_by_id` du BC `spaces`
n'a d'autre appelant que son propre test. Deux défauts y dorment, et les trois
méthodes ci-dessus en feraient le premier usage réel :

- **un membre sans avatar est silencieusement absent de `coaches`** — le
  `let-else` du chargement traite une icône `NULL` comme un membre manquant.
  L'invariant du dernier administrateur ne tomberait pas en panne : il
  **répondrait faux**, et laisserait un espace perdre son dernier admin ;
- **le SQL joint `auth__users`**, table du BC `auth` — violation de
  souveraineté, et rédhibitoire pour un BC extractible.

Carte **375**, à faire avant 365 et 367. Elle n'est pas un nettoyage adjacent :
sans elle, les règles de cette phase calculent sur une liste incomplète.

## Tests unitaires

Sur l'agrégat seul — pas de dépôt, pas de bus, pas d'async.

| Test | Attendu |
|---|---|
| ajout d'un non-membre en Membre | `UserAddedToSpaceByAdmin`, compte d'admins inchangé |
| ajout d'un non-membre en Admin | compte +1 |
| ajout d'un coach **déjà membre** | `DejaMembre`, **`coaches` inchangé** |
| ajout d'un coach déjà membre avec un **autre profil** | `DejaMembre` — ce n'est pas un changement de rôle déguisé |
| `added_by` porte l'acteur | le champ n'est pas oublié |

L'avant-dernier mérite d'exister : ajouter en Admin quelqu'un qui est déjà
Membre **n'est pas** une promotion. Deux opérations distinctes, deux intentions
distinctes, et confondre les deux ferait de l'ajout un chemin détourné pour
changer un rôle — sans passer par `change_member_role`, donc sans sa règle de
dernier administrateur.

## Question ouverte pour la phase 7

- L'ajout émet `memberAdded`, qui rafraîchit la liste des membres. Mais le
  compte tout juste créé peut ne pas encore être dans `spaces__user_cache`. Le
  contrôleur doit-il rendre quelque chose qui permette au journal de session
  d'afficher la ligne sans attendre — c'est ce que la phase 2 a retenu — ou le
  payload de l'événement suffit-il ?
