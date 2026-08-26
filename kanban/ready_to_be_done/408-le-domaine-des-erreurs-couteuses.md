# Le domaine des erreurs coûteuses

**Priorité : haute**
**Dépend de :** rien — première carte de la fonctionnalité
**Conception :** `docs/specs/erreurs-couteuses/ecran-du-jet/06-domaine.md`
**Fichiers :** `src/app/teams/domain/costly_mistakes.rs` (nouveau),
`domain/team.rs`, `domain/value_objects.rs`,
`use_cases/validate_dismissals_phase_use_case.rs`

## Objectif

Une équipe qui garde plus de 100 kPo après ses renvois entre dans une nouvelle
phase, et le domaine sait dire ce qu'il lui en coûte.

```
Recruitment → Dismissals → CostlyMistakes → ReadyToPlay
                        └──────────────────────↑  trésorerie < 100 kPo
```

## La table, telle qu'elle se relit

| Trésorerie | Crise évitée | Mineur | Majeur | Catastrophe |
|---|---|---|---|---|
| 100 – 199 | 2–6 | 1 | — | — |
| 200 – 299 | 3–6 | 1–2 | — | — |
| 300 – 399 | 4–6 | 2–3 | 1 | — |
| 400 – 499 | 5–6 | 3–4 | 1–2 | — |
| 500 – 599 | 6 | 4–5 | 2–3 | 1 |
| 600 et + | — | 5–6 | 3–4 | 1–2 |

**Les tranches sont fermées à la centaine**, là où le règlement écrit
`100-195`, `200-295`. Celui-ci suppose des montants en multiples de 5 kPo ; la
trésorerie est un `u32`, et une équipe à **197 kPo** ne doit tomber dans aucun
trou. Aucun montant régulier ne change de tranche pour autant.

**Un tableau de bornes parcouru**, pas un `match` sur des plages : il se relit à
côté du règlement, ligne pour ligne, et c'est cette lecture-là qui a trouvé le
trou des 195.

## Les effets

```rust
IncidentType::None        => Kpo(0)
IncidentType::Minor       => Kpo(d3 * 10)
IncidentType::Major       => Kpo((treasury.0 / 2) / 5 * 5)
IncidentType::Catastrophe => Kpo(treasury.0.saturating_sub(somme_2d6 * 10))
```

**L'arrondi porte sur la perte**, pas sur ce qui reste : à 345 kPo, un incident
majeur retire 170 et en laisse 175.

À vérifier **par un test, pas par le raisonnement** : sur des entiers,
`345 / 2 = 172` puis `172 / 5 * 5 = 170`, ce qui coïncide avec 172,5 arrondi.
C'est le genre d'égalité qui tient par accident — d'où le cas impair dans la
liste des tests.

## L'agrégat recalcule, il ne reçoit pas

```rust
pub fn apply_costly_mistakes(&self, roll: u8, damage_dice: Vec<u8>)
    -> Result<TeamDomainEvent, DomainError>
```

Il ne reçoit que **les dés bruts**. L'incident et la perte, il les établit
lui-même depuis sa propre trésorerie.

La forme rejetée les passait en paramètres : le use case aurait pu produire un
événement disant « incident mineur, 2 000 kPo perdus » sans que rien ne l'en
empêche, et **l'agrégat aurait signé un fait qu'il n'a pas établi**. Le prix est
un double appel à une fonction pure sur deux entiers.

## Deux sorties pour la validation des renvois

```rust
pub fn validate_dismissals_phase(&self) -> Result<TeamDomainEvent, DomainError> {
    self.expect_phase(GamePhase::Dismissals)?;
    Ok(if self.treasury.0 >= SEUIL_ERREURS_COUTEUSES {
        TeamDomainEvent::CostlyMistakesPhaseStarted
    } else {
        TeamDomainEvent::DismissalsPhaseValidated
    })
}
```

La règle vit dans la méthode de commande ; `apply()` reste bête, il applique un
fait sans en décider. **Aucune migration** : les équipes dont l'historique ne
porte que `DismissalsPhaseValidated` se rejouent à l'identique.

Le commentaire de `team.rs:573` sur la retraite temporaire **reste vrai et doit
être conservé** — c'est `DismissalsPhaseValidated` qui saute la carte 39, pas la
branche nouvelle.

**Une hypothèse à ne pas perdre** : `cloturer_la_phase` travaille sur l'agrégat
chargé **avant** l'application des renvois. C'est juste parce qu'**un renvoi ne
rembourse rien**. Si cela changeait un jour, la méthode devrait recevoir la
trésorerie d'après-lot au lieu de la lire dans `self`.

## Le use case de validation change de signature

Il rend `Result<(), …>` ; il rendra `Result<ValidateDismissalsOutcome, …>`,
l'issue se lisant sur le dernier événement du lot. **Sans cette adaptation, le
projet ne compile plus** — elle appartient donc à cette carte.

## Checklist

- [ ] `GamePhase::CostlyMistakes` et `TeamDomainEvent::CostlyMistakesPhaseStarted`
- [ ] `CostlyMistakesApplied` gagne `damage_dice: Vec<u8>` en `#[serde(default)]`
- [ ] `domain/costly_mistakes.rs` : table, `incident_for`, `loss_for`, `dice_needed`
- [ ] `apply_costly_mistakes(roll, damage_dice)` sur `Team`
- [ ] `validate_dismissals_phase` à deux sorties ; `apply()` pour la nouvelle phase
- [ ] `ValidateDismissalsOutcome` et l'adaptation du use case
- [ ] Tests unitaires :
  - [ ] **les 36 cas de la table** — six tranches × six jets
  - [ ] 99 kPo → `DismissalsPhaseValidated` ; **100 kPo** → `CostlyMistakesPhaseStarted`
  - [ ] 197 kPo → tranche 100-199
  - [ ] mineur : 1D3 de 1, 2, 3 → 10, 20, 30 kPo
  - [ ] majeur : 345 → 170 ; 300 → 150 ; **347 → 170** (cas impair)
  - [ ] catastrophe : 560 avec (3,4) → perte 490, reste 70
  - [ ] `incident_for` sous 100 kPo → `None`, sans panique
  - [ ] `apply_costly_mistakes` hors phase → erreur, aucun événement
- [ ] `make lint`, `make check-arch`, `make test`

**Les 36 cas ne sont pas du zèle.** C'est la seule règle du projet dont une
erreur ne se voit pas : un incident majeur là où il fallait un mineur retire de
l'argent sans que personne ne puisse le contester.
