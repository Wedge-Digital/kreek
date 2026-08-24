# Le dépôt sait chercher dans l'annuaire de la plateforme

**Priorité : haute**
**Dépend de :** rien — parallélisable avec 376
**Conception :** `docs/specs/space-admin/ajout-direct/07-integration.md`
**Fichiers :** `src/app/spaces/domain/space_repository_port/space_repository_port.rs`,
`io/repository/space_repository.rs`, `io/repository/sql/space/search_platform_coaches.sql`

## Objectif

```rust
async fn search_platform_coaches(&self, space_id: &SpaceId, q: &str, limite: i64)
    -> Result<Vec<CandidateRow>, SpaceRepositoryError>;
```

`CandidateRow` porte `est_membre` : les membres de l'espace sont **rendus, pas
exclus**. Les exclure laisserait croire qu'un coach n'existe pas alors qu'il est
déjà là, et l'administrateur chercherait à créer un compte qui existe.

## Le piège de la jointure — la raison d'être de cette carte

```sql
LEFT JOIN spaces__user_space m
       ON m.coach_id = u.id AND m.space_id = $1
```

**`space_id` est dans la condition de jointure, pas dans le `WHERE`.** L'y
déplacer transforme la jointure externe en jointure interne : la recherche ne
rend plus que les membres, c'est-à-dire l'exact inverse du besoin.

Sans erreur, sans exception, avec une liste qui a l'air d'une liste.

## Les garde-fous ne sont pas négociables

Plafond de **vingt** résultats, seuil de **deux** caractères. Tous deux en dur
côté serveur, jamais en paramètre de requête — les exposer permettrait à
n'importe quel appelant de redemander l'annuaire entier.

Le seuil s'applique **avant** la lecture, pas après : un seuil qui filtrerait le
résultat aurait déjà interrogé l'annuaire.

## Pas d'index pour l'instant

`ILIKE '%q%'` n'est pas ancré, donc aucun index B-tree ne servirait, et
`spaces__user_cache` est petite. À reconsidérer quand l'annuaire grossira — pas
avant, et pas sans mesure.

## Checklist

- [x] Méthode au port, `CandidateRow` avec `est_membre`
- [x] SQL dans un fichier dédié, jointure sur `spaces__user_cache`
- [ ] ~~`sqlx::query_as!` de préférence à `query_as`~~ — même motif qu'en carte
      366 : le BC entier utilise la forme non-macro, et l'adopter ici seule
      créerait une exception
- [x] Tests d'intégration sur une vraie `PgPool` :
  - [x] un membre de l'espace est rendu avec `est_membre = true`
  - [x] un non-membre est rendu avec `est_membre = false`
  - [x] **un membre d'un *autre* espace est rendu comme non-membre**
  - [x] la recherche par email trouve
  - [x] vingt-cinq correspondants en rendent vingt
  - [x] l'ordre est celui du pseudo
- [x] Le piège **vu se refermer** en déplaçant `space_id` dans le `WHERE`
- [x] `make lint`, `make check-arch`, `make test` passent — 1154 tests

## Ce qu'on a appris en la faisant

**Le piège est plus large que la carte ne le disait.** Elle annonçait une panne
discrète — « une liste qui a l'air d'une liste ». Mesuré : déplacer `space_id`
dans le `WHERE` fait échouer **cinq tests sur cinq**. La jointure devient interne,
plus aucun non-membre n'est rendu, et la recherche cesse entièrement de
fonctionner.

Le défaut ne resterait discret que sur un espace dont tous les candidats sont
déjà membres — le cas d'usage le plus rare. La carte décrivait le pire scénario,
pas le plus probable.

**Le port a cinq implémentations, pas quatre.** `FakeSpaceRepo`, créé en carte
367, s'est ajouté aux trois doublures d'origine. Le volume d'une carte de dépôt
reste dans les stubs, pas dans la méthode.
