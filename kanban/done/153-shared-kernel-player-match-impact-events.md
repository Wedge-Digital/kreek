# Shared kernel — App events `PlayerMatchImpactAppEvent`

**Priorité : haute**
**Dépend de :** rien
**Contexte :** `shared_kernel/app_events` — contrat inter-BC `match_report` → `players`

## Objectif

Définir la famille d'app events « PlayerReportEvents » émise par BC `match_report` à
la publication d'un rapport, consommée par BC `players` pour alimenter les stats de
carrière, le SPP et les blessures d'un joueur. Voir
`docs/specs/player-match-impact/player-report-events/07-integration.md` §4 pour le
contexte complet.

---

## Conception

### Nouveau fichier : `src/app/shared_kernel/app_events/player_match_impact_app_events.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerMatchContextPayload {
    pub match_report_id:    String,
    pub round_id:           String,
    pub round_label:        String,
    pub opponent_team_id:   String,
    pub opponent_team_name: String,
    pub player_id:          String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InjuryTypePayload {
    Commotion,
    Amoche,
    BlessureSerieuse,
    Sequel { stat: String },   // "Ma" | "St" | "Ag" | "Pa" | "Av"
    Mort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerMatchImpactAppEvent {
    PlayerPerformedTouchdown(PlayerMatchContextPayload),
    PlayerPerformedPass(PlayerMatchContextPayload),
    PlayerPerformedInterception(PlayerMatchContextPayload),
    PlayerPerformedCasualty(PlayerMatchContextPayload),
    PlayerPerformedMvp(PlayerMatchContextPayload),
    PlayerPerformedFoul(PlayerMatchContextPayload),
    PlayerInjured { context: PlayerMatchContextPayload, injury_type: InjuryTypePayload },
    TeamMatchConcluded { team_id: String, match_report_id: String },
}
```

Suivre le même pattern que `match_report_app_events.rs` existant (`event_type()`,
`to_enveloppe()`, constantes `&'static str` pour chaque type d'event).

### Point de vigilance

`InjuryTypePayload` est **structuré** (porte `stat` pour `Sequel`), volontairement
distinct de `injury_label(&InjuryType) -> String` déjà utilisé par
`MatchReportPublishedPayload` (qui perd l'info de stat — suffisant pour l'affichage
humain existant, insuffisant pour piloter `stat_adjustments` côté `players`, carte
154). Ne pas réutiliser/fusionner les deux.

`TeamMatchConcluded` est un variant de la **même** enum que les events par action —
un seul point d'écoute côté `players` à filtrer, cohérent avec le fait que les deux
proviennent du même publisher/moment (carte 157).

---

## Checklist

- [ ] `PlayerMatchContextPayload`, `InjuryTypePayload`, `PlayerMatchImpactAppEvent`
- [ ] `event_type()` + constantes de nom + `to_enveloppe()`, même pattern que `match_report_app_events.rs`
- [ ] Export dans `shared_kernel/app_events/mod.rs`
