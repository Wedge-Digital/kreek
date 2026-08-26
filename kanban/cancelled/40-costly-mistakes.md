> **Carte annulée le 2026-08-26 — devenue l'épic [E13](../epics/ready_to_be_done/E13-gestion-des-erreurs-couteuses.md).**
>
> Ses deux « à définir » ont trouvé leur réponse, et la fonctionnalité est
> désormais spécifiée en entier par le workflow feature :
> `docs/specs/erreurs-couteuses/`, huit phases, et quatre cartes — **408 à 411**.
>
> Ce que cette carte contenait et qui a survécu : la table de déclenchement,
> reprise telle quelle dans la 408, aux tranches près — fermées à la centaine
> plutôt qu'à 195, 295, 395, pour qu'une trésorerie de 197 kPo ne tombe dans
> aucun trou.
>
> Ce qui a changé depuis son écriture : **c'est le système qui tire le dé**, et
> non le coach comme cette carte le prévoyait « par cohérence avec le principe
> établi ». La phase se joue seul devant un écran, sans dé sous la main —
> exception assumée, écrite dans la spec.

# BC `teams` — Erreur couteuse + retour "Prête à jouer"

**Priorité : haute**
**Dépend de :** `39-temporary-retirement-phase.md`
**Contexte :** `teams` — automatisme système

## Objectif

Appliquer automatiquement les erreurs couteuses après la phase de retraite temporaire, puis faire repasser l'équipe en `ReadyToPlay`.

---

## Conception

### Règles BB2025 — Erreur couteuse (source : dadidimerda.it#card-exp-mistakes)

Un seul jet de 1D6 par équipe. La table de déclenchement dépend du montant en trésorerie :

| Trésorerie | Pas d'incident | Incident mineur | Incident majeur | Catastrophe |
|---|---|---|---|---|
| 100–195 kPo | 2–6 | 1 | — | — |
| 200–295 kPo | 3–6 | 1–2 | — | — |
| 300–395 kPo | 4–6 | 2–3 | 1 | — |
| 400–495 kPo | 5–6 | 3–4 | 1–2 | — |
| 500–595 kPo | 6 | 4–5 | 2–3 | 1 |
| 600 kPo+ | — | 5–6 | 3–4 | 1–2 |

Moins de 100 kPo en trésorerie : aucun jet effectué.

**À compléter** : les effets exacts de chaque type d'incident (perte d'argent, blessure ?) ne figurent pas sur la source. À récupérer dans le règlement complet.

| Type d'incident | Effet |
|---|---|
| Pas d'incident | Aucun |
| Incident mineur | *(à définir)* |
| Incident majeur | *(à définir)* |
| Catastrophe | *(à définir)* |

### Mécanisme

Comme pour les fans dévoués, **c'est le coach qui tire le dé physiquement** et saisit le résultat sur la plateforme — cohérence avec le principe établi en question 2/3. Le serveur applique l'effet correspondant selon la trésorerie courante.

### Déclenchement

Appelé automatiquement par `ValidateRetirementPhaseCommand` use case, sans UI spécifique. Le résultat est affiché dans un fragment de notification.

```rust
pub enum IncidentType {
    None,
    Minor,
    Major,
    Catastrophe,
}

pub struct CostlyMistakesResult {
    pub incident:    IncidentType,
    pub gp_lost:     u32,
    pub description: String,
}

pub fn apply_costly_mistakes(&mut self, dice_roll: u8) -> Result<CostlyMistakesResult, DomainError> {
    // détermine IncidentType selon treasury + dice_roll
    // applique la perte sur self.treasury
    // self.game_phase = Some(GamePhase::ReadyToPlay)
}
```

### Affichage

Fragment HTMX affiché au coach résumant l'incident tiré et le montant perdu, puis message "Votre équipe est prête pour le prochain match".

---

## Points en suspens

- **Effets des incidents** : montants perdus pour Minor / Major / Catastrophe (non présents sur la source)

---

---

## Checklist

- [ ] Service de tirage aléatoire (réutiliser ou créer un `DiceService`)
- [ ] `Team::apply_costly_mistakes(rolls)` avec table de résultats
- [ ] `CostlyMistakesResult` value object
- [ ] Intégration dans le use case `ValidateRetirementPhase`
- [ ] Fragment HTML de résumé des erreurs couteuses
- [ ] Test unitaire : pour chaque jet (1–6), vérifier la déduction correcte
