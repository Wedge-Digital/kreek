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

- [x] Les trois méthodes au port, avec `SpaceMemberRow`
- [x] Trois fichiers SQL sous `io/repository/sql/space/`
- [ ] ~~`sqlx::query_as!` de préférence à `query_as`~~ — **écarté, et c'est un
      choix**. Le BC entier utilise `query_as` non-macro. Adopter la macro ici
      seule créerait une exception dans un dépôt qui n'en a aucune, et imposerait
      `DATABASE_URL` à la compilation de ce fichier. À faire pour le BC entier ou
      pas du tout — pas à trancher dans une carte de trois méthodes
- [x] Tests d'intégration sur une vraie `PgPool`, jamais de mock sqlx :
  - [x] `list_members_with_profile` rend le profil de chaque membre
  - [x] l'ordre est celui du pseudo, indépendant de l'ordre d'insertion
  - [x] **ajouté** : la liste ne rend que les membres de l'espace demandé — même
        famille que les deux suivants, la lecture pouvant déborder comme les
        écritures
  - [x] `update_member_profile` sur un membre d'un **autre** espace touche
        **zéro ligne**
  - [x] `delete_member` sur un membre d'un **autre** espace touche **zéro ligne**
- [x] Les deux derniers **vus échouer** en retirant `space_id` du `WHERE`, avec
      le message qui dit quoi réparer
- [x] `make lint`, `make check-arch`, `make test` passent — 1109 tests

## Ce qu'on a appris en la faisant

**Le volume était dans les doublures.** Trois méthodes au port, et **neuf
stubs** à écrire dans les trois implémentations de test du trait —
`SpaceRepoOk`, `SpaceRepoNameTaken`, `FakeRepo`. C'est le prix d'un trait large,
et il se paiera à chaque méthode ajoutée.

**Le montage des deux tests du piège compte autant que leur assertion.** Il faut
un coach membre de **deux** espaces avec le même profil : sans lui, une écriture
qui omet `space_id` ne se voit pas, et le test passe en donnant l'illusion de
couvrir le cas.
