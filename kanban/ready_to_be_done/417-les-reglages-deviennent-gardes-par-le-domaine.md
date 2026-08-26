# Les réglages deviennent gardés par le domaine

**Épic :** E14 · **Ordre :** 1 · **Dépend de :** rien
**Conception :** `docs/specs/modifier-une-competition/onglet-parametres/06-domaine.md`

## Objectif

Poser dans le domaine les trois règles que l'onglet Paramètres fera respecter,
et fermer la porte par laquelle on les contournerait. Aucun écran, aucune route.

## Ce qui se joue

`RankingGroupConfig` s'instancie aujourd'hui par littéral, champs publics :

```rust
pub struct RankingGroupConfig {
    pub use_ranking_groups: UseRankingGroups,
    #[serde(default)] pub dispatch_type: DispatchType,
    pub ranking_groups: Vec<RankingGroup>,
}
```

Un smart constructor posé là-dessus ne garde rien — on l'évite par un littéral,
et `Deserialize` l'évite tout seul. Or les réglages arrivent en JSON depuis le
navigateur. Le projet a déjà tranché ce cas sur `TiebreakConfig`, dont le
commentaire est la justification :

> `#[serde(try_from = ...)]` est indispensable : sans lui, un `Deserialize` nu
> reconstruirait le newtype sans passer par `try_new`, et n'importe quel payload
> JSON contournerait les invariants.

## Conception

### `RankingGroupConfig` — champs privés, une seule porte

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "RankingGroupConfigData")]
pub struct RankingGroupConfig { /* les trois champs, privés */ }

#[derive(Deserialize)]
struct RankingGroupConfigData { /* miroir, privé au module */ }

impl TryFrom<RankingGroupConfigData> for RankingGroupConfig { … }

impl RankingGroupConfig {
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

`try_new` refuse **deux poules de même nom** et **deux poules de même
identifiant**. Rien d'autre : une liste vide est valide, avec ou sans le
drapeau — retirer toutes les poules est un usage prévu.

`groups()` rend `&[RankingGroup]`, jamais `&mut Vec` : aucune référence mutable
ne sort. `Serialize` reste dérivé — sérialiser ne contourne aucun invariant.

### `CompetitionRules` — trois méthodes

```rust
pub fn with_ranking_rules(&self, ranking_rules: RankingRules) -> Self;
pub fn with_inducements_from(&self, tiers: Vec<TierRule>) -> Result<Self, DomainError>;
pub fn ensure_roster_unicity(&self) -> Result<(), DomainError>;
```

`with_inducements_from` refuse un nombre de tiers différent, et tout écart de
`name`, `budget`, `starting_xp` ou `rosters`. **Un refus, pas une correction** :
accepter la valeur reçue rendrait modifiable par requête forgée ce que l'écran
n'ouvre pas.

Elle **ne valide pas les uid** des coups de pouce : le corpus vit hors du dépôt
et peut changer sous les pieds d'une compétition.

`ensure_roster_unicity` est **déplacée** depuis `save_competition_rules.rs:53`
— copier-coller, pas réécriture (règle 5 du `CLAUDE.md`).

### `DomainError` — cinq variantes

```rust
DuplicatePoolName { name: String },
DuplicatePoolId { id: String },
TierCountChanged { before: usize, after: usize },
ImmutableTierField { tier: String, field: &'static str },
RosterInMultipleTiers { roster: String, tiers: (String, String) },   // déplacée
```

`Display` écrit à la main, comme les trois existantes — le BC n'utilise pas
`thiserror`, et le message sert de corps de réponse 422.

`RosterInMultipleTiers` quitte `SaveCompetitionRulesError` pour `DomainError` ;
la variante du use case reste, sa source change.

## Les neuf sites de lecture à convertir

```
new_competition_phase_5.rs:166,167   competition_widget.rs:353,355
admin_page.rs:161                     summary_tab.rs:282,283
groups_widgets.rs:160
```

Tous de la forme `s.ranking_group.use_ranking_groups.0` ou `.ranking_groups` →
`use_ranking_groups()` et `groups()`.

## Le piège au déploiement

Une saison portant déjà deux poules homonymes **ne se désérialisera plus**. À
vérifier avant de livrer :

```sql
select season_id, name, count(*) from competition_groups
group by 1,2 having count(*) > 1;
```

C'est le seul endroit où cette carte peut casser des données en place.

## Tests

| Test | Règle |
|---|---|
| `try_new_refuse_deux_poules_de_meme_nom` | doublon de nom |
| `try_new_refuse_deux_poules_de_meme_id` | doublon d'identifiant |
| `try_new_accepte_une_liste_vide` | retirer toutes les poules |
| `try_new_accepte_une_liste_vide_avec_le_drapeau_actif` | le drapeau ne commande pas la liste |
| `deserialize_passe_par_try_new` | **garde l'encapsulation** : un JSON à deux homonymes est refusé |
| `with_inducements_from_accepte_un_changement_de_coups_de_pouce` | cas nominal |
| `with_inducements_from_accepte_un_tier_sans_coup_de_pouce` | liste vide valide |
| `with_inducements_from_refuse_un_budget_modifie` | champ non modifiable |
| `with_inducements_from_refuse_un_nom_modifie` | idem |
| `with_inducements_from_refuse_un_xp_modifie` | idem |
| `with_inducements_from_refuse_des_rosters_modifies` | idem |
| `with_inducements_from_refuse_un_tier_ajoute` | nombre de tiers |
| `with_inducements_from_refuse_un_tier_retire` | nombre de tiers |
| `ensure_roster_unicity_*` | déplacés, inchangés |

`deserialize_passe_par_try_new` est celui qui compte : sans lui, retirer le
`#[serde(try_from)]` au détour d'un refactor ne casserait rien de visible.

Les quatre refus de champ sont écrits séparément, pas en boucle : l'erreur porte
le nom du champ, et c'est ce nom qu'on veut voir échouer nommément.

## Checklist

- [ ] `RankingGroupConfig` encapsulé, `try_new`, `TryFrom`, trois accesseurs
- [ ] Les neuf sites de lecture convertis
- [ ] `CompetitionRules` : trois méthodes
- [ ] `ensure_roster_unicity` déplacée par copier-coller, use case adapté
- [ ] Cinq variantes de `DomainError` et leur `Display`
- [ ] Les 14 tests
- [ ] La requête de vérification passée sur la base de production
- [ ] `make lint && make test && make check-arch`
