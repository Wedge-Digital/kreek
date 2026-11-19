# Changer le rôle et retirer un membre, côté applicatif

**Priorité : haute**
**Dépend de :** 365 et 366
**Conception :** `docs/specs/space-admin/membres/05-use-cases.md`
**Fichiers :** `src/app/spaces/use_cases/{change_member_role_use_case.rs, remove_member_use_case.rs}`

## Objectif

Deux use cases, une seule forme :

```
charger l'agrégat → appeler la méthode domaine → persister → émettre
→ rendre le nombre d'administrateurs postérieur
```

Le compte est lu **sur l'agrégat muté**, pas relu en base : l'agrégat vient
d'appliquer le changement, il est la source la plus fraîche qui soit. Il sert au
contrôleur à recalculer `role_locked` lors du re-rendu de la ligne — rétrograder
l'avant-dernier administrateur fige le sélecteur du dernier.

## Les commandes

Aucune primitive nue. **L'acteur vient d'`AuthSession`, jamais du formulaire** :
deux règles portent sur lui, et une identité qui transite par le client est une
identité réécrivable.

## Ce que les use cases ne font pas

**Ils n'émettent aucun app event.** `emettre()` publie sur le bus **interne** ;
c'est le publisher qui convertit et publie sur l'`app_event_bus`. Le retrait
franchit donc la frontière vers `competitions` sans que le use case le sache.

**Ils ne décident aucune règle.** « Est-ce le dernier administrateur ? » vit
dans l'agrégat. Le use case charge, appelle, persiste, émet.

**Ils ne réinterprètent pas les erreurs du domaine**, ils les transportent :
`From<SpaceMembershipError>` vers une variante `Metier`. C'est le contrôleur qui
choisira le statut HTTP.

## Ce qu'on accepte, et qu'on écrit plutôt que de le taire

Le domain event part sur un bus en mémoire **après** l'écriture. Si le processus
tombe entre les deux, le coach est retiré de l'espace et reste administrateur de
compétition.

Ce n'est pas une régression — c'est le comportement de tous les listeners
cross-BC du projet — et `competitions_members` reste reconstructible depuis
`spaces__user_space`. À noter dans le code, pas à corriger ici.

## Checklist

- [x] Les deux use cases, signature `execute(cmd, repo, bus)`
- [x] ~~`-> Result<MembershipOutcome, _>`~~ → **`Result<NombreAdministrateurs, _>`**.
      Le domaine produit déjà le value object ; un `usize` nu dans un retour
      applicatif aurait été une primitive là où le type existe, et le wrapper
      n'ajoutait qu'un niveau d'indirection
- [x] `#[tracing::instrument(skip_all, fields(cmd = ?cmd))]` sur les deux
- [x] Émission par `emettre()`, jamais `.send(` — vérifié, aucun dans
      `use_cases/`
- [x] L'`app_event_bus` n'est paramètre d'aucun des deux
- [x] Le repost du rôle courant rend `Ok` **sans écrire ni émettre**
- [x] Treize tests unitaires sur un `FakeSpaceRepo` mutualisé
- [x] Chaque test de refus vérifie **aucune écriture et aucun événement** — vu
      échouer sur une implémentation qui écrit avant de valider
- [x] `make lint`, `make check-arch`, `make test` passent — 1122 tests

## Ce qu'on a appris en la faisant

**Le compteur d'écritures couvre plus que les refus.** En introduisant
l'implémentation « écrire d'abord, valider ensuite », **six tests sur sept** ont
rougi — pas seulement les deux de refus. Les chemins nominaux comptent aussi
leurs écritures, et en voyaient deux au lieu d'une. Le compteur vérifie donc
l'ordre des opérations partout, pas seulement là où on l'attendait.

**Le publisher n'a rien demandé.** Il désérialise n'importe quel
`SpacesDomainEvent` et applique `to_app_event()` : le mapping écrit en carte 365
suffit à faire franchir la frontière au retrait. Le use case l'ignore
complètement, ce qui est exactement l'intention de la règle — l'`app_event_bus`
ne remonte jamais jusqu'à lui.

**Le `FakeSpaceRepo` est mutualisé** dans `use_cases/test_doubles.rs`, sur le
patron du BC `teams`. Il servira aux use cases de l'onglet Ajout direct.
