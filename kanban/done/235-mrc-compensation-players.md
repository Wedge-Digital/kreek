# `players` — Compensation d'une dépublication

**Priorité : haute**
**Dépend de :** `231-mrc-publisher-app-events.md`, **`225-bug-disponibilite-joueur-blesse.md`**
**Fichiers :** `src/app/players/domain/{player,events}.rs`, `src/app/players/io/repository/player_repository.rs`, `src/app/players/io/app_events/player_match_impact_listener.rs`
**Spec :** `docs/specs/match-report-correction/recap/06-domaine.md`, `07-integration.md`

## Objectif

Retirer l'impact d'un match sur l'ensemble des joueurs des 2 équipes : SPP,
compteurs de carrière, blessures, séquelles, statut de participation.

## Dépendance bloquante — carte 225

La carte 225 fixe la définition de « statut de participation **avant** le
match ». Tant qu'elle n'est pas tranchée, `participation_status_before` n'a pas
de sémantique stable et la règle 15 ne peut pas être implémentée correctement.
**Ne pas démarrer cette carte avant que la 225 soit close.**

## Conception

### Ce qui ne demande aucun instantané

`injuries: Vec<PlayerInjuryRecord>` porte déjà `context.match_report_id`. Sont
donc dérivables **par filtrage** :

| À défaire | Dérivation |
|---|---|
| entrées de `injuries` | filtrer sur `context.match_report_id` |
| `stat_adjustments` ajoutés | un par blessure `Sequel` de ce match |
| `career_persistent_injuries` | une par `BlessureSerieuse` de ce match |

### Ce qui demande un instantané

Les compteurs d'action et les SPP sont des scalaires cumulés, non tagués :

```rust
struct LastMatchContribution {
    match_report_id:             MatchReportId,
    spp_earned:                  Spp,
    touchdowns:                  TouchdownCount,
    passes:                      PassCount,
    interceptions:               InterceptionCount,
    casualties:                  CasualtyCount,
    mvps:                        MvpCount,
    fouls:                       FoulCount,
    matches_played:              MatchesPlayedCount,
    participation_status_before: PlayerParticipationStatus,
    availability_restored:       bool,
}
```

Accumulé dans `apply()` sur les événements dont `context.match_report_id`
correspond ; **réinitialisé dès qu'un nouveau `match_report_id` apparaît**.
Comme seul le dernier match est corrigible, un seul accumulateur suffit.

Rebuildable depuis les événements existants : aucune migration.

### Méthode domaine

```rust
pub fn revert_match_impact(&self, match_report_id: &MatchReportId)
    -> Option<PlayerDomainEvent>
```

Retourne `None` si `last_match` est absent ou porte un autre `match_report_id`.
C'est à la fois l'idempotence (règle 11) et ce qui permet au listener d'itérer
sur tout l'effectif sans se soucier de qui a joué.

Aucun risque de SPP négatif : le garde-fou garantit qu'aucun SPP n'a été dépensé.

Les joueurs temporaires (star, mercenaire, journalier) sont hors sujet — ils
n'ont jamais reçu d'impact (BR1 existante).

### Listener — dans le listener d'impact existant

`TeamMatchImpactReverted` est traité **dans
`player_match_impact_listener`**, pas dans un listener dédié. Raison documentée
dans `team_match_concluded_listener.rs` : deux tâches concurrentes se
disputeraient la version optimiste du même agrégat joueur, et l'une des deux
perdrait la course en silence.

### Projection

`players_proj` porte `spp` et `value_kpo`. `upsert_player_projection` doit
traiter le nouvel événement, **dans la transaction de l'append** (règle ES).

Piège : un événement non traité compile sans broncher et laisse la projection
figée sur les valeurs post-match, avec un agrégat pourtant juste. D'où un test
d'intégration repository.

## Checklist

- [ ] Carte 225 close avant de démarrer
- [ ] `LastMatchContribution` et son accumulation dans `apply()`
- [ ] Réinitialisation à l'apparition d'un nouveau `match_report_id`
- [ ] Événement de compensation + `type_name()`
- [ ] `revert_match_impact()` retournant `None` hors périmètre
- [ ] Blessures, séquelles et compteur de blessures persistantes défaits par filtrage
- [ ] Statut de participation restauré selon la définition fixée par la 225
- [ ] Branche `TeamMatchImpactReverted` dans `player_match_impact_listener`
- [ ] `upsert_player_projection` traite le nouvel événement, même transaction
- [ ] Test : SPP du match retirés
- [ ] Test : compteurs de carrière du match retirés
- [ ] Test : seules les blessures **de ce match** sont retirées
- [ ] Test : malus de séquelle retiré
- [ ] Test : statut de participation restauré
- [ ] Test : un autre `match_report_id` → `None`
- [ ] Test : une seconde compensation ne produit rien
- [ ] Test d'intégration : la projection reflète l'état compensé
- [ ] `make test` passe
- [ ] `make check-arch` passe
