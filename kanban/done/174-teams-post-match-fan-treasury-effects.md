# BC `teams` — Fan factor et trésorerie réels à la publication d'un rapport de match

**Priorité : haute**
**Dépend de :** `172-teams-match-report-published-listener.md` (listener existant, stubbé)
**Contexte :** `teams/domain` + `teams/io/app_events`

## Objectif

Remplacer les valeurs stubbées (`fans_roll=0`, `treasury_income=Kpo(0)`) du
listener `match_report_published_listener` par les valeurs réelles portées
par le rapport de match publié (`home_fan_mod`/`away_fan_mod`,
`home_gain_kpo`/`away_gain_kpo`), sans jamais croiser les données home/away.
Spec complète : `docs/specs/post-match-team-effects/README.md`.

---

## Conception

### Domaine (`domain/team.rs`)

Signature de `start_post_match_sequence` modifiée : `fans_roll: u8` devient
`fan_mod: i8` (valeur déjà finale du rapport, bornée -2..2 côté
`match_report`, aucun recalcul via `result.fan_modifier()`) :

```rust
pub fn start_post_match_sequence(
    &self,
    result: MatchResult,
    fan_mod: i8,
    treasury_income: Kpo,
    spp_gains: Vec<SppGain>,
) -> Result<TeamDomainEvent, DomainError> {
    self.expect_phase(GamePhase::MatchReporting)?;
    let raw = (self.dedicated_fans.into_inner() as i16 + fan_mod as i16).max(0) as u8;
    let dedicated_fans = DedicatedFans::try_new(raw.min(20)).expect("clamped to valid range");
    Ok(TeamDomainEvent::PostMatchSequenceStarted { result, dedicated_fans, treasury_income, spp_gains })
}
```

`result` reste stocké dans l'événement comme fait historique, mais ne sert
plus au calcul — `MatchResult::fan_modifier()` (`value_objects.rs`) devient
mort et est supprimé (plus aucun appelant).

### Listener (`io/app_events/match_report_published_listener.rs`)

`handle_team` prend en plus `fan_mod: i8` et `gain_kpo: u32`, transmis
séparément pour chaque équipe :

```rust
handle_team(&team_repo, &payload.home_team_id,
    derive_result(payload.home_score, payload.away_score),
    payload.home_fan_mod, payload.home_gain_kpo).await;
handle_team(&team_repo, &payload.away_team_id,
    derive_result(payload.away_score, payload.home_score),
    payload.away_fan_mod, payload.away_gain_kpo).await;
```

`handle_team` appelle `team.start_post_match_sequence(result, fan_mod, Kpo(gain_kpo), vec![])`.

### Event sourcing — rappel

Aucune mutation directe : la méthode domaine calcule et retourne
l'événement (`dedicated_fans`/`treasury_income` déjà calculés dedans),
`apply()` mute l'agrégat à partir de l'événement, rejouable via `hydrate()`.
Le listener ne fait qu'`append()`.

---

## Checklist

- [ ] `start_post_match_sequence` : `fans_roll: u8` → `fan_mod: i8`, calcul direct sans `fan_modifier()`
- [ ] `MatchResult::fan_modifier()` supprimé (devenu mort)
- [ ] `handle_team` du listener transmet `fan_mod`/`gain_kpo` distincts par équipe, jamais croisés
- [ ] Tests unitaires domaine mis à jour (`post_match_sequence_calculates_fans_correctly`, `post_match_sequence_clamps_fans_at_20`) pour la nouvelle signature
- [ ] Nouveau test : fan_mod négatif fait baisser `dedicated_fans` sans descendre sous 0
- [ ] Nouveau test (listener) : home et away reçoivent bien leurs propres valeurs respectives, jamais celles de l'adversaire
