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

- [x] Les deux routes, les deux contrôleurs, garde `is_admin()` sur chacun
- [x] Chaque contrôleur sous vingt lignes — construire, appeler, répondre
- [x] `coach_id` validé par `CoachId::try_new()`, 400 sinon
- [x] `ChangeRoleForm.profile` converti par `SpaceProfile::try_from(&str)`
- [x] Les cinq traductions d'erreur, dont le 409
- [x] Les deux `HX-Trigger`, posés aussi sur le repost sans effet
- [x] Neuf tests du harnais, dont la matrice d'autorisation
- [ ] ~~POST rôle sur le **dernier administrateur**, en admin → 409~~ —
      **impossible à construire**, voir ci-dessous. Remplacé par un test qui
      constate la fermeture anticipée
- [x] POST retrait sur **soi-même**, en admin → **403**, avec le message du
      domaine — le refus vient bien de la règle, pas de la couche web
- [x] Les tests frappent l'endpoint **sans passer par l'interface**, et la garde
      a été **vue tomber** : `is_admin()` retirée, deux tests rougissent
- [x] `make lint`, `make check-arch`, `make test` passent — 1137 tests

## Ce qu'on a appris en la faisant

**`DernierAdministrateur` est inatteignable depuis le web.** Trois conditions
seraient nécessaires ensemble : un acteur administrateur, distinct de la cible,
et une cible **seule** administratrice. Si la cible est seule et l'acteur
distinct, l'acteur n'est pas administrateur — `is_admin()` l'arrête avant le use
case.

L'espace ne peut donc pas perdre son dernier administrateur, mais par la
combinaison `is_admin()` + `ActeurEstLaCible`, **pas par la règle qui porte ce
nom**. Celle-ci garde sa valeur — elle protège les autres appelants, et les tests
d'agrégat la couvrent — mais ce n'est pas elle qui tient la porte ici.

**Un test passait pour la mauvaise raison.** Le membre simple tentait de se
retirer *lui-même* : `ActeurEstLaCible` rendait 403 à la place de la garde, et le
test passait même sans `is_admin()`. Découvert en retirant la garde — un seul
test rougissait au lieu de deux. Un test qui passe pour la mauvaise raison est
pire qu'un test qui échoue : il donne une couverture qui n'existe pas.

**Le retour des use cases n'est pas utilisé par ces contrôleurs.** Pour re-rendre
la ligne il faut le pseudo, l'email et les initiales, que le use case ne rend
pas ; la relecture de la liste s'impose, et elle donne le compte d'administrateurs
à jour du même coup. La décision de la phase 5 — faire rendre le compte par le
use case — ne paie donc pas ici. Elle n'est pas fausse, elle est sans emploi : si
aucun autre appelant ne s'en sert, les deux signatures pourront se simplifier.
