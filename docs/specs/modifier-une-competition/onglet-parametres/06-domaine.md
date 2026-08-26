# Onglet Paramètres · Phase 6 : domaine

**Phase 5** : `05-use-cases.md`

## Les règles métier — récapitulatif validé

⊕ nouvelle · ⊙ existante, rappelée pour que la liste se lise seule

### Écriture des réglages

| | Règle |
|---|---|
| ⊕ | Toute écriture relit le document JSONB, remplace la seule part éditée, réécrit le tout |
| ⊕ | Un document absent est une erreur, jamais un défaut reconstruit |
| ⊕ | Administrateurs, `invited_coaches`, `registration_deadline`, `max_participants`, calendrier et phase finale traversent sans être touchés |
| ⊙ | Le nom de compétition est unique dans l'espace |
| ⊕ | Un mode d'accès ou une validation inconnus sont un refus, jamais un repli sur le défaut |

### Poules

| | Règle |
|---|---|
| ⊕ | Deux poules du même nom sont refusées |
| ⊕ | Deux poules du même identifiant sont refusées |
| ⊕ | Retirer toutes les poules est autorisé — les équipes repassent en attente d'affectation |
| ⊕ | Retirer une poule supprime sa ligne et désaffecte ses équipes, atomiquement avec la structure |
| ⊙ | Retirer une poule ne touche ni résultat ni point |

### Tiers

| | Règle |
|---|---|
| ⊕ | Nom, budget, XP de départ et rosters ne sont pas modifiables ici — un écart est un refus |
| ⊕ | Le nombre de tiers ne change pas |
| ⊕ | Un tier sans aucun coup de pouce est valide |
| ⊙ | Un roster n'appartient qu'à un seul tier |

### Barème et classement

| | Règle |
|---|---|
| ⊙ | Liste de départage non vide, sans doublon, avec au moins un critère actif |
| ⊙ | Les codes de départage existent au catalogue ; l'exhaustivité n'est pas exigée |
| ⊕ | Le barème enregistré s'applique immédiatement, sans gel ni annonce |
| ⊕ | Le rejeu reconstruit toute la saison depuis zéro, dans l'ordre du cumul |
| ⊕ | Les statistiques d'un match sont la différence de deux lignes cumulatives |
| ⊕ | Le résultat V/N/D se redérive des scores, jamais lu |
| ⊕ | Le rejeu est idempotent |
| ⊕ | Un rejeu en échec ne défait pas l'enregistrement du barème |
| ⊕ | Un écart de score aberrant échoue le recalcul, il ne se replie pas |

### Accès

| | Règle |
|---|---|
| ⊙ | Admin d'espace ou admin de compétition ouvre les cinq panneaux ; le contrôle est à l'entrée, GET compris |

---

## Ce que cette page ajoute au domaine

**Aucun agrégat.** `CompetitionRules`, `CompetitionStructure` et
`CompetitionInvitations` sont des documents de configuration : pas de machine à
états, pas d'événements, pas d'identité propre. Cette page ne leur en donne pas.

Elle ajoute **deux gardiens d'invariant** dans `competitions` et **une fonction
pure** dans `ranking`. C'est tout, et c'est normal : l'essentiel des règles de
cet onglet est déjà tenu par les bornes des nutypes.

### La correction que porte cette phase

La phase 5 plaçait deux règles dans les use cases. Elles répondent à « est-ce
autorisé ? » — le `CLAUDE.md` les réserve au domaine. Elles descendent :

| Règle | Phase 5 | Phase 6 |
|---|---|---|
| Doublon de nom de poule | use case | `RankingGroupConfig::try_new()` |
| Champs de tier non modifiables | `ensure_only_inducements_changed` en use case | `CompetitionRules::with_inducements_from()` |

Et une troisième les rejoint, qui n'attendait que l'occasion :
`ensure_roster_unicity` — « un roster n'appartient qu'à un seul tier » — est une
règle purement domaine, écrite en fonction privée dans
`save_competition_rules.rs:53`. Puisqu'on ouvre un `impl` sur `CompetitionRules`,
elle y va.

---

## `RankingGroupConfig` — encapsulé pour de bon

### Pourquoi, et pas seulement un `try_new`

Le type s'instancie aujourd'hui par littéral :

```rust
pub struct RankingGroupConfig {
    pub use_ranking_groups: UseRankingGroups,
    #[serde(default)] pub dispatch_type: DispatchType,
    pub ranking_groups: Vec<RankingGroup>,
}
```

Un smart constructor posé sur des champs publics ne garde rien : on l'évite en
écrivant le littéral, et `Deserialize` l'évite tout seul. Le projet a déjà
rencontré le problème et l'a tranché sur `TiebreakConfig`, dont le commentaire
est la meilleure justification possible :

> `#[serde(try_from = ...)]` est indispensable : sans lui, un `Deserialize` nu
> reconstruirait le newtype sans passer par `try_new`, et n'importe quel payload
> JSON contournerait les invariants.

C'est exactement notre cas : les réglages arrivent en JSON depuis le navigateur
(phase 4).

### La forme

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "RankingGroupConfigData")]
pub struct RankingGroupConfig {
    use_ranking_groups: UseRankingGroups,
    dispatch_type: DispatchType,
    ranking_groups: Vec<RankingGroup>,
}

/// Miroir sérialisable — la seule porte d'entrée depuis JSON, et elle passe
/// par `try_new`. Privé au module : rien d'autre n'a de raison de le nommer.
#[derive(Deserialize)]
struct RankingGroupConfigData {
    use_ranking_groups: UseRankingGroups,
    #[serde(default)] dispatch_type: DispatchType,
    ranking_groups: Vec<RankingGroup>,
}

impl TryFrom<RankingGroupConfigData> for RankingGroupConfig {
    type Error = DomainError;
    fn try_from(d: RankingGroupConfigData) -> Result<Self, Self::Error> {
        Self::try_new(d.use_ranking_groups, d.dispatch_type, d.ranking_groups)
    }
}

impl RankingGroupConfig {
    /// Refuse deux poules de même nom et deux poules de même identifiant.
    /// **N'exige rien d'autre** : une liste vide est valide, avec ou sans
    /// `use_ranking_groups` — retirer toutes les poules est un usage prévu.
    pub fn try_new(
        use_ranking_groups: UseRankingGroups,
        dispatch_type: DispatchType,
        ranking_groups: Vec<RankingGroup>,
    ) -> Result<Self, DomainError>;

    pub fn use_ranking_groups(&self) -> bool;
    pub fn dispatch_type(&self) -> &DispatchType;
    pub fn groups(&self) -> &[RankingGroup];
}
```

**`Serialize` reste dérivé sur les champs privés** — la sérialisation ne
contourne aucun invariant, elle écrit un état déjà validé. Seule la
désérialisation avait besoin d'une porte.

**Aucune référence mutable ne sort** : `groups()` rend `&[RankingGroup]`, jamais
`&mut Vec`. Modifier les poules passe par un `try_new` complet — ce que fait
déjà le use case, qui construit la liste avant de l'enregistrer.

### Le coût, mesuré

Neuf sites de lecture dans six fichiers, tous de la forme
`s.ranking_group.use_ranking_groups.0` ou `.ranking_groups` :

```
new_competition_phase_5.rs:166,167   competition_widget.rs:353,355
admin_page.rs:161                     summary_tab.rs:282,283
groups_widgets.rs:160                 (+ une chaîne JSON de test)
```

Neuf lignes à passer par `use_ranking_groups()` et `groups()`. La chaîne de test
de `competition_notifications.rs:164` porte `"ranking_groups":[]` et reste
valide — la liste vide est autorisée.

---

## `CompetitionRules` — deux méthodes

```rust
impl CompetitionRules {
    /// Le barème change, les tiers sont conservés tels quels.
    pub fn with_ranking_rules(&self, ranking_rules: RankingRules) -> Self;

    /// Les coups de pouce et les stars changent, **rien d'autre**.
    ///
    /// Refuse un nombre de tiers différent, et tout écart de `name`, `budget`,
    /// `starting_xp` ou `rosters`. Le refus est délibéré : accepter la valeur
    /// reçue rendrait modifiable par requête forgée ce que l'écran n'ouvre pas,
    /// et corriger en silence ferait croire à un enregistrement qui n'a pas eu
    /// lieu.
    pub fn with_inducements_from(&self, tiers: Vec<TierRule>) -> Result<Self, DomainError>;

    /// Un roster n'appartient qu'à un seul tier.
    /// Déplacée depuis `save_competition_rules.rs:53`, inchangée.
    pub fn ensure_roster_unicity(&self) -> Result<(), DomainError>;
}
```

`with_*` rendent un nouveau `CompetitionRules` plutôt que de muter : le type est
`Clone`, il n'a pas d'identité, et une méthode qui rend le document complet dit
mieux que la mutation ce que le use case va enregistrer — **le tout**, jamais
une part.

**`with_inducements_from` ne valide pas les uid** des coups de pouce. Un uid
inconnu du corpus n'est pas une violation d'invariant : le corpus vit hors du
dépôt et peut changer sous les pieds d'une compétition. Le VM les affiche tels
quels plutôt que de les faire disparaître (phase 4), et c'est la bonne réponse à
ce cas.

### Nouvelles variantes de `DomainError`

```rust
pub enum DomainError {
    EmptyTiebreakConfig,                              // existante
    NoActiveTiebreaker,                               // existante
    DuplicateTiebreakCode { code: String },           // existante
    DuplicatePoolName { name: String },               // ⊕
    DuplicatePoolId { id: String },                   // ⊕
    TierCountChanged { before: usize, after: usize }, // ⊕
    ImmutableTierField { tier: String, field: &'static str },  // ⊕
    RosterInMultipleTiers { roster: String, tiers: (String, String) },  // déplacée
}
```

Le `Display` est écrit à la main, comme les trois existantes — le projet
n'utilise pas `thiserror` dans ce BC, et le message sert directement de corps de
réponse 422.

`RosterInMultipleTiers` **quitte `SaveCompetitionRulesError`** pour
`DomainError`. Le use case existant la reconvertit : sa variante reste, sa
source change.

---

## `ranking` — la fonction inverse

```rust
impl RankingLine {
    /// L'inverse exact de `record_match` : retrouve les statistiques du match
    /// qui a fait passer les cumuls de `previous` à `current`.
    pub fn stats_between(
        previous: Option<&CumulativeTotals>,
        current: &RankingLine,
    ) -> Result<MatchStats, DomainError>;
}
```

Elle rend un `Result` et non un `MatchStats` nu : `td_for` est un cumul `u32`,
`MatchScore` un `u8`. Un écart supérieur à 255 ne peut venir que de lignes
corrompues, et **doit arrêter le rejeu** — un `as u8` replierait la valeur en un
score plausible (phase 3).

**Dans le domaine et non dans le use case** parce qu'elle est l'inverse de
`record_match` : les deux doivent être modifiées ensemble. Un champ ajouté à
`MatchStats` sans être ajouté ici produirait un recalcul qui perd cette
statistique — silencieusement, puisque la ligne resterait bien formée.

---

## Tests unitaires — un par règle

### `RankingGroupConfig`

| Test | Règle |
|---|---|
| `try_new_refuse_deux_poules_de_meme_nom` | doublon de nom |
| `try_new_refuse_deux_poules_de_meme_id` | doublon d'identifiant |
| `try_new_accepte_une_liste_vide` | retirer toutes les poules |
| `try_new_accepte_une_liste_vide_avec_le_drapeau_actif` | le drapeau ne commande pas la liste |
| `deserialize_passe_par_try_new` | **le test qui garde l'encapsulation** : un JSON à deux poules homonymes est refusé |

Le dernier est celui qui compte. Sans lui, retirer le `#[serde(try_from)]` au
détour d'un refactor ne casserait rien de visible, et la porte se rouvrirait.

### `CompetitionRules`

| Test | Règle |
|---|---|
| `with_inducements_from_accepte_un_changement_de_coups_de_pouce` | cas nominal |
| `with_inducements_from_accepte_un_tier_sans_coup_de_pouce` | liste vide valide |
| `with_inducements_from_refuse_un_budget_modifie` | champ non modifiable |
| `with_inducements_from_refuse_un_nom_modifie` | idem |
| `with_inducements_from_refuse_un_xp_modifie` | idem |
| `with_inducements_from_refuse_des_rosters_modifies` | idem |
| `with_inducements_from_refuse_un_tier_ajoute` | nombre de tiers |
| `with_inducements_from_refuse_un_tier_retire` | nombre de tiers |
| `ensure_roster_unicity_*` | déplacés depuis le use case, inchangés |

Les quatre refus sont écrits séparément et non en boucle sur les champs :
l'erreur porte le nom du champ, et c'est ce nom qu'on veut voir échouer
nommément quand il se trompe.

### `RankingLine::stats_between`

| Test | Règle |
|---|---|
| `stats_between_est_l_inverse_de_record_match` | **propriété** : `stats_between(record_match(p, ctx, s, r)) == s` |
| `stats_between_sur_la_premiere_ligne` | `previous = None`, les cumuls partent de zéro |
| `stats_between_echoue_sur_un_ecart_aberrant` | `u8::try_from`, pas de repliement |
| `rejeu_idempotent_a_bareme_inchange` | le filet du recalcul |

Le premier est le seul qui protège vraiment le couple : écrit sur plusieurs jeux
de statistiques, il échoue dès qu'un champ est ajouté d'un côté sans l'autre.
