# Shared kernel — `TeamMatchConcluded` enrichi

**Priorité : haute**
**Dépend de :** rien
**Contexte :** `shared_kernel/app_events` — contrat inter-BC `match_report` → `players`

## Objectif

Enrichir `TeamMatchConcluded` (déjà émis, carte 157) avec les données nécessaires
pour reconstruire une carte d'historique de match côté `players` : journée,
adversaire, scores. Fondation de la carte de détail joueur
(`docs/specs/player-match-impact/` — suite).

---

## Conception

Fichier : `src/app/shared_kernel/app_events/player_match_impact_app_events.rs`

```rust
PlayerMatchImpactAppEvent::TeamMatchConcluded {
    team_id: String,
    match_report_id: String,
    round_id: String,           // nouveau
    round_label: String,        // nouveau
    opponent_team_id: String,   // nouveau
    opponent_team_name: String, // nouveau
    team_score: u8,             // nouveau
    opponent_score: u8,         // nouveau
}
```

Pas de champ `result` (Victoire/Défaite/Nul) — dérivé à l'affichage par
comparaison des scores (`team_score.cmp(&opponent_score)`), même logique que
`build_team_banner` existant (`match_report/io/web/builders.rs`). Une seule
source de vérité, pas de redondance.

---

## Checklist

- [ ] Champs ajoutés au variant `TeamMatchConcluded`
- [ ] `to_enveloppe()`/`event_type()` inchangés (même variant, juste plus de champs)
