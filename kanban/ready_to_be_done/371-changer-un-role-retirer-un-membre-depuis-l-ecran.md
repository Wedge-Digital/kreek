# Changer un rôle, retirer un membre, depuis l'écran

**Priorité : haute** — c'est la carte qui rend l'onglet utile
**Dépend de :** 367 et 369
**Conception :** `docs/specs/space-admin/membres/07-integration.md`
**Fichiers :** `io/web/controllers/{change_member_role_controller.rs, remove_member_controller.rs}`,
`routes.rs`, `router.rs`

## Objectif

Les deux actions de la ligne. Le contrôleur est un traducteur HTTP : il
construit la commande, appelle le use case, bâtit la réponse.

## Les réponses

```
POST …/role    → 200, la ligne re-rendue     HX-Trigger: memberRoleChanged
POST …/remove  → 200, corps vide             HX-Trigger: memberRemoved
```

Le changement de rôle **re-rend la ligne** plutôt que de ne rien renvoyer : le
serveur est seul à savoir que le sélecteur du dernier administrateur doit se
figer. Le use case rend le nombre d'administrateurs postérieur, qui sert à
recalculer `role_locked`.

Le retrait rend un **corps vide** avec `hx-swap="outerHTML"` : la ligne sait se
supprimer, pas besoin de re-rendre la liste.

**Le repost du rôle courant re-rend la ligne comme tout autre succès.** Aucun
événement n'a été émis — rien ne s'est passé — mais la réponse est uniforme : un
204 sur une action réussie se lit comme un trou dans un journal, et force le
client à distinguer deux formes de succès pour rien.

## La traduction des erreurs

| Erreur | Statut |
|---|---|
| `ActeurEstLaCible` | 403 |
| `DernierAdministrateur` | **409** |
| `PasMembre` | 404 |
| `EspaceInconnu` | 404 |
| `Database` | 500 |

409 et non 400 pour `DernierAdministrateur` : la requête est bien formée, c'est
l'**état** de l'espace qui la refuse, et cet état peut changer.

Fragment HTML d'erreur, jamais de JSON — ce sont des réponses HTMX.

## Ce qui ne vient pas du client

`space_id` est celui de `SpacePermissions`, déjà validé — aucun contrôleur ne
l'extrait une seconde fois, sous peine d'avoir deux sources de vérité pour la
même valeur.

L'acteur vient d'`AuthSession`. Deux règles portent sur lui.

## Checklist

- [ ] Les deux routes, les deux contrôleurs, garde `is_admin()` sur chacun
- [ ] Chaque contrôleur sous vingt lignes — construire, appeler, répondre
- [ ] `coach_id` validé par `CoachId::try_new()`, 400 sinon
- [ ] `ChangeRoleForm.profile` converti par `SpaceProfile::try_from(&str)`
- [ ] Les cinq traductions d'erreur, dont le 409
- [ ] Les deux `HX-Trigger`, posés aussi sur le repost sans effet
- [ ] Tests du harnais handler (`src/web/test_harness.rs`) :
  - [ ] les cinq endpoints de la page en `SpaceUser` → 403
  - [ ] les cinq en non-membre → 403
  - [ ] POST rôle sur le **dernier administrateur**, en admin → **409**
  - [ ] POST retrait sur **soi-même**, en admin → **403**
- [ ] Les deux derniers frappent l'endpoint **sans passer par l'interface** :
      c'est ce qui prouve que le grisage de la carte 369 n'est pas la garde
- [ ] `make lint`, `make check-arch`, `make test` passent
