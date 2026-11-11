# Deux coachs écrivent en même temps

**Épic :** E22 — Le mode « Chacun son équipe » · **Ordre :** 2 · **Dépend de :** 652
**Conception :** `docs/specs/saisie-mobile/E22-cadrage.md` (§ 1 « L'écriture concurrente », § 9, § 12.2)

## Le constat

Un flux de compte rendu n'a qu'une version : la contrainte
`match_report_es_version` refuse toute seconde écriture simultanée
(`src/app/match_report/io/repository/match_report_repository.rs:394-401`, `523-530`),
rendue en 409 avec `HX-Refresh` (`io/web/write_conflict.rs`). En « Je saisis
tout », un seul saisisseur : c'est rare. En mode B, deux coachs écrivent en même
temps dans deux camps indépendants, et le rechargement effacerait ce que le
second était en train de saisir — un paragraphe de résumé, par exemple.

## Ce qui change

- Les use cases d'écriture **d'un camp que le domaine protège** (coups de
  pouce et leurs joueurs temporaires, action, suppression, MVP, tirage)
  réessaient **une fois** sur `RepositoryError::ConcurrentWrite` : ils relisent
  l'agrégat, rejouent la commande — le domaine la revalide sur l'état relu —,
  puis appendent.
- **Liste blanche, décision du 2026-11-11** (cadrage § 12.2) : l'event store ne
  dit pas qui a gagné la course, on ne distingue donc pas un conflit entre camps
  d'un conflit sur le même camp. C'est le domaine qui fait la garde : une action
  s'ajoute, un MVP ou des coups de pouce déjà saisis par le même camp sont
  refusés (`MvpAlreadyRecorded`, `AlreadyRecorded`), jamais écrasés.
- **Le fan factor et l'après-match sont sortis de la carte** : ils écrivent
  encore les deux camps d'un coup et leur domaine écrase sans refuser — un
  réessai y serait un « dernier qui écrit gagne » silencieux. Ils entrent dans
  le réessai aux cartes 654 et 658, une fois devenus des commandes d'un seul
  camp.
- Tout ce qui se calcule avant l'append (valeurs d'équipe, budget, trésorerie)
  est recalculé dans la boucle, jamais réutilisé.
- Un second conflit garde la réponse de E21 : 409 et rechargement.

## Fichiers touchés

`use_cases/record_inducements_use_case.rs`, `init_temp_players_use_case.rs`,
`record_action_use_case.rs`, `delete_action_use_case.rs`,
`record_mvp_use_case.rs`, `draw_mvp_use_case.rs` (et ceux des cartes 654, 658,
659 quand ils naissent) ; le helper partagé `use_cases/conflict_retry.rs`
(`ConflictAware`, `lost_the_race`, `settle`) ; la doublure
`ScriptedRepository` dans `use_cases/test_doubles.rs` ;
`io/web/write_conflict.rs` inchangé.

**Deux tours écrits à la main, et non une fermeture.** La première forme
prenait l'essai en `AsyncFnMut` ; le compilateur refuse alors le futur du
contrôleur axum (« implementation of `Send` is not general enough » sur les
`&dyn Port` empruntés). Chaque use case écrit donc `let first = …once(…).await;
if !lost_the_race(…) { return first; } settle(…, …once(…).await)`, et le
module ne garde que la décision et le journal.

## Règles et invariants

- Un seul réessai : deux conflits de suite disent que l'état change sous la
  main, l'utilisateur doit le voir.
- Le réessai ne contourne aucune règle : la commande repasse par le domaine.
- Les commandes à deux camps (démarrage, annulation des coups de pouce,
  publication, dépublication) ne réessaient pas : un conflit y est un vrai
  conflit.
- Le use case reste instrumenté ; le réessai se journalise sous `kreek::`, au
  niveau `info` (« conflit de version, relecture et nouvelle tentative »), et
  l'abandon aussi (« second conflit de version, rendu au client ») : en mode B,
  une course entre camps est le fonctionnement normal.
- `record_inducements` réessaie sa sélection seule ; `init_temp_players`, qui
  suit, a son propre réessai : la sélection est déjà écrite quand il échoue.

## Tests

- **Unitaires** : un dépôt qui refuse la première écriture puis accepte —
  la commande passe ; qui refuse deux fois — `Conflict` ; une commande que
  l'état relu rend invalide — le refus du domaine, pas un 409.
- **Intégration, sur la vraie base** (`io/repository/test_concurrent_side_writes.rs`) :
  `InterleavingRepository` écrit l'action de l'autre camp juste avant l'append
  du use case, à la même version — la contrainte Postgres lève le vrai
  `ConcurrentWrite` à chaque exécution, sans attente ni chance.
- **E2E** (`test_match_report_concurrent_writes.py`) : deux coachs postent une
  action dans leur camp au même instant (deux sessions, barrière, dix paires) ;
  toutes les actions sont au journal. La course y est probable, pas garantie :
  la preuve déterministe du réessai est le test d'intégration.

## Conséquences sur les données de production

Aucune donnée modifiée, aucune migration, aucun événement nouveau. En mode A
aussi, un conflit sur une commande de la liste blanche est désormais réessayé
au lieu de rendre 409 : le résultat est celui qu'aurait produit un second clic.
Le fan factor et l'après-match gardent leur 409.

## Terminé quand

Deux actions saisies au même instant par les deux coachs d'un match en mode B
figurent toutes deux au journal, sans rechargement de page.
