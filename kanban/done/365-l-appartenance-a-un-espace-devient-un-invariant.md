# L'appartenance à un espace devient un invariant

**Priorité : haute** — fondation des cartes 366 à 374
**Dépend de :** 364 et **375** — sans elle l'agrégat calcule sur une liste incomplète
**Conception :** `docs/specs/space-admin/membres/06-domaine.md`
**Fichiers :** `src/app/spaces/domain/{space.rs, membership_error.rs, domain_event.rs}`,
`src/app/shared_kernel/identity/` pour le value object

## Objectif

L'agrégat `Space` porte déjà `coaches: Vec<Coach>`, et `Coach` porte son
`profile`. Il a donc tout sous la main pour garder l'invariant — il ne le garde
simplement pas : aucune méthode ne modifie une appartenance après coup.

Cette carte lui en donne deux, et l'invariant qui va avec.

## Les quatre règles portées

1. **Un espace a toujours au moins un administrateur.** Le dernier ne peut être
   ni rétrogradé, ni retiré, par personne — lui compris.
2. On ne modifie pas son propre rôle.
3. On ne se retire pas soi-même.
4. On n'agit que sur un membre de l'espace.

Les règles 2 et 3 portent sur l'**acteur**, d'où sa présence dans les
signatures : sans lui, le use case devrait les trancher, c'est-à-dire faire du
métier.

## Ce que les méthodes rendent

```rust
pub struct ChangementDAppartenance {
    pub evenement:       Option<SpacesDomainEvent>,
    pub administrateurs: NombreAdministrateurs,
}
```

Le compte voyage **à côté** de l'événement, jamais dedans : les événements sont
persistés pour toujours par `event_log_feeder`, et un compte inscrit au journal
serait un instantané que plus rien ne rendra vrai. Il n'y a pas non plus de
getter public — le compte n'existe que comme produit d'une opération réussie,
au seul instant où il est exact.

`evenement` est optionnel parce que reposter le rôle courant réussit **sans que
rien ne se passe** : ce n'est pas une erreur, et ça ne doit pas inscrire au
journal un changement qui n'a pas eu lieu.

## Le piège du value object

`NombreAdministrateurs` refuse zéro — le type porte l'invariant. Ce n'est sûr
que parce qu'il n'est **jamais construit au chargement de l'agrégat**, seulement
sur le chemin de succès d'une opération.

Un espace hérité sans administrateur doit continuer à se charger sans erreur.
Refuser de le charger le rendrait inaccessible au lieu de le réparer ; ses
opérations de rôle échoueront proprement, ce qui est le bon symptôme.

## Checklist

- [x] Les champs de `Space` sont **privés**, avec accesseurs en lecture et
      `coaches()` rendant une tranche. Trois consommateurs à adapter — `save()`,
      `register_new_space`, les tests d'intégration — et pas trente : tous les
      autres `space.name` du dépôt portent sur `SpaceSummary`, un autre type
- [x] `NombreAdministrateurs` en `nutype`, `greater_or_equal = 1`
- [x] Vérifié qu'aucun chemin de chargement ne le construit
- [x] `SpaceMembershipError` — trois variantes
- [x] `Space::change_member_role(acteur, cible, nouveau)`
- [x] `Space::remove_member(acteur, cible)`
- [x] `UserDemotedToSpaceUser` et `UserUnsubscribedFromSpace` : variantes, types,
      tags, mapping — seul le retrait franchit la frontière, vers
      `SpacesAppEvent::UserUnsubscribed` qui existait déjà sans émetteur
- [x] Aucune primitive nue dans les nouveaux événements
- [x] L'absence de garde sur les équipes de la cible est commentée dans
      `remove_member`
- [x] Onze tests unitaires, dont :
  - [x] retirer un membre **ordinaire** d'un espace à un seul administrateur
        **réussit** — vu échouer sur la garde naïve posée sur tous les retraits
  - [x] chaque refus vérifie que `coaches` est **inchangé**
  - [x] reposter le rôle courant rend `Ok` sans événement
- [x] `make lint`, `make check-arch`, `make test` passent — 1104 tests

## Ce qu'on a appris en la faisant

**L'invariant était déjà violé en base.** Quatre espaces peuplés n'avaient aucun
administrateur : `is_admin()` y était faux pour tous leurs membres, et la page
d'administration leur aurait été inaccessible de façon définitive. Huit autres
espaces sans administrateur n'ont aucun membre — sans conséquence.

Un type ne peut pas porter un invariant que les données violent : le repli de
`compte()` mentait sur ces quatre espaces. La migration `20260823000002` les
répare en promouvant Bagouze, qui était `SpaceUser` dans exactement ces quatre-là
et dans aucun autre espace cassé.

**La migration répare la prémisse du type plutôt que d'affaiblir le type.**
Zéro administrateur devient inatteignable : `compte()` n'est appelé que sur un
chemin de succès, lequel exige une cible membre, et tout espace peuplé a
désormais un administrateur. Un espace sans membre en a zéro, mais aucune
commande n'y réussit.

Le repli est conservé plutôt qu'un `panic!` : si l'une des deux prémisses tombe
un jour, un compte faux se répare, un `panic` en production non.
