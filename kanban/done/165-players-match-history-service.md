# BC `players` — Use case : reconstruction de l'historique de matchs

**Priorité : haute**
**Dépend de :** `161-players-domain-match-concluded.md`, `162-players-persistence-match-concluded.md`
**Contexte :** `players/use_cases/match_history_service.rs` — nouveau domain service

## Objectif

Reconstruire, à la lecture, la liste des cartes "match" affichées sur la fiche
joueur — sans rien stocker de plus sur l'agrégat (BR établie dès la conception
du domaine match-impact). Regroupe les events bruts d'un joueur par
`match_report_id` : `MatchConcluded` fournit l'en-tête (adversaire, journée,
score), les events d'action du même match fournissent le détail.

---

## Conception

### Nouveau fichier `src/app/players/use_cases/match_history_service.rs`

```rust
pub struct MatchHistoryEntry {
    pub match_report_id:    String,
    pub round_label:        String,
    pub opponent_team_name: String,
    pub team_score:         u8,
    pub opponent_score:     u8,
    pub actions:            Vec<MatchHistoryAction>,
}

pub struct MatchHistoryAction {
    pub kind:       MatchHistoryActionKind,
    pub spp_earned: Option<u32>,   // None pour Foul/Injury
}

pub enum MatchHistoryActionKind { Touchdown, Pass, Interception, Casualty, Mvp, Foul, Injury }

pub fn build_match_history(events: &[PlayerDomainEvent]) -> Vec<MatchHistoryEntry> {
    // 1. Parcourir les events dans l'ordre (déjà chronologique, version ASC)
    // 2. Pour chaque MatchConcluded rencontré, ouvrir/mettre à jour une entrée
    //    indexée par context.match_report_id (en-tête : adversaire, journée, scores)
    // 3. Pour chaque event d'action (TouchdownScored/PassCompleted/InterceptionMade/
    //    CasualtyInflicted/MatchMvpNamed/FoulCommitted/InjurySustained), ajouter une
    //    ligne à l'entrée correspondant à son context.match_report_id
    // 4. Retourner les entrées, ordre chronologique inversé (plus récent d'abord)
}
```

Note : un `match_report_id` peut avoir des events d'action **avant** d'avoir
son `MatchConcluded` (les actions sont émises pendant la boucle du publisher,
`TeamMatchConcluded` juste après, carte 163) — l'algorithme doit donc pouvoir
créer l'entrée à partir du premier event rencontré (action ou MatchConcluded)
et compléter l'en-tête quand `MatchConcluded` arrive, plutôt que d'exiger un
ordre strict.

---

## Checklist

- [ ] `MatchHistoryEntry`/`MatchHistoryAction`/`MatchHistoryActionKind`
- [ ] `build_match_history()` — regroupement tolérant à l'ordre relatif action/MatchConcluded
- [ ] Tests unitaires : plusieurs matchs distincts correctement séparés, actions rattachées au bon match, ordre le plus récent en premier, un match sans action (joueur non sollicité) apparaît quand même via son seul `MatchConcluded`
