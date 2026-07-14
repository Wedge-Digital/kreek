# BC `players` — Domain service : résolution des stats finales

**Priorité : moyenne**
**Dépend de :** `154-players-domain-match-impact.md`
**Contexte :** `players/use_cases` — couche applicative, pas domaine

## Objectif

Calculer les statistiques finales d'un joueur (MA/ST/AG/PA/AV) en combinant la stat
de base de son poste (`references`) avec ses `stat_adjustments` accumulés
(séquelles). Le domaine `Player` reste pur — il ne stocke que le delta, jamais la
valeur résolue (BR13, `06-domaine.md`).

Rien dans le reste de la feature ne consomme encore ce service (pas de fiche joueur
câblée) — carte de fondation pour une future carte d'affichage, mais la correction
de la logique de résolution fait partie du périmètre validé de cette feature.

---

## Conception

### Nouveau fichier `src/app/players/use_cases/player_stats_service.rs`

```rust
pub struct ResolvedPlayerStats { pub ma: u8, pub st: u8, pub ag: u8, pub pa: u8, pub av: u8 }

pub fn resolve_stats(player: &Player, ref_repo: &dyn IReferenceRepository) -> Option<ResolvedPlayerStats> {
    let base = ref_repo.find_position_by_uid(player.roster_line_id.as_ref())?;
    let mut stats = ResolvedPlayerStats { ma: base.ma, st: base.st, ag: base.ag, pa: base.pa, av: base.av };
    for adj in &player.stat_adjustments {
        match adj.stat {
            // MA/ST : plus haut = meilleur → le malus DIMINUE la valeur
            StatKind::Ma => stats.ma = stats.ma.saturating_sub(adj.malus),
            StatKind::St => stats.st = stats.st.saturating_sub(adj.malus),
            // AG/PA/AV : nombres cibles de dé, plus bas = meilleur → le malus AUGMENTE la valeur
            StatKind::Ag => stats.ag = stats.ag.saturating_add(adj.malus),
            StatKind::Pa => stats.pa = stats.pa.saturating_add(adj.malus),
            StatKind::Av => stats.av = stats.av.saturating_add(adj.malus),
        }
    }
    Some(stats)
}
```

**Pas de nouveau port ACL.** `find_position_by_uid` existe déjà sur
`IReferenceRepository` et retourne déjà MA/ST/AG/PA/AV (`PlayerPosition`). `players`
a déjà un précédent d'accès direct à ce trait
(`io/app_events/team_created_listener.rs`) — on le réutilise, pas de nouvelle
couche d'abstraction.

---

## Checklist

- [ ] `src/app/players/use_cases/player_stats_service.rs` — `resolve_stats()`
- [ ] Tests unitaires : malus MA/ST diminue la valeur, malus AG/PA/AV l'augmente, plusieurs `stat_adjustments` cumulés sur la même stat s'additionnent, aucun ajustement → stats de base inchangées
