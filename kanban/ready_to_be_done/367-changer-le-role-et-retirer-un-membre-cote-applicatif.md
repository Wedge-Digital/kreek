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

- [ ] Les deux use cases, signature `execute(cmd, repo, bus) -> Result<MembershipOutcome, _>`
- [ ] `#[tracing::instrument(skip_all, fields(cmd = ?cmd))]` sur les deux —
      `skip_all` obligatoire, `repo` n'implémente pas `Debug`
- [ ] Émission par `emettre()`, jamais `.send(` — l'axe 12 de `check-arch` le vérifie
- [ ] L'`app_event_bus` n'est paramètre d'aucun des deux
- [ ] Le repost du rôle courant rend `Ok` sans émettre
- [ ] Tests unitaires sur `FakeRepo`, les huit du tableau de `05-use-cases.md`
- [ ] Chaque test de refus vérifie **aucune écriture et aucun événement** — le
      `FakeRepo` compte ses appels, le bus est lu et doit être vide. Un test qui
      ne vérifie que le type d'erreur passerait sur une implémentation qui écrit
      d'abord et échoue ensuite
- [ ] `make lint`, `make check-arch`, `make test` passent
