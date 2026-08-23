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

- [ ] Les champs de `Space` deviennent **privés**, avec des accesseurs en
      lecture et `coaches()` rendant une **tranche** — trois méthodes qui gardent
      un invariant ne servent à rien tant que `space.coaches.push(…)` compile.
      Un seul test lit ce champ dans tout le dépôt
- [ ] `NombreAdministrateurs` en `nutype`, `greater_or_equal = 1`
- [ ] Vérifié qu'aucun chemin de chargement ne le construit
- [ ] `SpaceMembershipError` — trois variantes, dans `domain/membership_error.rs`
- [ ] `Space::change_member_role(acteur, cible, nouveau)`
- [ ] `Space::remove_member(acteur, cible)`
- [ ] `UserDemotedToSpaceUser` et `UserUnsubscribedFromSpace` ajoutés à l'enum,
      avec leur type, leurs tags et leur mapping `to_app_event()` —
      `UserUnsubscribedFromSpace` vers `SpacesAppEvent::UserUnsubscribed`, qui
      **existe déjà** ; les deux changements de rôle vers `None`
- [ ] Aucune primitive nue dans les nouveaux événements
- [ ] L'absence de garde sur les équipes de la cible est **commentée** dans
      `remove_member`, sinon quelqu'un l'ajoutera en croyant réparer un oubli
- [ ] Tests unitaires — les dix du tableau de `06-domaine.md`, dont :
  - [ ] retirer un membre **ordinaire** d'un espace à un seul administrateur
        **réussit** — l'invariant ne porte que sur les administrateurs
  - [ ] chaque refus vérifie que `self.coaches` est **inchangé**, pas seulement
        le type d'erreur
  - [ ] reposter le rôle courant rend `Ok` sans événement
- [ ] `make lint`, `make check-arch`, `make test` passent
