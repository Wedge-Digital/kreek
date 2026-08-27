# Éditeur de roster · Phase 6 : domaine

**Phase 5** : `05-use-cases.md`

## Les règles, récapitulées — dix-sept, en quatre familles

### Portée et accès

| | Règle |
|---|---|
| P1 | Seul un **admin d'espace** crée, modifie ou supprime |
| P2 | Un roster d'un autre espace est **introuvable**, pas interdit |
| P3 | La **résolution** par uid ne filtre pas par espace ; seule la **liste** filtre |

P3 mérite son motif : une équipe déjà créée doit résoudre son roster où qu'on la
regarde — dans un classement, un rapport, une page publique. Filtrer à la
résolution casserait l'affichage d'une équipe vue depuis un autre espace. Ce qui
se garde par l'espace, c'est **le choix**, pas la lecture.

### Verrou d'usage

| | Règle |
|---|---|
| U1 | Un roster joué par au moins une équipe ne se modifie ni ne se supprime |
| U2 | Le compte est **re-vérifié à l'écriture**, jamais celui de l'écran |
| U3 | Un port d'usage **indisponible refuse** l'opération |
| U4 | Les tiers de compétition ne bloquent pas : ils suivent par app event |

U3 est la règle qu'on oublie : traiter une indisponibilité comme un zéro
laisserait supprimer un roster joué par cent équipes parce qu'une requête a
échoué. **Le doute ferme la porte.**

### Structure d'un roster

| | Règle |
|---|---|
| S1 | Au moins un poste |
| S2 | **Exactement un** poste journalier |
| S3 | Les noms de postes sont distincts |
| S4 | Chaque poste porte **au moins un accès primaire** ; le secondaire peut être vide |
| S5 | Chaque poste porte une espèce et un rôle |
| S6 | Une limite croisée vise des postes **existants du roster** |
| S7 | Le tier appartient à une liste fermée de trois valeurs |

### Identité et bornes

| | Règle |
|---|---|
| I1 | Les uid sont **engendrés**, jamais dérivés d'un nom |
| I2 | Un poste qui subsiste garde son uid |
| I3 | Bornes du LRB : M 1–9, F 1–8, AG et CP 1–6, AR 3–11 |
| I4 | La **Capacité de Passe est obligatoire** pour un poste de roster |
| I5 | Nom, prix, quantité, relance : les bornes des value objects existants |

---

## La ligne : ce que le domaine sait juger, et ce qu'il ne peut pas

Contrairement aux documents de configuration rencontrés jusqu'ici — les
paramètres de compétition, le relevé de trésorerie — **celui-ci a de vrais
invariants**. Un roster mal formé n'est pas un affichage bancal, c'est une
équipe qu'on ne pourra pas construire.

Deux familles de contrôles, qui n'ont pas la même nature :

| Contrôle | Couche | Pourquoi |
|---|---|---|
| S1 à S7, I3 à I5 | **domaine** | le roster se juge **seul** |
| « cette compétence existe », « ce mot-clef existe », « ce staff existe », « cette règle spéciale existe », « cette catégorie existe » | **use case** | il faut le corpus |

Le domaine ne peut pas vérifier une existence sans connaître
`IReferenceRepository` — ce qui lui ferait connaître un port, que le `CLAUDE.md`
lui interdit.

**Le domaine garantit qu'un roster se tient debout tout seul. Le use case
garantit qu'il ne référence rien d'inexistant**, en réutilisant les cinq
contrôles de `check_consistency` déjà écrits pour le corpus.

## `CustomRoster` — la forme

```rust
// shared_kernel/bloodbowl/roster.rs ? non — references/domain/custom_roster.rs
pub struct CustomRoster {
    uid: RosterUid,
    space_id: SpaceId,
    name: RosterName,
    tier: RosterTier,
    reroll_cost: RerollBasePrice,
    special_rules: Vec<SpecialRuleUid>,
    allowed_staff: Vec<StaffUid>,
    cross_limits: Vec<CrossLimit>,
    positions: Vec<RosterPosition>,
}

pub struct RosterPosition {
    uid: PositionUid,
    name: PlayerName,
    cost: PlayerPrice,
    stats: StatLine,
    max_quantity: PlayerMaxQuantity,
    is_journeyman: bool,
    skills: Vec<SkillUid>,
    primary_access: Vec<SkillCategoryUid>,    // ≥ 1
    secondary_access: Vec<SkillCategoryUid>,  // peut être vide
    keywords: Vec<KeywordUid>,                // ≥ 1 espèce, ≥ 1 rôle
}
```

**Champs privés**, comme `RankingGroupConfig` l'a appris : un smart constructor
sur des champs publics est du théâtre, on l'évite par un littéral et
`Deserialize` l'évite tout seul.

`CustomRoster` **n'est pas désérialisé depuis le web** — la commande l'est, et
elle porte déjà ses value objects (phase 4). Il n'a donc pas besoin du
`#[serde(try_from)]` de `RankingGroupConfig`. Il **est** sérialisé, en revanche,
puisqu'il se stocke : `to_reference_team()` produit le `Team` du corpus, et
c'est ce `Team` qui part en JSONB.

### Le constructeur prend une structure, pas dix arguments

```rust
pub struct CustomRosterDraft {
    pub uid: RosterUid,
    pub space_id: SpaceId,
    pub name: RosterName,
    pub tier: RosterTier,
    pub reroll_cost: RerollBasePrice,
    pub special_rules: Vec<SpecialRuleUid>,
    pub allowed_staff: Vec<StaffUid>,
    pub cross_limits: Vec<CrossLimitDraft>,
    pub positions: Vec<RosterPositionDraft>,
}

impl CustomRoster {
    pub fn try_new(draft: CustomRosterDraft) -> Result<Self, DomainError>;
}
```

**Dix arguments positionnels dont quatre `Vec`**, c'est la garantie qu'un jour
`special_rules` et `allowed_staff` s'inversent — deux `Vec<String>` voisins, et
le compilateur ne bronche pas. Une structure nomme chaque place.

### Ce que `try_new` refuse

```rust
pub enum DomainError {
    EmptyRoster,                                        // S1
    NoJourneymanPosition,                               // S2
    SeveralJourneymanPositions { count: usize },        // S2
    DuplicatePositionName { name: String },             // S3
    PositionWithoutPrimaryAccess { position: String },  // S4
    PositionWithoutSpecies { position: String },        // S5
    PositionWithoutRole { position: String },           // S5
    CrossLimitTargetsUnknownPosition { uid: String },   // S6
}
```

**Chaque variante nomme le poste fautif.** Sur un roster à huit postes, « un
poste n'a pas d'accès primaire » envoie chercher ; « le Kroxigor n'a pas d'accès
primaire » se corrige.

S2 se scinde en deux variantes plutôt que d'être un `WrongJourneymanCount` :
zéro et trois ne se corrigent pas du même geste, et le message doit le dire.

## Les value objects

### Six nouveaux — les caractéristiques

Aucune borne n'existait : le corpus est un fichier de confiance, et cet écran
est le premier où un humain tape une caractéristique.

```rust
#[nutype(validate(greater_or_equal = 1, less_or_equal = 9),  …)] pub struct Movement(u8);
#[nutype(validate(greater_or_equal = 1, less_or_equal = 8),  …)] pub struct Strength(u8);
#[nutype(validate(greater_or_equal = 1, less_or_equal = 6),  …)] pub struct AgilityTarget(u8);
#[nutype(validate(greater_or_equal = 1, less_or_equal = 6),  …)] pub struct PassingTarget(u8);
#[nutype(validate(greater_or_equal = 3, less_or_equal = 11), …)] pub struct ArmourTarget(u8);

#[derive(…)] pub enum RosterTier { One, Two, Three }
```

**Le piège du tableau du LRB, redit ici parce qu'il se relit à l'implémentation** :
« Maximum » y veut dire *le meilleur*, pas *le plus grand*. Le meilleur AG est
`1+`, numériquement le plus petit ; la meilleure armure est `11+`, numériquement
la plus grande. Les bornes ci-dessus sont **numériques** — les écrire ainsi
plutôt que de recopier le tableau évite d'en inverser deux sur cinq.

### Cinq déplacés depuis `team_creation`

`RosterName`, `PlayerName`, `PlayerPrice`, `PlayerMaxQuantity`,
`RerollBasePrice`, `CrossLimitCount` montent dans
`shared_kernel::bloodbowl::roster`.

**Déplacement par copier-coller** (règle 5 du `CLAUDE.md`), `team_creation`
suivant par un `use`.

**Pourquoi déplacer plutôt que redéclarer** : deux définitions de « ce qu'est un
nom de poste valide » dérivent. Un roster accepté à l'écriture et refusé à la
lecture, c'est le roster escamoté sans un mot de la carte 438.

`RosterName` existe **aussi** dans `teams/domain/value_objects.rs`. Celui-là
n'est pas le même — il nomme le roster d'une équipe, pas un roster de référence
— et il reste où il est. À vérifier au moment du déplacement plutôt qu'à
supposer.

## `to_reference_team()` — la conversion vers le format de stockage

```rust
impl CustomRoster {
    /// Le `Team` du corpus, tel qu'il sera stocké en JSONB et servi par
    /// `find_team_by_uid`. Total : un roster valide produit toujours un `Team`.
    pub fn to_reference_team(&self) -> Team;
}
```

**Total, jamais faillible.** Un `CustomRoster` construit est valide par
construction ; sa conversion vers un type moins strict ne peut pas échouer. Une
signature qui rendrait `Result` obligerait chaque appelant à traiter un cas qui
n'arrive pas.

Le sens inverse n'existe pas : on ne reconstruit pas un `CustomRoster` depuis un
`Team`. Pour l'édition, c'est **la base** qu'on relit, et le `Team` stocké qui
alimente le formulaire — pas un aller-retour de types.

## Tests

### `try_new` — un par règle

| Test | Règle |
|---|---|
| `refuse_un_roster_sans_poste` | S1 |
| `refuse_un_roster_sans_journalier` | S2 |
| `refuse_un_roster_a_deux_journaliers` | S2 |
| `refuse_deux_postes_de_meme_nom` | S3 |
| `refuse_un_poste_sans_acces_primaire` | S4 |
| `accepte_un_poste_sans_acces_secondaire` | S4 — le cas passant qu'on oublie |
| `refuse_un_poste_sans_espece` | S5 |
| `refuse_un_poste_sans_role` | S5 |
| `refuse_une_limite_croisee_vers_un_poste_inconnu` | S6 |
| `accepte_un_roster_minimal` | un seul poste, journalier, une espèce, un rôle |

`accepte_un_poste_sans_acces_secondaire` compte autant que les refus : une
validation trop stricte se découvre en production, quand un ligueur ne peut plus
enregistrer un poste que le règlement autorise.

### Les bornes

| Test | Règle |
|---|---|
| `les_bornes_de_caracteristiques_suivent_le_lrb` | I3 — les cinq, aux deux extrémités |
| `une_force_de_9_est_refusee` | la borne que je m'étais trompé à poser à 7 |
| `une_armure_de_2_est_refusee` | AR commence à 3, pas à 1 |

### La conversion

| Test | Ce qu'il prouve |
|---|---|
| `to_reference_team_produit_les_uid_prefixes` | I1 — `CUSTOM_…` et `…__…` |
| `to_reference_team_conserve_les_limites_croisees` | S6 survit à la conversion |

## Ce que la phase n'ajoute pas

- **Aucun agrégat événementiel.** `CustomRoster` n'a pas d'historique : il
  s'écrit, se relit, se remplace. Le seul événement de la fonctionnalité est
  émis à la **suppression**, par le use case, et il ne décrit pas une transition
  d'état interne (phase 5).
- **Aucune méthode de mutation.** Modifier un roster, c'est en construire un
  neuf avec le même uid — un `try_new` de plus, pas un `set_*`. Les invariants
  se vérifient alors en bloc, ce qu'une suite de mutateurs ne garantirait pas.
