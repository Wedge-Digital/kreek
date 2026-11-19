# BC `teams` — Phase d'amélioration des joueurs (dépense de SPP)

**Priorité : haute**
**Dépend de :** `35-match-played-fans.md`
**Contexte :** `teams` — action coach

## Objectif

Permettre au coach de **saisir** les améliorations de ses joueurs pendant la phase `PlayerImprovement`. La plateforme est un registre : les jets de dés sont effectués physiquement par le coach, qui reporte ici le résultat. Aucune simulation de dés côté serveur.

---

## Ce qui est défini

- La phase est une **saisie libre** : le coach entre quelle compétence ou amélioration de caractéristique a été obtenue pour chaque joueur éligible
- Le système **ne simule pas** les tirages aléatoires et **n'impose pas** les catégories Primary/Secondary
- Le détail de la gestion des montées de niveau (coût SPP par niveau, liste des compétences par roster) sera défini dans une carte dédiée ultérieurement

---

## Ce qui reste à définir

### UI de saisie

- Comment le coach sélectionne-t-il l'amélioration pour un joueur ? Saisie texte libre, ou liste de compétences prédéfinie à cocher ?
- Peut-il passer un joueur sans amélioration (il a des SPP mais choisit de ne pas les dépenser ce tour) ?
- La phase peut-elle être validée même si certains joueurs éligibles n'ont pas été traités ?

### Lien avec le BC `players`

- Les améliorations saisies ici génèrent-elles un événement vers le BC `players`, ou c'est la carte 41 qui gère la synchronisation ?

### Commande

```rust
pub struct ApplyPlayerImprovementCommand {
    pub team_id:     TeamId,
    pub player_id:   PlayerId,
    pub improvement: PlayerImprovement,
}

pub enum PlayerImprovement {
    NewSkill(String),  // nom de la compétence saisi/choisi par le coach
    StatBoost(Stat),   // MA | ST | AG | PA | AV
}

pub struct ValidateImprovementPhaseCommand {
    pub team_id: TeamId,
}
```

### Calcul de `value_delta`

Le use case `ApplyPlayerImprovement` doit calculer le `value_delta` avant d'appender l'event, selon les règles BB2025 :

| Type d'amélioration | `value_delta` |
|---|---|
| Nouvelle compétence (Primary) | +10 kPo |
| Nouvelle compétence (Secondary) | +20 kPo |
| Amélioration MA, PA, AV | +30 kPo |
| Amélioration ST | +50 kPo |
| Amélioration AG | +40 kPo |

*(valeurs à confirmer avec le référentiel BB2025)*

Le `value_delta` est inclus dans `PlayerImprovementApplied { player_id, improvement, value_delta }` — BC `teams` met ainsi à jour `team_value` de façon autonome.

---

## Checklist (à compléter après raffinage UI)

- [ ] `PlayerImprovement` enum
- [ ] Table `value_delta` par type d'amélioration (référentiel BB2025 à confirmer)
- [ ] `ApplyPlayerImprovementCommand` + use case : calcule `value_delta` → append `PlayerImprovementApplied { value_delta }`
- [ ] `ValidateImprovementPhaseCommand` + use case → `advance_game_phase()`
- [ ] Route GET (fragment UI) + route POST (saisir amélioration) + route POST (valider phase)
- [ ] Template fragment de saisie
- [ ] Publication vers BC `players` (TBD)
