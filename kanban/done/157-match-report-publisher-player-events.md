# BC `match_report` — Publisher étendu : émission des `PlayerReportEvents`

**Priorité : haute**
**Dépend de :** `153-shared-kernel-player-match-impact-events.md`
**Contexte :** `match_report/io/app_events` — publisher existant, étendu

## Objectif

À la publication d'un rapport de match, émettre un `PlayerMatchImpactAppEvent` par
action de joueur `Regular`, plus un `TeamMatchConcluded` par équipe — en plus de
l'émission existante de `MatchReportPublished` (inchangée). Spec :
`docs/specs/player-match-impact/player-report-events/07-integration.md` §5.

---

## Conception

### Signature étendue — `app_event_publisher.rs`

```rust
pub fn match_report_app_event_publisher(
    event_bus: &EventBus,
    app_event_bus: EventBus,
    repo: Arc<dyn IMatchReportRepository>,
    competition_data: Arc<dyn ICompetitionDataPort>,   // nouveau
    team_data: Arc<dyn ITeamDataPort>,                  // nouveau
) {
```

Ports déjà existants (`ports.rs`), déjà utilisés ailleurs dans `match_report` pour
l'affichage — seule leur injection dans **ce** publisher est nouvelle.

### Dans le handler `MatchReportPublished`, après confirmation de l'état `Published(p)`

1. Résoudre **une seule fois** (pas par action) : `competition_data.find_round_context(&p.season_id, &p.round_id)` → `round_label` ; `team_data.find_team_info(&p.home_team_id)` / `find_team_info(&p.away_team_id)` → noms des deux équipes.
2. Construire le `PlayerMatchContextPayload` commun pour le côté home (opponent = away) et away (opponent = home).
3. Pour chaque action de `p.home_actions` puis `p.away_actions` : filtrer `ActionPlayer::Regular(player_id)` uniquement (`ActionPlayer::Temp` ignoré) ; mapper `MatchActionType` → variant :

   | `MatchActionType` | `PlayerMatchImpactAppEvent` |
   |---|---|
   | `Touchdown` | `PlayerPerformedTouchdown` |
   | `Passe` **ou** `Lancer` | `PlayerPerformedPass` (fusionnés — même notion domaine) |
   | `Interception` | `PlayerPerformedInterception` |
   | `Sortie` | `PlayerPerformedCasualty` |
   | `Mvp` | `PlayerPerformedMvp` |
   | `Agression` | `PlayerPerformedFoul` |
   | `Blesse{injury}` | `PlayerInjured{injury_type: map_injury(injury)}` — mapping **structuré** complet (avec `stat` pour `Sequel`), ne pas réutiliser `injury_label()` existant qui perd cette info |

   Émettre chaque event sur `app_event_bus`.
4. Émettre `TeamMatchConcluded{team_id: home_team_id, ...}` et `TeamMatchConcluded{team_id: away_team_id, ...}` — une fois chacun, indépendamment du nombre d'actions.

`build_published_payload` (existant, `MatchReportPublished` "classique") reste
inchangé — les deux émissions cohabitent dans le même handler.

### Wiring — `context.rs` / `main.rs`

- `match_report::context::init_listeners()` gagne 2 paramètres (`competition_data`, `team_data`), transmis à `match_report_app_event_publisher`.
- `main.rs` (`match_report::context::init_listeners(&event_bus, &app_event_bus, pool.clone())`) : réutiliser les mêmes instances `Arc<dyn ICompetitionDataPort>`/`Arc<dyn ITeamDataPort>` déjà construites pour `MatchReportContext::new` juste avant dans `main.rs` — pas de nouvel adapter à écrire, juste un clone d'`Arc` supplémentaire au call site.

---

## Checklist

- [ ] Signature `match_report_app_event_publisher` +2 params
- [ ] Résolution round_label + noms d'équipes une seule fois par publication
- [ ] Boucle sur `home_actions`/`away_actions`, filtre `Regular`, mapping complet (table ci-dessus)
- [ ] Émission `TeamMatchConcluded` ×2
- [ ] `context.rs::init_listeners()` +2 params
- [ ] `main.rs` call site mis à jour
- [ ] Tests unitaires du mapping `MatchActionType` → `PlayerMatchImpactAppEvent` (dont Passe+Lancer → même variant, Blesse{Sequel{stat}} → injury_type structuré complet)
