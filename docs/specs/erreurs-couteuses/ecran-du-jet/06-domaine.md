# L'écran du jet · Phase 6 : domaine

**Entrée** : `05-use-cases.md` validé.

## Récapitulatif exhaustif des règles métier — validé

| # | Règle | Où elle vit |
|---|---|---|
| R1 | Trésorerie **≥ 100 kPo** → phase `CostlyMistakes` ; sinon `ReadyToPlay` direct | `validate_dismissals_phase()` |
| R2 | Six tranches **fermées à la centaine**, quatre issues | `domain/costly_mistakes.rs` |
| R3 | Rien · **1D3 × 10** · **moitié arrondie au 5 inférieur** · **tout sauf 2D6 × 10** | idem |
| R4 | L'arrondi porte sur **la perte**, pas sur le reste | idem |
| R5 | La phase s'insère entre `Dismissals` et `ReadyToPlay` ; la retraite temporaire reste hors du chemin | `apply()` |
| R6 | **Un seul jet** — un second est refusé par la phase | `expect_phase` |
| R7 | Le jet est tiré par le système, jamais fourni par le client | port + POST sans corps |
| R8 | L'événement porte `roll`, `damage_dice`, `incident`, `gp_lost` | `TeamDomainEvent` |
| R9 | Le débit est **écrêté au solde** | `TreasuryMovement::debit`, déjà en place |
| R10 | Lancer le dé : propriétaire, admin d'espace, admin de compétition | couche web, `ITeamAccessPort` |
| R11 | Pas de consultation après coup | écarté délibérément |

## La forme retenue : l'agrégat recalcule

```rust
pub fn apply_costly_mistakes(&self, roll: u8, damage_dice: Vec<u8>)
    -> Result<TeamDomainEvent, DomainError>
```

Il ne reçoit **que les dés bruts**. L'incident et la perte, il les établit
lui-même depuis sa propre trésorerie.

La forme rejetée passait `incident` et `gp_lost` en paramètres. Le use case
aurait alors pu produire un événement disant « incident mineur, 2 000 kPo
perdus » sans que rien ne l'en empêche : **l'agrégat aurait signé un fait qu'il
n'a pas établi**. Ici, l'événement ne peut pas mentir.

Le coût est un double appel à `incident_for` — une fois par le use case pour
savoir quels dés tirer, une fois par l'agrégat pour conclure. Fonction pure sur
deux entiers : le prix est nul, la garantie ne l'est pas.

## `domain/costly_mistakes.rs` — pur, sans dépendance

```rust
pub enum DiceNeeded { None, OneD3, TwoD6 }

/// Les six tranches du règlement, fermées à la centaine.
struct Band { min: u32, max: u32, safe: RangeInclusive<u8>, minor: …, major: …, catastrophe: … }

pub fn incident_for(treasury: Kpo, roll: u8) -> IncidentType;
pub fn loss_for(incident: IncidentType, treasury: Kpo, dice: &[u8]) -> Kpo;

impl IncidentType {
    pub fn dice_needed(&self) -> DiceNeeded;
}
```

**Une table de bornes parcourue, plutôt qu'un `match` sur des plages.** Elle se
relit à côté du règlement, ligne pour ligne, et une correction future s'y fait
sans toucher à du code. Le compilateur n'y gagne rien, mais la vérification
humaine oui — et c'est elle qui a trouvé le trou des tranches à 195.

**Les tranches sont fermées à la centaine** : `100..=199`, `200..=299`, …
`600..=u32::MAX`. Le règlement écrit `100-195` en supposant des montants en
multiples de 5 ; la trésorerie est un entier de kPo, et 197 ne doit tomber dans
aucun trou.

**`incident_for` sous 100 kPo rend `None`.** Ce cas ne peut pas se produire — R1
l'écarte en amont — mais une fonction pure ne doit pas paniquer sur une entrée
qu'elle sait nommer. Un `None` est la réponse juste : sans jet, pas d'incident.

## Le calcul des pertes

```rust
IncidentType::None        => Kpo(0)
IncidentType::Minor       => Kpo(d3 * 10)
IncidentType::Major       => Kpo((treasury.0 / 2) / 5 * 5)
IncidentType::Catastrophe => Kpo(treasury.0.saturating_sub(somme_2d6 * 10))
```

**L'arrondi de l'incident majeur, sur des entiers.** À 345 kPo : `345 / 2 = 172`
en division entière, puis `172 / 5 * 5 = 170`. Le résultat coïncide avec 172,5
arrondi au multiple de 5 inférieur. **À vérifier par un test sur un cas impair**,
pas par ce raisonnement — c'est le genre d'égalité qui tient par accident.

**La catastrophe utilise `saturating_sub`** alors qu'elle ne peut survenir
qu'au-delà de 500 kPo, où `2D6 × 10` plafonne à 120. La soustraction ne peut pas
passer sous zéro ; le `saturating_` est là pour que ce soit vrai même si la table
change.

## Les deux sorties de la validation des renvois

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

La règle vit dans la **méthode de commande**, pas dans `apply()`, qui applique un
fait sans en décider. Bénéfice de bord : les équipes dont l'historique ne porte
que `DismissalsPhaseValidated` se rejouent à l'identique — **aucune migration**.

Le commentaire de `team.rs:573` — « simplification temporaire, la retraite
temporaire n'étant pas implémentée » — reste vrai et doit être conservé : c'est
`DismissalsPhaseValidated` qui saute la carte 39, pas la nouvelle branche.

## `apply()` reste bête

```rust
TeamDomainEvent::CostlyMistakesPhaseStarted => {
    self.game_phase = Some(GamePhase::CostlyMistakes);
}
TeamDomainEvent::CostlyMistakesApplied { .. } => {
    self.game_phase = Some(GamePhase::ReadyToPlay);   // déjà écrit
}
```

Le second existe **depuis longtemps**, ainsi que le mouvement de trésorerie qui
l'accompagne. Le code écrit d'avance supposait bien que cet événement fermait la
séquence.

## L'agrégat fuit, et il faut le savoir

`Team` porte **tous ses champs en `pub`**, `treasury` comprise. R1 et le calcul
des pertes sont donc gardés par des méthodes… que rien n'oblige à emprunter.
C'est le même constat que pour `MatchReportPreMatch` (spec de la Haine, phase 6),
et la même conclusion : cette fonctionnalité hérite d'un état de fait sans
l'aggraver. Fermer les agrégats est un sujet en soi.

## Erreurs domaine

Aucune nouvelle. `expect_phase` rend déjà l'erreur de phase incorrecte, et c'est
la seule que cette fonctionnalité peut produire.

## Tests unitaires — un par règle

| Test | Règle |
|---|---|
| Les **six tranches × six jets** : 36 cas, l'incident attendu pour chacun | R2 |
| 99 kPo → `validate_dismissals_phase` rend `DismissalsPhaseValidated` | R1 |
| 100 kPo → rend `CostlyMistakesPhaseStarted` — **la borne exacte** | R1 |
| 197 kPo → tranche 100-199, aucun trou | R2 |
| Mineur : 1D3 = 1, 2, 3 → 10, 20, 30 kPo | R3 |
| Majeur à 345 → 170 ; à 300 → 150 ; à **347 → 170** (cas impair) | R3, R4 |
| Catastrophe à 560 avec 2D6 = (3,4) → perte 490, reste 70 | R3 |
| Crise évitée → perte nulle, trésorerie inchangée | R3 |
| `dice_needed` pour les quatre incidents | R3 |
| `incident_for` sous 100 kPo → `None`, sans panique | robustesse |
| `apply_costly_mistakes` hors phase → erreur, aucun événement | R6 |
| L'événement porte les dés tirés, tels quels | R8 |
| Une équipe à 30 kPo qui perdrait 50 → débit de 30, solde 0 | R9 (déjà testé) |

**Les 36 cas de la table ne sont pas du zèle.** C'est la seule règle du projet
dont une erreur ne se voit pas : un incident majeur là où il fallait un mineur
retire de l'argent sans que personne ne puisse le contester. Écrire la table deux
fois — dans le code et dans les tests — est ce qui la rend vérifiable.
