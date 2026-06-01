# BC `teams` — Repos hors-saison

**Priorité : moyenne**
**Dépend de :** `39-temporary-retirement-phase.md`, BC `competitions` (fin de saison)
**Contexte :** `teams` — action coach + automatisme

## Objectif

Gérer la phase de fin de saison pendant laquelle le coach décide de réengager ou non chaque joueur en retraite temporaire, avant que l'équipe puisse s'inscrire à une nouvelle saison.

---

## Ce qui est défini

- La phase **repos hors-saison** est déclenchée en fin de saison (événement émis par BC `competitions`)
- Le coach passe en revue chaque joueur en retraite temporaire et choisit :
  - **Réengager** : le joueur retrouve son statut actif, disponible pour la saison suivante
  - **Ne pas réengager** : le joueur quitte définitivement l'effectif (libère son slot de quota)
- Un joueur en retraite temporaire **non réengagé** est différent d'un renvoi — c'est un départ à l'amiable en fin de saison
- Une fois tous les choix faits, l'équipe repasse en `PendingEnrollment` et peut s'inscrire à une nouvelle saison

---

## Ce qui reste à définir

- Quel événement du BC `competitions` déclenche la phase hors-saison ? (`SeasonEnded` ?)
- La phase hors-saison a-t-elle d'autres actions que la gestion des retraités temporaires (ex. : augmentation de salaires, vieillissement des joueurs selon BB2020) ?
- Un délai est-il imposé au coach pour statuer, ou la phase reste ouverte indéfiniment ?
- La transition vers `PendingEnrollment` est-elle automatique après que tous les joueurs ont été traités, ou le coach valide explicitement ?
- Le `GamePhase` doit-il avoir un variant `OffSeason` dédié, ou la phase hors-saison modifie-t-elle directement le `ParticipationStatus` ?

---

## Ébauche de conception

### Nouveaux événements domaine pressentis

```rust
TeamDomainEvent::OffSeasonStarted { season_id: String },
TeamDomainEvent::PlayerReEngaged  { player_id: String },
TeamDomainEvent::PlayerNotReEngaged { player_id: String },
TeamDomainEvent::OffSeasonCompleted,
// → repasse l'équipe en PendingEnrollment / ReadyToPlay selon le flux
```

### Nouveau variant `GamePhase`

```rust
pub enum GamePhase {
    ReadyToPlay,
    PlayerImprovement,
    Recruitment,
    Dismissals,
    TemporaryRetirement,
    OffSeason,  // Repos hors-saison
}
```

---

## Checklist (à compléter après raffinage)

- [ ] Payload de l'événement `SeasonEnded` côté BC `competitions` à définir
- [ ] `GamePhase::OffSeason` + transitions dans `Team::apply()`
- [ ] Use case `start_off_season` (listener sur `SeasonEnded`)
- [ ] Use case `re_engage_player` + `release_player`
- [ ] Use case `complete_off_season` → retour `PendingEnrollment`
- [ ] Routes + UI de gestion des joueurs retraités temporaires
