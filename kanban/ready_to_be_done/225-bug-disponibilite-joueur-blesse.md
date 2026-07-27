# Bug — Un joueur blessé pendant un match est immédiatement rendu disponible

**Priorité : haute**
**Fichiers :** `src/app/players/io/app_events/team_match_concluded_listener.rs`, `src/app/players/io/app_events/player_match_impact_listener.rs`, `src/app/match_report/io/app_events/app_event_publisher.rs`
**Contexte :** `tests/e2e/` (nouveau fichier)
**Bloque :** règle 15 de `docs/specs/match-report-correction/`

## Problème suspecté

Comportement **déduit par lecture, non reproduit** — la carte commence donc par la
reproduction.

`handle_team_match_concluded` charge l'effectif via `find_by_team_id`, puis
restaure en `Available` tout joueur trouvé en `MissingNextGame` :

```rust
if player.participation_status == PlayerParticipationStatus::MissingNextGame {
    let restored = player.restore_availability(context.match_report_id.clone());
```

Or à cet instant, les blessures **du match courant** sont déjà appliquées :

1. `app_event_publisher` émet les events d'action — dont `PlayerInjured` —
   **avant** les deux `TeamMatchConcluded`
2. `player_match_impact_listener` les traite dans **une même tâche séquentielle**
   (choix délibéré, documenté : éviter la contention sur la version optimiste du
   même agrégat joueur)
3. `find_by_team_id` s'exécute donc après l'append des blessures

Un joueur blessé au match N serait ainsi remis `Available` à la fin du traitement
du match N — l'effet « absent au prochain match » annulé aussitôt qu'appliqué.

Le commentaire du listener dit « lève `MissingNextGame` → `Available` (BR12) pour
ceux qui **l'étaient** », ce qui décrit l'intention inverse : restaurer ceux qui
l'étaient **avant** ce match.

## Portée si confirmé

`Amoche`, `BlessureSerieuse` et `Sequel` ne feraient jamais manquer le match
suivant. Seule `Mort` survivrait, le test ne portant que sur `MissingNextGame`.

Les compteurs de carrière et les malus de séquelle ne sont pas affectés — c'est
uniquement la **disponibilité** qui l'est.

## Scénario E2E de reproduction

1. Deux équipes A et B, rapport de match, un joueur X de A subit une
   `BlessureSerieuse`
2. Publier le rapport
3. **Vérifier** que X est marqué absent au prochain match
4. Créer, saisir et publier le match suivant de A
5. **Vérifier** que X redevient disponible après *ce* match, et pas avant

L'étape 3 est celle qui échoue si le bug est confirmé.

## Correction si confirmé

Distinguer « était `MissingNextGame` **avant** ce match » de « l'est devenu **à
cause de** ce match ». Deux pistes :

- capturer le statut de participation avant l'application des events d'impact du
  match
- ne restaurer que les joueurs dont la blessure provient d'un match
  **antérieur** — `injuries` porte déjà `context.match_report_id`, l'information
  est disponible sans rien stocker

## Checklist

- [ ] Test E2E écrit, reproduisant le scénario ci-dessus
- [ ] Le test confirme ou infirme le bug — **résultat consigné dans la carte**
- [ ] Si confirmé : correction, et le test E2E passe au vert
- [ ] Si infirmé : le test est conservé comme test de non-régression, la carte
      est fermée sans changement de code
- [ ] Test unitaire couvrant la distinction « blessé avant » / « blessé pendant »
- [ ] `make test` et `make e2e` passent
- [ ] `make check-arch` passe
