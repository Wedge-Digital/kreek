# Architecture — Nettoyage des faux positifs `check-arch` (axe 3 / axe 5)

**Priorité : haute**
**Dépend de :** rien
**Contexte :** `scripts/check-arch.sh` + fichiers de test cross-BC

## Objectif

`check-arch.sh` scanne chaque fichier `.rs` et considère qu'il est du code de test uniquement s'il trouve littéralement `#[cfg(test)]` **dans le fichier lui-même**. Plusieurs fichiers de test vivent dans un sous-dossier `tests/` et sont gatés via `#[cfg(test)] pub mod tests;` dans le `mod.rs` parent — le marqueur n'apparaît donc jamais dans le fichier scanné, et le script les traite à tort comme du code de production (violations axe 3 fantômes).

Un autre faux positif : `match_day_repository.rs::list_from_projection` est une **lecture** (SELECT), pas une mise à jour de projection — le regex axe 5 (`_projection`) matche son nom sans distinguer lecture/écriture.

Enfin, `match_report_published_listener.rs::update_projection` met à jour une projection **en réaction à un app event cross-BC** (`MatchReportPublished`, émis par `match_report`, déjà committé ailleurs au moment où ce listener s'exécute). Il est impossible de partager une transaction avec un commit distant d'un autre BC — ce pattern (listener async → projection locale) est le mécanisme documenté dans le CLAUDE.md pour les app events cross-BC. La règle axe 5 ("même transaction que l'append de l'événement qui la déclenche") vise les projections **intra-BC** : un agrégat et sa projection appartenant au même BC, appendés dans le même flux. Décision validée avec l'utilisateur (2026-07-22) : faire évoluer le script pour ne détecter que ce cas.

## Action

### 1. Gater les fichiers de test avec `#![cfg(test)]`

Ajouter `#![cfg(test)]` en première ligne de :
- `src/app/players/io/repository/tests/test_player_repository.rs`
- `src/app/players/io/app_events/tests/test_player_match_impact_pipeline.rs`
- `src/app/teams/io/app_events/tests/test_player_improvement_pipeline.rs`

### 2. Restreindre `check-arch.sh` (axe 5) aux projections intra-BC

Le script doit distinguer :
- une fonction de projection appelée depuis un listener qui souscrit au **bus interne du BC** (`event_bus`, domain event) → doit rester détectée si elle prend un `PgPool`
- une fonction de projection appelée depuis un listener qui souscrit à l'**app event bus** (`app_event_bus`, cross-BC) → à exclure de la détection

Approche : dans le script, exclure les fichiers situés sous `io/app_events/` dont l'`init()` s'abonne à `app_event_bus` plutôt qu'à `event_bus` interne — ou plus simplement, exclure spécifiquement les fonctions de projection appelées depuis un `handle_event` qui désérialise un type suffixé `AppEvent` (par opposition à `DomainEvent`). Choisir l'implémentation la plus simple qui reste correcte (heuristique regex, cohérente avec le reste du script).

Documenter cette exception dans le CLAUDE.md, section "Projections event sourcing" : ajouter un paragraphe précisant que la règle de transaction unique s'applique aux projections intra-BC ; les projections mises à jour en réaction à un app event cross-BC sont par nature asynchrones et rebuildables depuis l'event store du BC source en cas de désynchronisation.

### 3. `list_from_projection`

Aucun changement de code — faux positif documenté (nom de fonction contenant `_projection` mais lecture pure). Si une reformulation du nom est triviale et n'impacte rien d'autre, on peut envisager un renommage (`list_pairings_for_display` ou équivalent) pour lever toute ambiguïté future, à la discrétion de l'implémentation — sinon laisser tel quel.

## Checklist

- [ ] `#![cfg(test)]` ajouté aux 3 fichiers de test
- [ ] `check-arch.sh` axe 5 ne détecte plus `match_report_published_listener.rs::update_projection`
- [ ] `check-arch.sh` détecte toujours `pairing_projection_listener.rs::insert_projection`/`delete_projection` (intra-BC, vraie violation — carte 186)
- [ ] CLAUDE.md mis à jour (paragraphe exception cross-BC)
- [ ] `make check-arch` : axe 3 et axe 5 ne remontent plus ces faux positifs
- [ ] `cargo test` passe toujours (les 3 fichiers gatés compilent et s'exécutent en mode test)
