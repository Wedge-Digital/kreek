# BC `players` — Domaine : compteur de matchs joués + event `MatchConcluded`

**Priorité : haute**
**Dépend de :** rien (domaine pur)
**Contexte :** `players/domain` — agrégat, événement, méthode de commande

## Objectif

Ajouter un compteur `matches_played`, incrémenté à chaque fois que l'équipe du
joueur termine un match — indépendamment du fait que le joueur ait lui-même
joué ou non (confirmé explicitement). Cet event sert aussi d'ancre pour la
reconstruction de l'historique de matchs (carte 165).

---

## Conception

### `Player` — nouveau champ (`player.rs`)

```rust
pub matches_played: MatchesPlayedCount,   // nouveau compteur (match_impact.rs, même style que les autres)
```

Initialisé à 0 dans la branche `apply()` de `PlayerCreated`.

### Nouveau domain event (`events.rs`)

```rust
MatchConcluded {
    player_id: PlayerId,
    team_id:   TeamId,
    context:   MatchContext,
    team_score:     u8,
    opponent_score: u8,
}
```

Pas de champ `result` stocké — dérivé à l'affichage (cf. carte 160).

### Méthode de commande (infaillible, comme les autres)

```rust
pub fn record_match_concluded(&self, context: MatchContext, team_score: u8, opponent_score: u8) -> PlayerDomainEvent {
    PlayerDomainEvent::MatchConcluded {
        player_id: self.id.clone(), team_id: self.team_id.clone(), context, team_score, opponent_score,
    }
}
```

### `apply()` — nouvelle branche

```rust
PlayerDomainEvent::MatchConcluded { .. } => {
    let mut player = current?;
    player.matches_played.0 += 1;
    player.version += 1;
    Some(player)
}
```

Aucun autre effet — le résultat/score ne sont pas stockés sur l'agrégat, seulement
portés par l'event (consommés en lecture par la carte 165).

---

## Checklist

- [ ] `MatchesPlayedCount` dans `match_impact.rs`
- [ ] Champ `matches_played` sur `Player` + init dans `PlayerCreated`
- [ ] Variant `MatchConcluded` dans `PlayerDomainEvent`
- [ ] Méthode `record_match_concluded`
- [ ] Branche `apply()`
- [ ] Tests unitaires : `matches_played` incrémenté par l'event, jamais par les autres events existants
