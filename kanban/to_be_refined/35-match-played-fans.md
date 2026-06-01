# BC `teams` — Consommation `MatchPlayed` + fans dévoués → phase "Amélioration joueurs"

**Priorité : haute**
**Dépend de :** `32-team-enrollment.md`
**Contexte :** `teams` (consommateur) ← BC `match_report` (émetteur)

## Objectif

Écouter l'app event `MatchPlayed` publié par le BC `match_report`, mettre à jour le compteur de fans dévoués (calcul automatique), et faire transiter l'équipe vers la phase `PlayerImprovement`.

---

## Conception

### App event attendu de `match_report`

```rust
MatchPlayed {
    event_id:        String,
    team_id:         String,
    opponent_id:     String,
    result:          String,      // "win" | "draw" | "loss"
    touchdowns_for:  u8,
    touchdowns_against: u8,
    dedicated_fans_roll: u8,      // 1D6 tiré par match_report au moment de la saisie
    spp_gains:       Vec<SppGain>, // { player_id, spp_earned }
    treasury_income: u32,         // revenus du match en kPo
}
```

### Calcul fans dévoués

Règle BB2020 : `nouveaux_fans = dedicated_fans_roll + modificateur_résultat`
- Victoire : +1
- Nul : 0
- Défaite : -1

Le nouveau facteur de fans remplace l'ancien (pas cumulatif).

Le jet est inclus dans le payload `MatchPlayed` — le BC `teams` se contente d'appliquer la valeur, sans tirage de sa part.

### Listener

```rust
// io/app_events/match_played_listener.rs
// Reçoit MatchPlayed →
//   1. charge l'équipe
//   2. team.start_post_match_sequence(result, fans_roll, spp_gains, treasury_income)
//   3. save()
```

### Transition domaine

```rust
pub fn start_post_match_sequence(
    &mut self,
    result:          MatchResult,
    fans_roll:       u8,
    spp_gains:       Vec<SppGain>,
    treasury_income: u32,
) -> Result<(), DomainError> {
    // garde : doit être Enrolled + ReadyToPlay
    self.update_dedicated_fans(result, fans_roll); // automatique
    self.treasury += treasury_income;
    self.pending_spp_gains = spp_gains;            // stockés pour la phase suivante
    self.game_phase = Some(GamePhase::PlayerImprovement);
    Ok(())
}
```

---

## Points en suspens

- Le BC `teams` doit-il stocker les `spp_gains` en attente, ou le BC `players` les reçoit directement ?

---

## Checklist

- [ ] Aligner le payload exact de `MatchPlayed` avec le BC `match_report` (champs ci-dessus)
- [ ] `match_played_listener::init()` dans `teams`
- [ ] `MatchResult` value object (si pas déjà dans carte 28)
- [ ] `SppGain` value object `{ player_id, spp_earned }`
- [ ] `Team::start_post_match_sequence()` avec calcul fans + transition
- [ ] Colonne `pending_spp_gains` dans la table `teams` (JSON) ou table séparée
- [ ] Test unitaire du calcul des fans (V/N/D × jet de dé)
- [ ] Test d'intégration : event → transition en base
