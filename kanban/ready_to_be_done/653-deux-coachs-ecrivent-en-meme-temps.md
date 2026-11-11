# Deux coachs écrivent en même temps

**Épic :** E22 — Le mode « Chacun son équipe » · **Ordre :** 2 · **Dépend de :** 652
**Conception :** `docs/specs/saisie-mobile/E22-cadrage.md` (§ 1 « L'écriture concurrente », § 9, § 12.2)

## Le constat

Un flux de compte rendu n'a qu'une version : la contrainte
`match_report_es_version` refuse toute seconde écriture simultanée
(`src/app/match_report/io/repository/match_report_repository.rs:393-400`, `522-528`),
rendue en 409 avec `HX-Refresh` (`io/web/write_conflict.rs`). En « Je saisis
tout », un seul saisisseur : c'est rare. En mode B, deux coachs écrivent en même
temps dans deux camps indépendants, et le rechargement effacerait ce que le
second était en train de saisir — un paragraphe de résumé, par exemple.

## Ce qui change

- Les use cases d'écriture **d'un camp** (fan factor, coups de pouce, action,
  suppression, MVP, tirage, après-match, validation) réessaient **une fois** sur
  `RepositoryError::ConcurrentWrite` : ils relisent l'agrégat, rejouent la
  commande — le domaine la revalide sur l'état relu —, puis appendent.
- Tout ce qui se calcule avant l'append (valeurs d'équipe, budget, trésorerie)
  est recalculé dans la boucle, jamais réutilisé.
- Un second conflit garde la réponse de E21 : 409 et rechargement.

## Fichiers touchés

`use_cases/record_fan_factor_use_case.rs`, `record_inducements_use_case.rs`,
`record_action_use_case.rs`, `delete_action_use_case.rs`,
`record_mvp_use_case.rs`, `draw_mvp_use_case.rs`,
`record_post_match_use_case.rs` (et ceux des cartes 654, 658, 659 quand ils
naissent) ; un helper partagé dans `use_cases/` ; `io/web/write_conflict.rs`
inchangé.

## Règles et invariants

- Un seul réessai : deux conflits de suite disent que l'état change sous la
  main, l'utilisateur doit le voir.
- Le réessai ne contourne aucune règle : la commande repasse par le domaine.
- Les commandes à deux camps (démarrage, annulation des coups de pouce,
  publication, dépublication) ne réessaient pas : un conflit y est un vrai
  conflit.
- Le use case reste instrumenté ; le réessai se journalise sous `kreek::`, au
  niveau `info`.

## Tests

- **Unitaires** : un dépôt qui refuse la première écriture puis accepte —
  la commande passe ; qui refuse deux fois — `Conflict` ; une commande que
  l'état relu rend invalide — le refus du domaine, pas un 409.
- **E2E** : deux coachs postent une action dans leur camp au même instant
  (deux sessions, requêtes lancées ensemble) ; les deux actions sont au journal.

## Conséquences sur les données de production

Aucune donnée modifiée. En mode A, un conflit sur une commande de camp est
désormais réessayé au lieu de rendre 409 : le résultat est celui qu'aurait
produit un second clic.

## Terminé quand

Deux actions saisies au même instant par les deux coachs d'un match en mode B
figurent toutes deux au journal, sans rechargement de page.
