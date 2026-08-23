# Page hôte + onglet Membres — Domaine

**Entrée :** `05-use-cases.md` validé.

## Récapitulatif exhaustif des règles métier

Toutes celles identifiées des phases 1 à 5, sans exception, avec l'origine de
chacune.

| # | Règle | Origine | Où elle vit |
|---|---|---|---|
| 1 | **Un espace a toujours au moins un administrateur.** Le dernier ne peut être ni rétrogradé, ni retiré, par personne — lui compris | phase 1, réponse 2 | agrégat |
| 2 | **On ne modifie pas son propre rôle** | maquette (`select` désactivé sur sa ligne) | agrégat |
| 3 | **On ne se retire pas soi-même** | maquette (pas de bouton sur sa ligne) | agrégat |
| 4 | **On n'agit que sur un membre de l'espace** | implicite, rendue explicite en phase 3 | agrégat |
| 5 | **Retirer un coach est autorisé même s'il a une équipe engagée** en compétition | phase 1, réponse 3 | *aucune garde* — c'est une non-règle, écrite pour qu'on ne l'invente pas |
| 6 | **La réinitialisation de mot de passe envoie un email, y compris à soi-même** | phase 1, réponse 6 | `auth`, hors de cet agrégat |
| 7 | **Seul un administrateur d'espace accède à la page et à ses endpoints** | phase 1, réponse 7 | `SpacePermissions::is_admin()`, couche web |
| 8 | **Un espace privé n'apparaît pas dans l'annuaire** | phase 1, réponse 4 | onglet Paramètres — hors périmètre ici |
| 9 | **L'invitation nominative se fait par recherche, jamais par saisie libre** | phase 1, réponse 5 | onglet Invitations — hors périmètre ici |

Les règles 1 à 4 sont celles que cet agrégat porte. Les règles 5 à 9 sont
listées pour que le périmètre soit lisible : deux relèvent d'autres onglets, une
d'un autre BC, une de la couche web, et une est l'absence délibérée de garde.

## Les value objects

### `NombreAdministrateurs`

```rust
#[nutype(
    validate(greater_or_equal = 1),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AsRef)
)]
pub struct NombreAdministrateurs(usize);
```

**Le type porte l'invariant.** Zéro administrateur est un état qu'un espace n'a
pas le droit d'avoir ; le type refuse donc de le représenter.

Ce n'est sûr que parce qu'il n'est **construit que sur le chemin de succès**
d'une opération. Les trois cas se vérifient un par un :

- promotion → le compte augmente, donc ≥ 1 ;
- rétrogradation → refusée si elle amènerait à 0, donc ≥ 1 après succès ;
- retrait → même chose.

Il n'est **jamais** construit au chargement de l'agrégat depuis la base. Un
espace hérité qui se retrouverait sans administrateur se charge donc sans
erreur, et c'est voulu : refuser de charger une donnée existante rendrait
l'espace inaccessible au lieu de le réparer. Ses opérations de rôle échoueront
proprement, ce qui est le bon symptôme.

## Le retour des méthodes de commande

```rust
pub struct ChangementDAppartenance {
    pub evenement:       SpacesDomainEvent,
    pub administrateurs: NombreAdministrateurs,
}
```

Le compte voyage **à côté** de l'événement, pas dedans.

**Pas dedans, parce que les événements sont persistés pour toujours.**
`event_log_feeder::init(&event_bus, …)` (`main.rs:264`) souscrit au bus interne
et écrit chaque enveloppe dans le journal. Un `administrateurs: 3` inscrit dans
un événement serait un instantané qui ne sera plus jamais vrai, et qui
inviterait un lecteur futur à s'y fier. Un événement dit ce qui s'est passé, pas
l'état qui en résulte.

**Et pas non plus par un getter public.** `nombre_d_administrateurs()` exposé
sur l'agrégat serait une invitation à reprendre au-dehors une décision qui lui
appartient — « je compte, donc je décide si je peux ». Le compte n'existe que
comme **produit d'une opération réussie**, c'est-à-dire au seul instant où il est
vrai et où personne n'a plus rien à décider.

## La forme complète de l'agrégat

Les deux onglets touchent le même `Space`. Sa forme d'ensemble — champs privés,
lectures, les trois commandes, les trois aides privées — est présentée dans
`../ajout-direct/06-domaine.md`, et **la carte 365 la met en place** : elle est
la première à donner des méthodes à cet agrégat, donc la première à devoir
fermer ses champs.

Un préalable s'y ajoute, la **carte 375** : l'agrégat se charge aujourd'hui
incomplet — un membre sans avatar en est silencieusement absent — et par une
requête qui franchit la frontière du BC `auth`. Sans elle, l'invariant du
dernier administrateur **répond faux** au lieu de tomber en panne.

## Les méthodes de commande

```rust
impl Space {
    pub fn change_member_role(
        &mut self,
        acteur: &CoachId,
        cible:  &CoachId,
        nouveau: SpaceProfile,
    ) -> Result<ChangementDAppartenance, SpaceMembershipError>;

    pub fn remove_member(
        &mut self,
        acteur: &CoachId,
        cible:  &CoachId,
    ) -> Result<ChangementDAppartenance, SpaceMembershipError>;
}
```

**Les deux reçoivent l'acteur**, parce que les règles 2 et 3 portent sur lui et
non sur la cible. Une signature sans acteur obligerait le use case à les
trancher — c'est-à-dire à faire du métier.

### `change_member_role` — ordre de vérification

1. `acteur == cible` → `ActeurEstLaCible` (règle 2)
2. la cible n'est pas dans `self.coaches` → `PasMembre` (règle 4)
3. le profil demandé est déjà celui de la cible → `Ok`, **sans événement**
4. rétrogradation d'un administrateur alors qu'il est le seul →
   `DernierAdministrateur` (règle 1)
5. muter le profil dans `self.coaches`, produire `UserPromotedToSpaceAdmin` ou
   `UserDemotedToSpaceUser`

Le cas 3 mérite d'être décidé plutôt que subi : reposter le rôle courant n'est
pas une erreur, mais ne doit pas inscrire au journal un changement qui n'a pas
eu lieu. `ChangementDAppartenance` porte alors le compte inchangé et un
événement absent — d'où un `Option<SpacesDomainEvent>` dans la struct, ou une
variante de retour. **À trancher à l'implémentation** ; le comportement, lui,
est arrêté.

### `remove_member` — ordre de vérification

1. `acteur == cible` → `ActeurEstLaCible` (règle 3)
2. la cible n'est pas dans `self.coaches` → `PasMembre` (règle 4)
3. la cible est administrateur et il est le seul → `DernierAdministrateur`
   (règle 1)
4. retirer de `self.coaches`, produire `UserUnsubscribedFromSpace`

**Aucune vérification sur les équipes de la cible** (règle 5). L'absence de
garde est délibérée et doit être commentée dans le code, sinon quelqu'un
l'ajoutera en croyant réparer un oubli.

## Les erreurs

```rust
#[derive(Debug, PartialEq)]
pub enum SpaceMembershipError {
    DernierAdministrateur,
    ActeurEstLaCible,
    PasMembre,
}
```

Trois variantes, une par règle portée par l'agrégat — la règle 4 donnant
`PasMembre`. Elles vivent dans `domain/membership_error.rs`.

`spaces` n'a **pas** de `DomainError` central : ses erreurs vivent aujourd'hui
dans des enums par use case (`RegisterSpaceError`, `JoinSpacesError`). On crée
donc une erreur *de domaine* pour ce sujet, sans reprendre l'existant ni
prétendre unifier ce qui ne l'est pas.

## Les événements domaine

```rust
UserPromotedToSpaceAdmin  { event_id, user_id, space_id }   // existe, jamais émis
UserDemotedToSpaceUser    { event_id, user_id, space_id }   // nouveau
UserUnsubscribedFromSpace { event_id, user_id, space_id }   // nouveau
```

**Deux événements plutôt qu'un portant le rôle cible.**
`grep UserDemotedToSpaceUser` répond à une question ; `grep UserRoleChanged`
oblige à lire les charges utiles. C'est le journal qui décide, pas l'économie de
variantes.

Aucun ne porte de primitive nue — trois identifiants, tous des value objects.

### Le doublon de type à corriger d'abord

`USER_INVITED_IN_SPACE` et `USER_SUBSCRIBED_TO_SPACE` valent tous deux
`"UserRegisteredInSpace"`. Deux événements distincts partagent leur type : tout
listener qui filtre dessus attrape les deux.

Latent aujourd'hui — `UserInvitedInSpace` n'est jamais émis. L'onglet
Invitations sera le premier à l'émettre. **Corriger avant, pas pendant** : cette
carte touche déjà ce fichier, c'est le bon moment et ça coûte une ligne.

### Ce que le domaine ne sait pas

`to_app_event()` mappe `UserUnsubscribedFromSpace` vers
`SpacesAppEvent::UserUnsubscribed` — qui **existe déjà** dans l'enum et que
personne n'émet. Promotion et rétrogradation n'ont **pas** de mapping : le rôle
d'espace est relu en direct par `SpacePermissions` à chaque requête, aucun BC
n'en cache de copie.

Le domaine ignore tout de cette distinction. Elle est dans le mapping, c'est-à-
dire dans la couche IO, où elle a sa place.

## Tests unitaires

Un par règle, plus les chemins nominaux. Sur l'agrégat seul — pas de dépôt, pas
de bus, pas d'async.

| Test | Attendu |
|---|---|
| promotion d'un membre | `UserPromotedToSpaceAdmin`, compte +1 |
| rétrogradation, deux administrateurs | `UserDemotedToSpaceUser`, compte −1 |
| rétrogradation du seul administrateur | `DernierAdministrateur`, **état inchangé** |
| retrait du seul administrateur | `DernierAdministrateur`, **état inchangé** |
| retrait d'un membre ordinaire, un seul administrateur | `Ok` — l'invariant ne concerne que les administrateurs |
| acteur == cible, sur le rôle | `ActeurEstLaCible` |
| acteur == cible, sur le retrait | `ActeurEstLaCible` |
| cible absente de l'espace | `PasMembre` |
| reposter le rôle courant | `Ok`, aucun événement |
| `NombreAdministrateurs::try_new(0)` | `Err` |

**« État inchangé » est la moitié qui compte.** Un test qui vérifie seulement le
type d'erreur passerait sur une implémentation qui mute d'abord et valide
ensuite. Chaque test de refus relit `self.coaches` et vérifie qu'il est
identique.

L'avant-dernière ligne mérite d'exister pour elle-même : **retirer un membre
ordinaire d'un espace qui n'a qu'un administrateur doit réussir**. C'est le cas
qu'une lecture rapide de la règle 1 fait rater, en gardant l'invariant sur tous
les retraits au lieu des seuls administrateurs.

## Questions ouvertes pour la phase 7

- Le repost du rôle courant rend `Ok` sans événement. Le contrôleur doit-il
  quand même re-rendre la ligne ? Rendre 204 laisserait le `<kreek-select>`
  dans l'état où le client l'a mis, ce qui est correct — mais un 204 sur une
  action qui a « marché » se lit mal dans un journal.
