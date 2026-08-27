# `CustomRoster`, le gardien des invariants

**Épic :** E10 · **Ordre :** 2 · **Dépend de :** 439
**Conception :** `docs/specs/roster-personnalise/editeur-de-roster/06-domaine.md`

## Objectif

Le type qui garantit qu'un roster **se tient debout tout seul**. Aucun écran,
aucune base, aucun port.

## La ligne, et de quel côté cette carte est

| Contrôle | Couche |
|---|---|
| un seul journalier, noms distincts, accès primaire, limite croisée valide, bornes | **domaine — cette carte** |
| « cette compétence existe », « ce mot-clef existe » | use case (carte 443) — il faut le corpus |

Le domaine ne peut pas vérifier une existence sans connaître un port, ce que le
`CLAUDE.md` lui interdit.

## Conception

```rust
// references/domain/custom_roster.rs — champs privés
pub struct CustomRoster { uid, space_id, name, tier, reroll_cost,
                          special_rules, allowed_staff, cross_limits, positions }

pub struct RosterPosition { uid, name, cost, stats, max_quantity, is_journeyman,
                            skills, primary_access, secondary_access, keywords }

impl CustomRoster {
    pub fn try_new(draft: CustomRosterDraft) -> Result<Self, DomainError>;
    pub fn to_reference_team(&self) -> Team;
    // accesseurs — aucune référence mutable ne sort
}
```

### Le constructeur prend une structure, pas dix arguments

`CustomRosterDraft` nomme chaque place. **Dix arguments positionnels dont quatre
`Vec`**, c'est la garantie qu'un jour `special_rules` et `allowed_staff`
s'inversent — deux `Vec<String>` voisins, et le compilateur ne bronche pas.

### Ce que `try_new` refuse

```rust
pub enum DomainError {
    EmptyRoster,
    NoJourneymanPosition,
    SeveralJourneymanPositions { count: usize },
    DuplicatePositionName { name: String },
    PositionWithoutPrimaryAccess { position: String },
    PositionWithoutSpecies { position: String },
    PositionWithoutRole { position: String },
    CrossLimitTargetsUnknownPosition { uid: String },
}
```

**Chaque variante nomme le poste fautif.** Sur un roster à huit postes, « un
poste n'a pas d'accès primaire » envoie chercher ; « le Kroxigor n'a pas d'accès
primaire » se corrige.

**Zéro et plusieurs journaliers sont deux variantes**, pas un
`WrongJourneymanCount` : ils ne se corrigent pas du même geste, et le message
doit le dire.

`Display` écrit à la main, comme les autres BCs — le projet n'utilise pas
`thiserror`, et le message sert de corps de réponse `422`.

### `to_reference_team()` est totale

```rust
pub fn to_reference_team(&self) -> Team;   // jamais Result
```

Un `CustomRoster` construit est valide **par construction** ; sa conversion vers
un type moins strict ne peut pas échouer. Rendre un `Result` obligerait chaque
appelant à traiter un cas qui n'arrive pas.

**Le sens inverse n'existe pas.** On ne reconstruit pas un `CustomRoster` depuis
un `Team` : pour l'édition, c'est le `Team` stocké qui alimente le formulaire.

### Aucune méthode de mutation

Modifier un roster, c'est en construire un neuf avec le même uid — un `try_new`
de plus, jamais une suite de `set_*`. Les invariants se vérifient alors **en
bloc**, ce qu'une suite de mutateurs ne garantirait pas : un roster passerait
par des états invalides entre deux appels.

## Tests

| Test | Règle |
|---|---|
| `refuse_un_roster_sans_poste` | S1 |
| `refuse_un_roster_sans_journalier` | S2 |
| `refuse_un_roster_a_deux_journaliers` | S2 |
| `refuse_deux_postes_de_meme_nom` | S3 |
| `refuse_un_poste_sans_acces_primaire` | S4 |
| **`accepte_un_poste_sans_acces_secondaire`** | S4 — le cas passant |
| `refuse_un_poste_sans_espece` | S5 |
| `refuse_un_poste_sans_role` | S5 |
| `refuse_une_limite_croisee_vers_un_poste_inconnu` | S6 |
| `accepte_un_roster_minimal` | un poste journalier, une espèce, un rôle |
| `to_reference_team_produit_les_uid_prefixes` | I1 |
| `to_reference_team_conserve_les_limites_croisees` | S6 survit à la conversion |

`accepte_un_poste_sans_acces_secondaire` compte autant que les refus : **une
validation trop stricte se découvre en production**, quand un ligueur ne peut
plus enregistrer un poste que le règlement autorise.

## Checklist

- [ ] `CustomRoster`, `RosterPosition`, `CustomRosterDraft`, champs privés
- [ ] `try_new` et les huit refus
- [ ] `to_reference_team`, totale
- [ ] `DomainError` et son `Display`
- [ ] Les douze tests
- [ ] `make lint && make test && make check-arch`
