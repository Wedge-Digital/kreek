# BC `players` — Listeners `PlayerReportEvents` + `TeamMatchConcluded`

**Priorité : haute**
**Dépend de :** `152-references-spp-scale.md`, `153-shared-kernel-player-match-impact-events.md`, `154-players-domain-match-impact.md`, `155-players-persistence-match-impact.md`
**Contexte :** `players/io/app_events` — deux nouveaux listeners

## Objectif

Consommer les `PlayerMatchImpactAppEvent` (carte 157) pour mettre à jour les
agrégats `Player` concernés : stats de carrière, SPP, blessures, et levée de
`MissingNextGame` en fin de match. Spec :
`docs/specs/player-match-impact/player-report-events/07-integration.md` §6.

---

## Conception

### `src/app/players/io/app_events/player_match_impact_listener.rs` (nouveau)

- S'abonne à `app_event_bus`, désérialise en `PlayerMatchImpactAppEvent`, ignore `TeamMatchConcluded` (traité par l'autre listener ci-dessous) et tout ce qui échoue à désérialiser.
- Pour chaque event reçu : `player_repo.find_by_id(player_id)` (charge l'agrégat pour connaître sa version courante — introuvable → `tracing::warn!` + continue, même pattern que `match_report_confirmed_listener`).
- Pour les 5 variants SPP (`PlayerPerformedTouchdown`/`Pass`/`Interception`/`Casualty`/`Mvp`) : résout le montant via `ref_repo.touchdown_spp()`/`pass_spp()`/etc. (carte 152), construit `MatchContext` depuis le payload, appelle `player.record_touchdown(context, SppEarned::try_new(amount).unwrap())` (le barème garantit `>= 1`), persiste (`insert_player_event` + `upsert_player_projection` dans une tx, même pattern que `team_created_listener`).
- `PlayerPerformedFoul` → `player.record_foul(context)`, même persistance.
- `PlayerInjured` → mapping `InjuryTypePayload` → `InjuryType` domaine, `player.record_injury(context, injury_type)`, même persistance.

### `src/app/players/io/app_events/team_match_concluded_listener.rs` (nouveau)

- S'abonne à `app_event_bus`, filtre `PlayerMatchImpactAppEvent::TeamMatchConcluded`.
- `player_repo.find_by_team_id(&team_id)` (déjà existant, carte 155 n'y touche pas), filtre `participation_status == MissingNextGame`, pour chacun : `player.restore_availability(match_report_id)`, persiste.
- No-op silencieux pour les joueurs non `MissingNextGame`.

### Wiring — `players/context.rs`

```rust
pub fn init_listeners(app_event_bus: &EventBus, pool: PgPool, refs: Arc<dyn IReferenceRepository>) {
    team_created_listener::init(app_event_bus, pool.clone(), refs.clone());
    player_match_impact_listener::init(app_event_bus, pool.clone(), refs);       // nouveau
    team_match_concluded_listener::init(app_event_bus, pool);                     // nouveau
}
```

Aucun nouveau paramètre à `init_listeners` — `refs` déjà présent suffit.

---

## Checklist

- [ ] `player_match_impact_listener.rs` — 7 branches (5 SPP + foul + injury)
- [ ] `team_match_concluded_listener.rs` — restauration `MissingNextGame` uniquement
- [ ] Wiring dans `players/context.rs::init_listeners()`
- [ ] Tests d'intégration (vraie PgPool) : chaque type d'event met à jour l'agrégat comme attendu, `TeamMatchConcluded` ne touche que les joueurs `MissingNextGame` de l'équipe concernée
