> **Carte annulée le 2026-08-29 — remplacée par les cartes 474 et 475.**
>
> Son constat était **à moitié périmé**. « Les onglets Équipes *et* Stats
> servent de la donnée fictive » : c'est faux pour Équipes, branché depuis les
> cartes du BC `teams` — `TeamsTabTemplate` ne porte plus que `app_routes`,
> `space_id` et `season_id`. `mock_teams()` existe encore dans le fichier mais
> **n'est appelé nulle part** : c'est du code mort qui gonflait le constat, et
> la 474 le supprime.
>
> Son action 2 disait « vérifier si la donnée est modélisée ; sinon retourner
> des collections vides et créer un ticket de modélisation ». **Vérifié : elle
> l'est**, et dans la table du BC lui-même — `competition_match_display_proj`
> porte les scores et les blessures par match depuis l'origine. Il n'y a donc
> ni modélisation à faire, ni collections vides à servir en attendant.
>
> La conception vit dans `docs/specs/statistiques-de-competition/`.

# 100% mock data dans `competition_detail.rs`

**Priorité : moyenne**
**Fichier :** `src/app/competitions/io/web/competition_detail.rs`

## Problème

Les onglets Équipes et Stats du détail de compétition servent encore de la donnée entièrement hardcodée :

```rust
fn mock_teams()     -> Vec<TeamCard>    { vec![...] }
fn mock_top_tds()   -> Vec<StatRow>     { vec![...] }
fn mock_top_casualties() -> Vec<StatRow> { vec![...] }
fn mock_flop_tds()   -> Vec<StatRow>    { vec![...] }
fn mock_flop_casualties() -> Vec<StatRow> { vec![...] }
```

Ces fonctions sont appelées directement dans les handlers de production. L'utilisateur voit la même donnée fictive quelle que soit la compétition consultée. Il n'y a aucun marqueur `TODO` ou feature-flag visible.

**Classement et Matchs (Calendrier/Résultats) sont réglés** : Classement est désormais un widget hébergé par le BC `ranking` (feature ranking, cartes 192-198) ; Calendrier/Résultats consomment déjà de vraies données (plus de `mock_journees`).

## Action

1. Pour Équipes : brancher le vrai repository (données déjà modélisées via `teams`)
2. Pour Stats (top/flop TDs, casualties) : vérifier si la donnée est modélisée ; sinon retourner des collections vides avec un état vide explicite dans l'UI, et créer un ticket de modélisation
3. Supprimer les fonctions `mock_*` restantes du fichier de production une fois branchées
