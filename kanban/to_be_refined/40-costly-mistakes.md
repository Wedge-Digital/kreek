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

## Ce que le code porte déjà — constat du 2026-08-25

Cette carte est passée en `done/` le 2026-08-18, dans le commit `2bd45c3` qui a
clos l'épic E01 « par vérification une par une dans le code ». La vérification a
vu les types et l'événement, qui existent bel et bien ; elle n'a pas vu que
**personne ne les produit**. La checklist ci-dessous n'a jamais eu une seule
case cochée.

**Ce qui existe — tout l'aval :**

| | Où |
|---|---|
| `TeamDomainEvent::CostlyMistakesApplied { roll, incident, gp_lost }` | `teams/domain/team.rs:160` |
| Le débit, **écrêté au solde** (perdre 50 avec 30 retire 30, sans report) | `teams/domain/treasury.rs`, testé |
| La ligne au grand livre, motif `CostlyMistake` | `teams/io/repository/team_repository.rs` |
| Le retour en `ReadyToPlay` | `team.rs:583` |
| `IncidentType { None, Minor, Major, Catastrophe }` | `teams/domain/value_objects.rs:110` |
| Les deux listeners qui réagissent — recalcul de TV, purge des paniers | `teams/io/listeners/` |

**Ce qui manque — tout l'amont, c'est-à-dire tout ce qui décide :**

- aucun émetteur : l'événement n'est construit que dans un test
  (`team_repository.rs:828`, sous `#[cfg(test)]`) ;
- aucune méthode `Team::apply_costly_mistakes` — elle est dans cette carte, pas
  dans le code ;
- aucune table, aucun seuil : rien ne sait que le jet ne se fait qu'au-delà de
  100 kPo, ni comment trésorerie × 1D6 donne un type d'incident ;
- aucun effet chiffré : ce que coûtent Minor, Major et Catastrophe n'est écrit
  nulle part — c'est le point en suspens ci-dessous, jamais levé ;
- aucun use case, aucune route, aucun écran pour saisir le jet du coach ;
- **la séquence d'après-match ne passe jamais par là** :
  `DismissalsPhaseValidated` renvoie directement en `ReadyToPlay`, avec ce
  commentaire dans `team.rs:573` — « Simplification temporaire : la retraite
  temporaire (carte 39, to_be_refined) n'étant pas encore implémentée ». La
  carte 39 est, elle aussi, dans `done/` sans code, et ses sept cases sont
  vides.

**Conséquence sur l'épic E01**, close le 2026-08-18 : son critère « une équipe
traverse une saison complète — […] paie ses erreurs coûteuses […] » n'est pas
vérifié. À trancher : rouvrir l'épic, ou réécrire son critère pour dire ce
qu'elle couvre réellement.

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
