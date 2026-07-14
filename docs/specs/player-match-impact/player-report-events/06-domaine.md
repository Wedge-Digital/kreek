# Player report events — Phase 6 : Domaine ✅

## Récapitulatif des règles métier (validé)

| # | Règle | Responsable |
|---|---|---|
| BR1 | Seules les actions `Regular` (player_id stable) produisent des `PlayerReportEvents` — stars/mercenaires/journaliers exclus | **Publisher `match_report`** (filtrage avant émission, hors domaine `players`) |
| BR2 | `Passe` et `Lancer` sont la même notion domaine (`PassCompleted`) | **Publisher `match_report`** (mapping avant émission) |
| BR3 | `round_label`/noms d'équipe résolus par `match_report` au publish, embarqués dans l'event | **Publisher `match_report`**, hors domaine `players` |
| BR4 | Le SPP par type d'action est résolu par `players` via `references` (port + domain service), jamais calculé par `match_report` ni codé en dur dans `players` | **Domain service `players/use_cases/`** |
| BR5 | Essai/Passe/Interception/Sortie/MVP créditent du SPP et incrémentent le compteur de carrière correspondant | **Domaine** (`Player::apply`) |
| BR6 | Agression (`FoulCommitted`) incrémente `career_fouls`, ne crédite aucun SPP | **Domaine** |
| BR7 | Toute blessure (y compris Commotion) est journalisée dans `injuries` | **Domaine** |
| BR8 | Commotion : aucun effet sur le statut ni les compteurs, au-delà de la journalisation | **Domaine** |
| BR9 | Mort : statut → `Dead` (terminal) | **Domaine** |
| BR10 | Blessure sérieuse : statut → `MissingNextGame`, incrémente `career_persistent_injuries` | **Domaine** |
| BR11 | Séquelle (`Sequel{stat}`) : statut → `MissingNextGame`, ajoute un `StatAdjustment` — n'incrémente **pas** `career_persistent_injuries` | **Domaine** |
| BR11b | Amoché (`Amoche`) : statut → `MissingNextGame` — n'incrémente **pas** `career_persistent_injuries`, n'ajoute **pas** de `StatAdjustment` | **Domaine** |
| BR12 | `MissingNextGame` ne dure qu'un seul match — levé à la réception de `TeamMatchConcluded` pour l'équipe du joueur | **Domaine**, déclenché par un listener `players` |
| BR13 | La résolution des stats finales (base + `stat_adjustments`) se fait hors de l'agrégat, dans un domain service appelant `references` — `Player::apply()` reste pur, ne stocke que le delta | **Domain service `players/use_cases/`**, pas le domaine |
| BR13b | Le malus d'une séquelle diminue MA/ST/AV (plus haut = meilleur, AV 2020 inclus) mais augmente AG/PA (nombres cibles de dé à atteindre, plus bas = meilleur) — corrigé après implémentation, cohérent avec `SequelStat::MinusAv` côté `match_report` | **Domain service `players/use_cases/`** |
| BR14 | Aucune garde métier sur ces méthodes d'enregistrement — ce sont des faits déjà survenus et validés par `match_report`, `players` les applique sans les remettre en question | **Domaine** (méthodes infaillibles, pas de `Result`) |
| BR15 | `Retired` existe dans `PlayerParticipationStatus` pour complétude du type mais n'est produit par aucun event de cette feature | **Domaine** (aucune méthode ne le produit ici) |

---

## Nouveau fichier — `src/app/players/domain/match_impact.rs`

Regroupe tous les nouveaux types liés à l'impact des rapports de match, distinct de `value_objects.rs` (VOs de compétences/position déjà existants) et de `player.rs` (agrégat + identifiants).

```rust
use crate::app::players::domain::player::TeamId;
use serde::{Deserialize, Serialize};

// ── Contexte de match (embarqué dans chaque event, zéro appel inter-BC en lecture) ──

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchReportId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchContext {
    pub match_report_id:    MatchReportId,
    pub round_id:           RoundId,
    pub round_label:        String,           // texte libre, cf. CLAUDE.md exception
    pub opponent_team_id:   TeamId,
    pub opponent_team_name: String,           // texte libre
}

// ── SPP gagné par une action (résolu en amont via references, jamais calculé ici) ──

#[nutype(validate(greater_or_equal = 1), derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize))]
pub struct SppEarned(u32);

// ── Statut de participation ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerParticipationStatus {
    Available,
    MissingNextGame,
    Retired,   // jamais produit par cette feature (BR15)
    Dead,
}

// ── Blessures ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatKind { Ma, St, Ag, Pa, Av }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InjuryType {
    Commotion,
    Amoche,
    BlessureSerieuse,
    Sequel { stat: StatKind },
    Mort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInjuryRecord {
    pub injury_type: InjuryType,
    pub context:     MatchContext,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StatAdjustment {
    pub stat:  StatKind,
    pub malus: u8,   // toujours 1 (règle BB standard pour une séquelle) — hypothèse, à confirmer
}

// ── Compteurs de carrière (même style que Spp/ValueKpo dans player.rs) ─────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TouchdownCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PassCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InterceptionCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CasualtyCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MvpCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FoulCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PersistentInjuryCount(pub u16);
```

Sens du malus (BR13b) : MA/ST/AV diminuent, AG/PA augmentent — implémenté dans le domain service de résolution (voir plus bas). Corrigé pendant l'implémentation (carte 156) : AV suit MA/ST (2020 : plus haut = plus dur à blesser), pas AG/PA — confirmé par le nommage `SequelStat::MinusAv` déjà présent côté `match_report`.

---

## Agrégat `Player` — nouveaux champs

Fichier : `src/app/players/domain/player.rs`

```rust
pub struct Player {
    // ... champs existants inchangés ...
    pub participation_status:       PlayerParticipationStatus,
    pub career_touchdowns:          TouchdownCount,
    pub career_passes:              PassCount,
    pub career_interceptions:       InterceptionCount,
    pub career_casualties:          CasualtyCount,
    pub career_mvps:                MvpCount,
    pub career_fouls:               FoulCount,
    pub career_persistent_injuries: PersistentInjuryCount,
    pub injuries:                   Vec<PlayerInjuryRecord>,
    pub stat_adjustments:           Vec<StatAdjustment>,
}
```

`PlayerCreated` (hydratation existante) initialise tous ces champs à leur valeur par défaut (`Available`, compteurs à 0, listes vides) — un seul point d'ajout dans la branche `apply()` existante.

---

## Nouveaux domain events — `PlayerDomainEvent`

Fichier : `src/app/players/domain/events.rs`

```rust
TouchdownScored            { context: MatchContext, spp_earned: SppEarned },
PassCompleted               { context: MatchContext, spp_earned: SppEarned },
InterceptionMade            { context: MatchContext, spp_earned: SppEarned },
CasualtyInflicted           { context: MatchContext, spp_earned: SppEarned },
MatchMvpNamed                { context: MatchContext, spp_earned: SppEarned },
FoulCommitted                { context: MatchContext },
InjurySustained              { context: MatchContext, injury_type: InjuryType },
PlayerAvailabilityRestored   { match_report_id: MatchReportId },
```

Nommage en termes domaine (pas `PlayerPerformedTouchdown` qui trahirait l'origine app event, conforme à la règle CLAUDE.md sur le nommage des domain events).

---

## Méthodes domaine — `Player`

Toutes infaillibles (pas de `Result`, cf. BR14) — miroir du pattern `publish()` de `match_report` (aucune garde à vérifier).

```rust
impl Player {
    pub fn record_touchdown(&self, context: MatchContext, spp_earned: SppEarned) -> PlayerDomainEvent {
        PlayerDomainEvent::TouchdownScored { context, spp_earned }
    }
    pub fn record_pass(&self, context: MatchContext, spp_earned: SppEarned) -> PlayerDomainEvent {
        PlayerDomainEvent::PassCompleted { context, spp_earned }
    }
    pub fn record_interception(&self, context: MatchContext, spp_earned: SppEarned) -> PlayerDomainEvent {
        PlayerDomainEvent::InterceptionMade { context, spp_earned }
    }
    pub fn record_casualty(&self, context: MatchContext, spp_earned: SppEarned) -> PlayerDomainEvent {
        PlayerDomainEvent::CasualtyInflicted { context, spp_earned }
    }
    pub fn record_mvp(&self, context: MatchContext, spp_earned: SppEarned) -> PlayerDomainEvent {
        PlayerDomainEvent::MatchMvpNamed { context, spp_earned }
    }
    pub fn record_foul(&self, context: MatchContext) -> PlayerDomainEvent {
        PlayerDomainEvent::FoulCommitted { context }
    }
    pub fn record_injury(&self, context: MatchContext, injury_type: InjuryType) -> PlayerDomainEvent {
        PlayerDomainEvent::InjurySustained { context, injury_type }
    }
    pub fn restore_availability(&self, match_report_id: MatchReportId) -> PlayerDomainEvent {
        PlayerDomainEvent::PlayerAvailabilityRestored { match_report_id }
    }
}
```

Ces méthodes ne font que construire l'event — **toute la logique métier (quel compteur incrémenter, quel statut appliquer) vit exclusivement dans `apply()`**, jamais dans les méthodes de commande, conformément à la séparation event-sourcing déjà en place dans le projet.

### `apply()` — nouvelles branches

Fichier : `src/app/players/domain/player.rs`

```rust
PlayerDomainEvent::TouchdownScored { spp_earned, .. } => {
    let mut player = current?;
    player.spp = Spp(player.spp.0 + spp_earned.into_inner());
    player.career_touchdowns.0 += 1;
    Some(player)
}
// PassCompleted / InterceptionMade / CasualtyInflicted / MatchMvpNamed : même forme,
// chacun incrémente son propre compteur de carrière.

PlayerDomainEvent::FoulCommitted { .. } => {
    let mut player = current?;
    player.career_fouls.0 += 1;
    Some(player)
}

PlayerDomainEvent::InjurySustained { context, injury_type } => {
    let mut player = current?;
    player.injuries.push(PlayerInjuryRecord {
        injury_type: injury_type.clone(),
        context: context.clone(),
    });
    match injury_type {
        InjuryType::Commotion => {}
        InjuryType::Mort => player.participation_status = PlayerParticipationStatus::Dead,
        InjuryType::BlessureSerieuse => {
            player.participation_status = PlayerParticipationStatus::MissingNextGame;
            player.career_persistent_injuries.0 += 1;
        }
        InjuryType::Sequel { stat } => {
            player.participation_status = PlayerParticipationStatus::MissingNextGame;
            player.stat_adjustments.push(StatAdjustment { stat: *stat, malus: 1 });
        }
        InjuryType::Amoche => {
            player.participation_status = PlayerParticipationStatus::MissingNextGame;
        }
    }
    Some(player)
}

PlayerDomainEvent::PlayerAvailabilityRestored { .. } => {
    let mut player = current?;
    if player.participation_status == PlayerParticipationStatus::MissingNextGame {
        player.participation_status = PlayerParticipationStatus::Available;
    }
    Some(player)
}
```

Confirmé (BR11b) : `Amoche` → `MissingNextGame` uniquement, sans incrémenter `career_persistent_injuries` ni ajouter de `StatAdjustment` — traitement distinct de `BlessureSerieuse` et `Sequel`, malgré une branche `apply()` de forme similaire.

---

## Domain service — résolution des stats (couche `use_cases/`, documenté ici pour cohérence — BR13)

**Ce n'est pas du domaine** (le domaine `Player` reste pur, ne stocke que `stat_adjustments`) — mais comme la règle métier BR13 a été validée dans cette même discussion, je le documente ici plutôt que d'ouvrir un fichier Phase 5 séparé pour si peu.

Fichier : `src/app/players/use_cases/player_stats_service.rs` (nouveau)

```rust
pub struct ResolvedPlayerStats { pub ma: u8, pub st: u8, pub ag: u8, pub pa: u8, pub av: u8 }

pub fn resolve_stats(player: &Player, ref_repo: &dyn IReferenceRepository) -> Option<ResolvedPlayerStats> {
    let base = ref_repo.find_position_by_uid(player.roster_line_id.as_ref())?;
    let mut stats = ResolvedPlayerStats { ma: base.ma, st: base.st, ag: base.ag, pa: base.pa, av: base.av };
    for adj in &player.stat_adjustments {
        match adj.stat {
            // MA/ST/AV : plus haut = meilleur → le malus DIMINUE la valeur
            StatKind::Ma => stats.ma = stats.ma.saturating_sub(adj.malus),
            StatKind::St => stats.st = stats.st.saturating_sub(adj.malus),
            StatKind::Av => stats.av = stats.av.saturating_sub(adj.malus),
            // AG/PA : nombres cibles de dé, plus bas = meilleur → le malus AUGMENTE la valeur
            StatKind::Ag => stats.ag = stats.ag.saturating_add(adj.malus),
            StatKind::Pa => stats.pa = stats.pa.saturating_add(adj.malus),
        }
    }
    Some(stats)
}
```

**Correction post-recherche (voir 07-integration.md)** : pas besoin de nouveau port. `players` a déjà un précédent (`team_created_listener.rs`) qui consomme directement `references::domain::port::IReferenceRepository` (déjà injecté dans `players::context::init_listeners` sous le nom `refs`) — `find_position_by_uid(roster_line_id)` existe déjà et retourne déjà MA/ST/AG/PA/AV (`PlayerPosition`). On réutilise ce même accès direct plutôt que d'introduire une deuxième convention d'accès à `references` dans le même BC.

---

## Mécanique `TeamMatchConcluded` → levée de `MissingNextGame` (BR12)

- `match_report` émet, en plus des `PlayerReportEvents` par action, **un event par équipe** à la publication : `TeamMatchConcluded { team_id, match_report_id }` (app event, `shared_kernel`).
- Nouveau listener `players/io/app_events/team_match_concluded_listener.rs` : reçoit l'event, appelle `IPlayerRepository::find_by_team_id(team_id)` (méthode **déjà existante**, aucun nouveau repository à créer), et pour chaque joueur dont `participation_status == MissingNextGame`, appelle `player.restore_availability(match_report_id)` et persiste. No-op pour les autres.

---

## Erreurs domaine

Aucun nouveau variant `DomainError` (BR14) — toutes les méthodes ci-dessus sont infaillibles. `DomainError` reste l'enum vide actuel (`src/app/players/domain/error.rs`).

---

## Tests unitaires prévus

Dans `#[cfg(test)]` de `player.rs` (module à créer, le fichier n'en a pas encore) :

```rust
#[test]
fn touchdown_credits_spp_and_increments_counter() { /* spp += spp_earned, career_touchdowns == 1 */ }

#[test]
fn foul_increments_counter_without_spp() { /* career_fouls == 1, spp inchangé */ }

#[test]
fn commotion_is_logged_without_status_or_counter_change() {
    /* injuries.len() == 1, participation_status == Available, tous compteurs à 0 */
}

#[test]
fn death_sets_dead_status() { /* participation_status == Dead */ }

#[test]
fn serious_injury_sets_missing_next_game_and_increments_persistent_counter() {
    /* participation_status == MissingNextGame, career_persistent_injuries == 1, stat_adjustments vide */
}

#[test]
fn sequel_sets_missing_next_game_and_adds_stat_adjustment_without_persistent_counter() {
    /* participation_status == MissingNextGame, career_persistent_injuries == 0, stat_adjustments.len() == 1 */
}

#[test]
fn availability_restored_only_changes_missing_next_game_players() {
    /* joueur Available + PlayerAvailabilityRestored → reste Available (no-op) */
    /* joueur MissingNextGame + PlayerAvailabilityRestored → devient Available */
}

#[test]
fn availability_restored_does_not_affect_dead_or_retired_players() {
    /* joueur Dead + PlayerAvailabilityRestored → reste Dead */
}
```

Test du domain service (`player_stats_service.rs`) :

```rust
#[test]
fn resolve_stats_applies_ma_st_malus_as_decrease_and_ag_pa_av_malus_as_increase() {
    /* base MA=7 + adjustment{Ma, malus:1} → resolved.ma == 6 */
    /* base AG=3 + adjustment{Ag, malus:1} → resolved.ag == 4 */
}
```

---

## Résumé des fichiers créés/modifiés

| Fichier | Nature |
|---|---|
| `src/app/players/domain/match_impact.rs` | **Nouveau** — tous les types listés plus haut |
| `src/app/players/domain/player.rs` | Ajout champs `Player` + branches `apply()` + méthodes de commande + tests |
| `src/app/players/domain/events.rs` | Ajout des 8 nouveaux variants `PlayerDomainEvent` |
| `src/app/players/use_cases/player_stats_service.rs` | **Nouveau** — `resolve_stats()` (domain service) |
| `src/app/players/io/app_events/team_match_concluded_listener.rs` | **Nouveau** (Phase 7/8) |
