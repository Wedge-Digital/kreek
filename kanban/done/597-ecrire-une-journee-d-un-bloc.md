# Écrire une journée d'un bloc

**Épic :** E18 — Appariement des présents · **Ordre :** 3 · **Dépend de :** rien
**Conception :** `docs/specs/appariement-des-presents/calendrier/07-integration.md` (Persistance)

## Le constat

`generate_pairings.rs` écrit une journée **rencontre par rencontre**, une
transaction chacune (`save_pairing`). Une panne à la troisième laisse la
journée à moitié appariée ; deux générations concurrentes se superposent, faute
de rien qui les sérialise. R11 exige au contraire de constater « journée
toujours vide » au moment même d'écrire, et R14 que la génération soit tout ou
rien.

## Ce qui est repris

La branche `sondage-presence` l'a résolu (commit `3d66e971`, sa carte 509) :

- `IMatchDayRepository::save_pairings(match_day_id, &[(Pairing, NewPairingProjection)])`,
  **sans implémentation par défaut** — une boucle sur `save_pairing` compilerait
  et laisserait le vrai dépôt non atomique ;
- dans `match_day_repository.rs` : `SELECT … FOR UPDATE` sur la journée, refus
  `PairingsAlreadyExist` si elle porte une rencontre, puis chaque appariement et
  sa projection dans la même transaction ;
- la fonction privée `ecrire_appariement`, partagée avec `save_pairing` —
  renommée `write_pairing` ;
- `io/repository/tests/test_save_pairings.rs`.

Copie exacte (règle 5), puis renommage des identifiants français. **Attention**
: `match_day_repository.rs` a changé sur `main` depuis la branche (carte 557,
`move_pairing`). On copie les **fonctions**, pas le fichier.

Les doublures de test de `IMatchDayRepository` gagnent la méthode.

## Checklist

- [x] `save_pairings` dans le port, sans défaut
- [x] Implémentation sqlx, `write_pairing` partagé avec `save_pairing`
- [x] `MatchDayRepositoryError::PairingsAlreadyExist` — absent sur `main` au
      2026-10-29, à ajouter
- [x] Doublures de test mises à jour
- [x] `test_save_pairings.rs` : tout est écrit ; journée pourvue refusée ; un
      refus n'écrit rien
- [x] Aucun appelant à cette étape — la carte 598 le branche
- [x] `make lint`, `make check-arch`, `make test`
