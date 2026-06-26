# Step 2 — Avant-match — Domaine

## Récapitulatif des règles métier

1. **D3Roll** : valeur strictement dans {1, 2, 3}. Toute autre valeur est rejetée.
2. **Fan Factor** = Dedicated Fans (donnée équipe) + D3Roll. Calculé et persisté pour chaque équipe.
3. **Enregistrement** : le fan factor ne peut être enregistré que si le match report est en état `PreMatch`.
4. **Écrasement** : si le fan factor a déjà été enregistré, un nouvel appel écrase les valeurs précédentes en émettant un nouvel événement `FanFactorRecorded`.
5. **Rehydratation** : robuste aux multiples `FanFactorRecorded` — le dernier événement prévaut.
6. **Journeymen** : ajoutés automatiquement si < 11 joueurs. Informatif à cette étape (pas de mutation domaine ici).
7. **Inducements order** : l'équipe avec la CTV la plus haute achète en premier. Informatif à cette étape.

## Value object : D3Roll

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct D3Roll(u8);

impl D3Roll {
    pub fn try_new(value: u8) -> Result<Self, DomainError> {
        if (1..=3).contains(&value) {
            Ok(Self(value))
        } else {
            Err(DomainError::InvalidD3Roll(value))
        }
    }

    pub fn value(&self) -> u8 { self.0 }
}
```

## Événement domaine : FanFactorRecorded

```rust
FanFactorRecorded {
    home_fan_roll: D3Roll,
    away_fan_roll: D3Roll,
    recorded_by: CoachId,
}
```

Ajouté à l'enum `MatchReportDomainEvent`.

## Méthode agrégat : PreMatch::record_fan_factor

```rust
impl MatchReportPreMatch {
    pub fn record_fan_factor(
        &self,
        home_fan_roll: D3Roll,
        away_fan_roll: D3Roll,
        recorded_by: CoachId,
    ) -> (MatchReportPreMatch, MatchReportDomainEvent) {
        let event = MatchReportDomainEvent::FanFactorRecorded {
            home_fan_roll,
            away_fan_roll,
            recorded_by,
        };
        let mut updated = self.clone();
        updated.home_fan_roll = Some(home_fan_roll);
        updated.away_fan_roll = Some(away_fan_roll);
        updated.version += 1;
        (updated, event)
    }
}
```

Pas de Result — l'appel est toujours valide si on est en PreMatch (vérifié par le use case).

## Extension de l'agrégat PreMatch

Ajouter deux champs optionnels :

```rust
pub struct MatchReportPreMatch {
    // ... champs existants ...
    pub home_fan_roll: Option<D3Roll>,
    pub away_fan_roll: Option<D3Roll>,
}
```

Initialisés à `None` dans `from_draft()`. Mis à jour par la rehydratation de `FanFactorRecorded`.

## Rehydratation

Dans `match_report_state.rs`, ajouter le cas :

```rust
(Some(MatchReportState::PreMatch(pm)), MatchReportDomainEvent::FanFactorRecorded {
    home_fan_roll, away_fan_roll, ..
}) => {
    let mut updated = pm;
    updated.home_fan_roll = Some(*home_fan_roll);
    updated.away_fan_roll = Some(*away_fan_roll);
    updated.version += 1;
    MatchReportState::PreMatch(updated)
}
```

Le dernier `FanFactorRecorded` écrase toujours les valeurs précédentes.

## Erreur domaine

Ajouter à `DomainError` :

```rust
InvalidD3Roll(u8),
```

## Tests unitaires prévus

1. `d3roll_accepte_1_2_3` — D3Roll::try_new(1), (2), (3) → Ok
2. `d3roll_rejette_0_et_4` — D3Roll::try_new(0), (4) → Err
3. `record_fan_factor_emet_evenement` — appel sur PreMatch → event FanFactorRecorded
4. `record_fan_factor_met_a_jour_les_champs` — PreMatch.home_fan_roll/away_fan_roll mis à jour
5. `rehydratation_fan_factor` — rehydrate avec FanFactorRecorded → PreMatch avec les bons rolls
6. `rehydratation_double_fan_factor` — deux FanFactorRecorded → le second écrase le premier
