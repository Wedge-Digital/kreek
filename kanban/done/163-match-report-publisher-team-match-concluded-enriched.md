# BC `match_report` — Publisher : émission enrichie de `TeamMatchConcluded`

**Priorité : haute**
**Dépend de :** `160-shared-kernel-team-match-concluded-enriched.md`
**Contexte :** `match_report/io/app_events` — publisher existant (carte 157), étendu

## Objectif

Peupler les nouveaux champs de `TeamMatchConcluded` (carte 160) au moment de
l'émission — toutes les données sont déjà disponibles dans le scope existant,
aucun nouvel appel de port nécessaire.

---

## Conception

Fichier : `src/app/match_report/io/app_events/app_event_publisher.rs`,
fonction `publish_player_impact_events` (déjà présente, carte 157).

`round_label`/`home_team_name`/`away_team_name` sont déjà résolus une fois par
publication (réutilisés tels quels). Les scores sont déjà calculables via
`count_touchdowns(&p.home_actions)` / `count_touchdowns(&p.away_actions)`
(fonction privée déjà présente dans ce fichier, carte 157).

```rust
let home_score = count_touchdowns(&p.home_actions);
let away_score = count_touchdowns(&p.away_actions);

let _ = app_event_bus.send(
    PlayerMatchImpactAppEvent::TeamMatchConcluded {
        team_id:            p.home_team_id.to_string(),
        match_report_id:    p.id.to_string(),
        round_id:           p.round_id.to_string(),
        round_label:        round_label.clone(),
        opponent_team_id:   p.away_team_id.to_string(),
        opponent_team_name: away_team_name.clone(),
        team_score:         home_score,
        opponent_score:     away_score,
    }
    .to_enveloppe(),
);
let _ = app_event_bus.send(
    PlayerMatchImpactAppEvent::TeamMatchConcluded {
        team_id:            p.away_team_id.to_string(),
        match_report_id:    p.id.to_string(),
        round_id:           p.round_id.to_string(),
        round_label,
        opponent_team_id:   p.home_team_id.to_string(),
        opponent_team_name: home_team_name,
        team_score:         away_score,
        opponent_score:     home_score,
    }
    .to_enveloppe(),
);
```

Remplace les deux émissions existantes (déjà présentes, carte 157) — pas de
nouvel appel réseau/port, juste des champs supplémentaires peuplés depuis des
valeurs déjà en mémoire à cet endroit.

---

## Checklist

- [ ] Émission `TeamMatchConcluded` (home) enrichie
- [ ] Émission `TeamMatchConcluded` (away) enrichie, valeurs inversées (adversaire/score)
- [ ] Test unitaire : scores et labels corrects pour les deux camps (asymétrie home/away)
