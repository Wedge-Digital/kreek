# BC match_report — Use case record_fan_factor (extension TV + routing)

**Priorité : haute**
**Dépend de :** 106, 108
**Contexte :** match_report step2-inducements — use case

## Objectif

Modifier `record_fan_factor_use_case` pour capturer les TeamValues des deux équipes après le fan factor, et décider si on redirige vers la phase inducements ou directement vers step 3.

## Conception

Cf. `docs/specs/match-report/step2-inducements/05-use-cases.md`

### Modifications (`use_cases/record_fan_factor_use_case.rs`)

Nouveau type de retour :

```rust
pub enum RecordFanFactorOutcome {
    RedirectToInducements { topdog_team_id: String },
    RedirectToStep3,
}
```

Orchestration ajoutée après l'enregistrement du fan factor :

1. Fetch `team_data.find_team_value(home_id)` + `find_team_value(away_id)` en parallèle
2. Appelle `pm.record_team_values(home_tv, away_tv, recorded_by)` → event `TeamValuesRecorded`
3. Persiste `FanFactorRecorded` + `TeamValuesRecorded` dans la même transaction via `append_many`
4. Fetch `competition_data.find_tier_rules_for_roster(season_id, topdog_roster_id)`
5. Si listes vides → `RedirectToStep3` ; sinon → `RedirectToInducements { topdog_team_id }`

### Erreur ajoutée

```rust
TeamValueUnavailable(String),
```

### Handler (`io/web/pre_match_controller.rs`)

`post_pre_match` interprète le nouvel `RecordFanFactorOutcome` pour construire le redirect.

## Checklist

- [ ] `RecordFanFactorOutcome` avec les deux variants
- [ ] Fetch des deux TV en parallèle
- [ ] Appel `pm.record_team_values()`
- [ ] `append_many` avec les deux events dans une transaction
- [ ] Routing : vérification inducements disponibles → redirect approprié
- [ ] Erreur `TeamValueUnavailable`
- [ ] Handler `post_pre_match` mis à jour pour interpréter le nouvel outcome
- [ ] Tests unitaires du use case
