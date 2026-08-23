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

- [ ] Route `SPACE_ADMIN_MEMBER_ADD`, contrôleur, garde `is_admin()`
- [ ] Contrôleur sous vingt lignes
- [ ] `notifier` en `#[serde(default)]` — une case décochée n'est pas envoyée
- [ ] `HX-Trigger: memberAdded` avec `coach_id` **et** `name`
- [ ] La ligne candidate re-rendue en « Déjà membre »
- [ ] Journal de session en Alpine, alimenté par le payload, avec son bouton de
      retrait pointant sur la route de la carte 371
- [ ] Tests du harnais handler :
  - [ ] l'endpoint en `SpaceUser` → 403 ; en non-membre → 403
  - [ ] ajout d'un coach **déjà membre**, en admin → **409**
- [ ] `make lint`, `make check-arch`, `make test` passent
