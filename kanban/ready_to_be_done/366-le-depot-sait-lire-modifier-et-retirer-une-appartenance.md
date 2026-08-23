# Le dépôt sait lire, modifier et retirer une appartenance

**Priorité : haute**
**Dépend de :** rien — parallélisable avec 365
**Conception :** `docs/specs/space-admin/membres/07-integration.md`
**Fichiers :** `src/app/spaces/domain/space_repository_port/space_repository_port.rs`,
`src/app/spaces/io/repository/space_repository.rs`, `io/repository/sql/space/`

## Objectif

Trois méthodes que le port n'a pas :

```rust
async fn list_members_with_profile(&self, space_id: &SpaceId)
    -> Result<Vec<SpaceMemberRow>, SpaceRepositoryError>;
async fn update_member_profile(&self, space_id, coach_id, profile) -> …;
async fn delete_member(&self, space_id, coach_id) -> …;
```

## Une méthode de lecture de plus, pas une existante élargie

`list_members_for_space` rend des `User` **sans leur profil** — son SQL ne
sélectionne pas `m.profile`. L'onglet a besoin du rôle sur chaque ligne.

Élargir l'existante ferait porter à son appelant actuel — le sélecteur de coachs
— une colonne dont il n'a que faire. D'où une méthode distincte, dont le SQL est
celui de l'autre plus une colonne.

`SpaceMemberRow` est un **DTO de lecture** : primitives assumées, aucun
invariant à protéger.

## Aucune migration

Tout existe : `spaces__user_space(space_id, coach_id, profile)`, clé primaire
composite, et `spaces__user_cache`. La visibilité, qui en demanderait une, est
hors périmètre.

## Le piège de la clé composite

`spaces__user_space` a pour clé `(space_id, coach_id)`. Une requête d'écriture
qui oublierait `space_id` toucherait le même coach dans **tous** ses espaces —
et passerait tous les tests tant qu'ils n'utilisent qu'un espace.

D'où deux tests qui n'ont l'air de rien et qui sont la raison d'être de cette
carte.

## Checklist

- [ ] Les trois méthodes au port, avec `SpaceMemberRow`
- [ ] Trois fichiers SQL sous `io/repository/sql/space/`
- [ ] `sqlx::query_as!` de préférence à `query_as` — vérification à la compilation
- [ ] Tests d'intégration sur une vraie `PgPool`, jamais de mock sqlx :
  - [ ] `list_members_with_profile` rend le profil de chaque membre
  - [ ] l'ordre est celui du pseudo, indépendant de l'ordre d'insertion
  - [ ] `update_member_profile` sur un membre d'un **autre** espace touche
        **zéro ligne**
  - [ ] `delete_member` sur un membre d'un **autre** espace touche **zéro ligne**
- [ ] `make lint`, `make check-arch`, `make test` passent
