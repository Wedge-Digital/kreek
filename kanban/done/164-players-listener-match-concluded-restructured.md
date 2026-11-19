# BC `players` — Listener `TeamMatchConcluded` restructuré

**Priorité : haute**
**Dépend de :** `160-shared-kernel-team-match-concluded-enriched.md`, `161-players-domain-match-concluded.md`, `162-players-persistence-match-concluded.md`, `163-match-report-publisher-team-match-concluded-enriched.md`
**Contexte :** `players/io/app_events/team_match_concluded_listener.rs` — restructuration du listener existant (carte 158)

## Objectif

Le listener actuel ne fait que lever `MissingNextGame` → `Available`. Il doit
désormais, pour **chaque** joueur de l'équipe (pas seulement ceux en
`MissingNextGame`) : enregistrer `MatchConcluded` (compteur + historique), et
en plus, restaurer la disponibilité pour ceux qui étaient `MissingNextGame`.

---

## Conception

Fichier : `src/app/players/io/app_events/team_match_concluded_listener.rs`

```rust
async fn handle_team_match_concluded(
    player_repo: &dyn IPlayerRepository,
    team_id: &str,
    payload: &TeamMatchConcludedPayload,   // round_id, round_label, opponent_team_id/name, scores
) {
    let players = match player_repo.find_by_team_id(&TeamId(team_id.to_string())).await { /* inchangé */ };

    for player in &players {
        let context = to_match_context(payload);   // même mapping que player_match_impact_listener

        // 1. Toujours : MatchConcluded (compteur + ancre d'historique)
        let matched = player.record_match_concluded(context.clone(), payload.team_score, payload.opponent_score);
        let v1 = player.version + 1;
        let _ = player_repo.append(&player.id, &player.team_id, &matched, v1).await;

        // 2. En plus, si MissingNextGame : PlayerAvailabilityRestored (version suivante)
        if player.participation_status == PlayerParticipationStatus::MissingNextGame {
            let restored = player.restore_availability(MatchReportId(payload.match_report_id.clone()));
            let _ = player_repo.append(&player.id, &player.team_id, &restored, v1 + 1).await;
        }
    }
}
```

Point d'attention : deux appends possibles par joueur dans la même passe →
versions `player.version + 1` puis `+ 2`, pas de relecture entre les deux
(le `player` en mémoire ne change pas de statut après le premier append, donc
le test sur `participation_status` doit être fait **avant** le premier append,
sur l'état chargé initialement — pas de race, un seul listener consomme ce
flux).

---

## Checklist

- [ ] Signature du handler mise à jour pour recevoir les nouveaux champs de `TeamMatchConcluded`
- [ ] `MatchConcluded` enregistré pour tous les joueurs de l'équipe
- [ ] `PlayerAvailabilityRestored` toujours émis en plus pour les `MissingNextGame`, avec la bonne version (n+2)
- [ ] Tests d'intégration (vraie PgPool) : joueur sain → seul `matches_played` incrémenté ; joueur `MissingNextGame` → `matches_played` incrémenté **et** statut restauré
