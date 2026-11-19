# BC `teams` — Phase de recrutement (achat joueurs et staff)

**Priorité : haute**
**Dépend de :** `36-player-improvement-phase.md`
**Contexte :** `teams` — action coach

## Objectif

Permettre au coach d'acheter des joueurs et du staff pendant la phase `Recruitment`, dans les limites du budget disponible et des règles post-match, puis de valider pour passer aux renvois.

---

## Conception

### Règles post-match BB2020

- Achat de joueurs : mêmes contraintes de limites par poste que la création d'équipe
- Achat de relances supplémentaires : **prix doublé** par rapport au coût initial du roster
- Achat de staff (apothicaire, assistants, cheerleaders, facteur de fans) : prix normaux
- Budget : trésorerie disponible après revenus du match

### Commandes et use cases

```rust
pub struct RecruitPlayerCommand {
    pub team_id:    TeamId,
    pub player_pos: String,   // identifiant de position dans le roster
}

pub struct BuyStaffCommand {
    pub team_id:    TeamId,
    pub staff_type: StaffType,  // Reroll | Apothecary | Assistant | Cheerleader | FansFactor
    pub quantity:   u8,
}

pub struct ValidateRecruitmentPhaseCommand {
    pub team_id: TeamId,
}
```

### Route et UI

```
POST /app/{space_id}/teams/{team_id}/recruit-player
POST /app/{space_id}/teams/{team_id}/buy-staff
POST /app/{space_id}/teams/{team_id}/validate-recruitment
GET  /app/{space_id}/teams/{team_id}/recruitment-phase
```

L'UI réutilise des patterns proches de la phase de construction (carte 20-24) : catalogue de positions disponibles avec prix, panier courant, total trésorerie restante.

### Validation de phase

```rust
// team.advance_game_phase() → GamePhase::Dismissals
```

---

## Points en suspens

- Peut-on recruter des Star Players temporaires à ce stade ? (règle optionnelle BB2020)
- Les joueurs recrutés ici doivent-ils être publiés vers le BC `players` immédiatement ou en fin de phase ?
- Le référentiel des positions (avec prix) vient-il des JSON existants ou du BC `teams` ?

---

## Checklist

- [ ] `RecruitPlayerCommand` + use case (vérif budget, limite par poste, phase correcte)
- [ ] `BuyStaffCommand` + use case (prix doublé pour les relances en post-match)
- [ ] `ValidateRecruitmentPhaseCommand` + use case → `advance_game_phase()`
- [ ] Routes GET + POST dans `router.rs`
- [ ] Fragment UI recrutement
- [ ] Mise à jour trésorerie dans l'agrégat `Team`
- [ ] Publication vers BC `players` (TBD)
