# MatchDay accepte ou refuse un tirage

**Épic :** E18 — Appariement des présents · **Ordre :** 2 · **Dépend de :** 595
**Conception :** `docs/specs/appariement-des-presents/calendrier/06-domaine.md` (§ 2, § 4)

## Objectif

Que la journée soit le gardien de ce qu'on y écrit : un tirage proposé — par
l'aperçu des présents ou par « Générer une journée complète » — n'y entre que
s'il respecte R3, R4, R9 et R11. Le use case ne décide de rien ; il relaie le
refus.

## Conception

```rust
impl MatchDay {
    /// R9 — une journée de match, sans rencontres.
    pub fn can_receive_draw(&self) -> Result<(), DomainError>;

    /// R3, R4, R11 — accepte le tirage et y ajoute ses rencontres, ou refuse
    /// sans rien changer. Appelle d'abord `can_receive_draw`.
    pub fn accept_draw(
        &mut self,
        pairings: Vec<Pairing>,
        exempted: Option<TeamId>,
        rules: &DrawRules,
    ) -> Result<(), DomainError>;
}

pub struct DrawRules {
    pub enrolled: HashSet<TeamId>,
    pub forbidden: HashSet<(TeamId, TeamId)>, // même coach — clé normalisée par draw::pair
    pub pools: Vec<HashSet<TeamId>>,          // appartenance, sans identifiant
}
```

**Tout vérifier avant de rien pousser** : un refus laisse `pairings` intact. Les
aides privées `check_team`, `check_pair`, `check_exempted` gardent chaque
fonction sous vingt lignes.

| Variant de `DomainError` | Règle | Cas |
|---|---|---|
| `RestDayCannotBeDrawn` | R9 | journée de repos |
| `RoundAlreadyPaired` | R9, R11 | la journée porte déjà une rencontre |
| `TeamNotEnrolled { team }` | R11 | équipe absente de `enrolled` |
| `TeamDrawnTwice { team }` | R11 | une équipe dans deux rencontres, ou deux fois la même |
| `SameCoachPairing { home, away }` | R4 | paire dans `forbidden` |
| `PairingAcrossPools { home, away }` | R3 | les deux équipes ne partagent aucun ensemble de `pools` |
| `ExemptedTeamAlsoPaired { team }` | R11 | l'exemptée figure dans une rencontre |

Les messages `#[error(...)]` sont ceux que l'organisateur lira en toast :
« Halfling Legends n'est plus inscrite à la compétition », etc. Les identifiants
sont des `TeamId` ; les noms sont résolus par le handler.

**`pairings` reste `pub`** — encapsulation hors périmètre, carte 594.

## Checklist

- [x] `DrawRules`, `can_receive_draw`, `accept_draw` dans `domain/match_day.rs`
- [x] Les sept variants dans `domain/error.rs`
- [x] Tests : `can_receive_draw` refuse repos et journée pourvue ; `accept_draw`
      refuse chacun des cinq cas de R3, R4, R11 ; **ne modifie pas la journée**
      quand il refuse ; ajoute toutes les rencontres quand il accepte
- [x] `make lint`, `make check-arch`, `make test`
