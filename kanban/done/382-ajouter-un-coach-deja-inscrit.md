# Ajouter un coach déjà inscrit

**Priorité : haute** — c'est le premier des deux chemins de l'onglet
**Dépend de :** 378 et 381
**Conception :** `docs/specs/space-admin/ajout-direct/07-integration.md`
**Fichiers :** `io/web/controllers/add_member_controller.rs`, `routes.rs`,
`router.rs`, la page hôte pour le journal de session

## Objectif

Le bouton « Ajouter » d'une ligne candidate, et le journal de session qui s'en
nourrit.

```
POST …/admin/members/add   { coach_id, profile, notifier }
  → 200, la ligne candidate re-rendue en « Déjà membre »
    HX-Trigger: memberAdded {coach_id, name}
```

**La ligne est re-rendue, pas retirée.** Le coach existe toujours dans
l'annuaire, il est simplement devenu membre ; le faire disparaître laisserait
croire à une suppression.

## Le journal de session

**Aucun endpoint, aucun VM, aucun template serveur.** Une liste Alpine dans la
page hôte, alimentée par `memberAdded`, perdue au rechargement — c'est le sens
exact de « ajoutés pendant cette session ».

Son bouton « Retirer » appelle `SPACE_ADMIN_MEMBER_REMOVE`, écrite par la carte
371. Retirer quelqu'un qu'on vient d'ajouter et retirer un membre de longue date
sont la même opération ; l'écrire deux fois serait une duplication déguisée en
fonctionnalité distincte.

## Il affiche depuis le payload, jamais d'une relecture

`spaces__user_cache` est alimenté **de façon asynchrone**, par
`user_created_listener`. La liste des membres lit `spaces__user_space` jointe à
ce cache : un compte tout juste créé peut donc être membre **sans encore
apparaître** dans la liste.

Le journal, lui, affiche depuis le payload de `memberAdded` qu'il tient déjà.
L'écran dit vrai immédiatement, la liste se rattrape au rafraîchissement
suivant. **C'est la raison d'être du champ `name` dans l'événement** — sans lui,
le journal devrait relire, et retomberait dans la course qu'il masque.

## Traduction des erreurs

| Erreur | Statut |
|---|---|
| `DejaMembre` | **409** — requête bien formée, état qui la refuse |
| `EspaceInconnu` | 404 |
| `Database` | 500 |

Fragment HTML d'erreur, jamais de JSON.

## Ce qui ne vient pas du client

`space_id` est celui de `SpacePermissions`, déjà validé. L'acteur vient
d'`AuthSession` — il finit dans `added_by` de l'événement, et une identité qui
transite par le client est une identité réécrivable.

## Checklist

- [x] Route `SPACE_ADMIN_MEMBER_ADD`, contrôleur, garde `is_admin()`
- [x] `notifier` en `#[serde(default)]` — une case décochée n'est pas envoyée
- [x] `HX-Trigger: memberAdded` avec `coach_id` **et** `name`
- [x] La ligne candidate re-rendue en « Déjà membre », pas retirée
- [x] Journal de session en Alpine, alimenté par le payload, avec son bouton de
      retrait pointant sur la route de la carte 371
- [x] Tests du harnais : ajout nominal, contrat de l'événement, 409 sur un coach
      déjà membre, 403 pour un membre simple
- [x] `make lint`, `make check-arch`, `make test` passent — 1199 tests

## Ce qu'on a appris en la faisant

**Le trait de l'hôte gagne deux méthodes.** Le gabarit d'e-mail a besoin d'URL
**absolues** — un chemin ne mène nulle part dans une boîte mail — et `space_home`
n'en rend pas. `space_url()` et `app_url()` s'ajoutent, et l'adapter porte le
domaine. Le BC ne le connaît toujours pas.

**Un test passait pour la mauvaise raison, et c'est l'alphabet ULID qui l'a
révélé.** Crockford exclut `I`, `L`, `O` et `U` : des identifiants « parlants »
comme `01JSOLITAIRE…` sont refusés par le value object, et le contrôleur rend
400. Deux tests l'ont dit tout de suite — mais celui qui vérifie qu'un membre
simple ne peut pas ajouter passait quand même, la garde répondant **avant** la
validation. Il aurait continué à passer si la garde avait disparu et que seul le
400 restait.
