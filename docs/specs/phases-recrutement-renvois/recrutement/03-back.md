# Recrutement — Phase 3 : architecture back

**Entrée** : `02-front.md` validé.

Ce document couvre l'organisation back **commune aux deux pages** — brouillon,
trésorerie, ports — puis ce qui est propre au recrutement. `renvois/03-back.md` ne
consigne que ses écarts.

## 1. Le brouillon est un agrégat du domaine

Il porte des invariants forts : plafond de 16, quota par poste, limites croisées,
trésorerie suffisante. C'est donc un agrégat, pas un objet applicatif.

### La tension « le domaine n'appelle pas de port » se résout par hydratation

Le brouillon **porte** les données dont ses gardes ont besoin, exactement comme
`RosterSelectedTeam` porte son `roster: Roster` (avec `player_definitions`,
`allowed_staff`, `cross_limits`). Le use case l'hydrate depuis les ports ; ensuite
toutes les vérifications sont **pures, synchrones, sans port** — comme
`check_cross_limits`, `check_player_budget` et `check_staff_limit`
(`team_creation/domain/team_roster_selected.rs`).

```
Use case ──(ports)──► catalogue roster + effectif courant + trésorerie
                              │
                              ▼
                    RecruitmentDraft::hydrate(...)
                              │
                              ▼
              draft.add_player(line_id)  ← gardes pures
```

### Ce que l'agrégat contient

| Champ | Origine |
|---|---|
| `team_id`, `version` | brouillon persisté |
| `lines: Vec<DraftLine>` | brouillon persisté — **seul état muté** |
| `catalog: RosterCatalog` | port `references`, hydraté |
| `squad: SquadSnapshot` | port `players`, hydraté |
| `treasury: Kpo` | agrégat `Team`, hydraté |

**Seul `lines` est sérialisé** dans la colonne `state`. Le reste est rechargé à chaque
hydratation — c'est ce qui garantit qu'un brouillon vieux de dix minutes est évalué
contre les prix et l'effectif d'aujourd'hui.

### Persistance

Une table **unique pour les deux phases**, discriminée par phase — les deux brouillons
ne coexistent jamais, puisque les phases sont séquentielles :

```sql
CREATE TABLE teams__phase_drafts (
    team_id    TEXT        NOT NULL,
    phase      TEXT        NOT NULL,        -- 'Recruitment' | 'Dismissals'
    space_id   TEXT        NOT NULL,
    state      JSONB       NOT NULL,
    version    INT         NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (team_id, phase)
);
```

Écriture gardée par la version :

```sql
UPDATE teams__phase_drafts
   SET state = $3, version = version + 1, updated_at = now()
 WHERE team_id = $1 AND phase = $2 AND version = $4
```

Zéro ligne affectée ⇒ `RepositoryError::ConcurrentWrite`. La course à la création se
traite par le conflit de clé primaire, ramené à la même erreur.

## 2. Trésorerie — grand livre en projection

### La méthode domaine décide

```rust
impl TeamDomainEvent {
    /// `match` exhaustif **sans joker** : ajouter une variante casse la
    /// compilation tant que son effet sur la trésorerie n'est pas tranché.
    fn treasury_movement(&self) -> Option<TreasuryMovement> { … }
}
```

C'est l'idiome déjà en place dans `apply()`, qui liste explicitement ses événements
sans effet (`PlayerRetiredTemporarily | PlayerReEngaged => {}`) plutôt que de les
balayer par `_ =>`.

### Le tag est dérivé, jamais saisi

`team_event_store` gagne une colonne `tags JSONB` avec index GIN, sur le modèle exact
d'`event_log`. À l'append, le tag `"treasury"` est posé **à partir du résultat de
`treasury_movement()`** — jamais à la main événement par événement, sous peine de
recréer l'endroit qu'on peut oublier.

### La projection

`teams__treasury_ledger (team_id, position, occurred_at, direction, amount_kpo,
reason, balance_after_kpo)`, mise à jour **dans la même transaction que l'append**.

Reconstruction complète par `WHERE tags @> '["treasury"]'`.

### Suppression de `refund_kpo`

Vérifié : à la création, `remaining_budget()` recalcule `budget − dépensé` depuis les
listes courantes (`team_roster_selected.rs:92-95`) et `remove_staff()` retire
simplement l'élément. **Aucun remboursement n'existe ni n'est nécessaire nulle part.**

`refund_kpo` disparaît donc de `StaffDismissed`, et avec lui le crédit de trésorerie
de `team.rs:477`.

## 3. Ports

Deux ports, tous deux consommés par les use cases, jamais par les handlers.

### `IRosterCatalogPort` → `references` (nouveau)

```rust
pub struct RosterCatalogDto {
    pub positions:        Vec<CatalogPositionDto>,  // uid, nom, stats, compétences,
                                                    // prix, max_quantity, is_journeyman
    pub cross_limits:     Vec<CrossLimitDto>,       // { max, position_uids }
    pub allowed_staff:    Vec<String>,
    pub staff_prices:     Vec<StaffPriceDto>,       // uid, prix, max_quantity
    pub reroll_base_cost: u32,
}
```

**Les limites croisées ne sont exposées nulle part aujourd'hui** — ni
`references::TeamDefinition`, ni le port de `team_creation`. Il faut les remonter
jusqu'ici, et au passage **unifier les deux schémas JSON incompatibles**
(`{max, in}` contre `{limit, limitedPlayerIds}`).

Ce port remplace `IRosterInfoPort`, qui devient un sous-ensemble.

### `IPlayerValuePort` → `players` (étendu, carte 250)

La carte 250 le crée pour la valeur d'équipe. Il faut y ajouter la ligne de roster de
chaque joueur, pour compter les effectifs par poste :

```rust
pub struct PlayerValueDto {
    pub player_id:                String,
    pub roster_line_id:           String,   // ← ajout
    pub value_kpo:                u32,
    pub available_for_next_match: bool,
}
```

**L'étendre, pas le doubler** : les deux features arrivent sur le même port et se
marcheraient dessus.

### `IPhaseDraftRepository` → persistance du brouillon

`load(team_id, phase)`, `save(draft, expected_version)`, `delete(team_id, phase)`.

## 4. Domain services

`use_cases/draft_hydration_service.rs` — construit `RecruitmentDraft` à partir des
DTOs des deux ports et de l'agrégat `Team`.

C'est l'application stricte de la règle « domain services pour données inter-BCs » :
aucun handler, aucun template ne voit un DTO de port.

## 5. Fichiers

### Domaine

| Fichier | Contenu |
|---|---|
| `domain/recruitment_draft.rs` | agrégat, gardes pures, `DraftLine` |
| `domain/treasury.rs` | `TreasuryMovement`, `MovementDirection`, `MovementReason` |
| `domain/team.rs` | `treasury_movement()`, retrait de `refund_kpo`, **méthodes de recrutement à créer** (`PlayerRecruited` n'est aujourd'hui jamais construit) |
| `domain/value_objects.rs` | `DraftLineId`, `DraftVersion`, `Jersey` |

### Use cases

`add_draft_player`, `remove_draft_player`, `add_draft_staff`, `remove_draft_staff`,
`draft_hydration_service`, et `validate_recruitment_phase_use_case` (existant, dont le
rôle s'élargit à l'application du lot).

### IO — web

| Fichier | Rôle |
|---|---|
| `io/web/recruitment.rs` | page hôte |
| `io/web/widgets/recruitment_catalog_widget.rs` | widget + les 2 POST d'ajout |
| `io/web/widgets/recruitment_cart_widget.rs` | widget + les 2 POST de retrait |
| `io/web/view_models.rs` | VMs suffixés `Vm`, `from_domain()` co-localisés |
| `io/web/builders.rs` | VMs dépendant des DTOs de port |
| `templates/recruitment.html` | page d'assemblage, deux conteneurs `hx-get` |
| `templates/widgets/recruitment-catalog.html` | + fragments de ligne |
| `templates/widgets/recruitment-cart.html` | |
| `templates/widgets/draft-error.html` | fragment d'erreur, y compris `ConcurrentWrite` |

### IO — persistance et infrastructure

`io/repository/phase_draft_repository.rs`,
`infrastructure/teams/roster_catalog_adapter.rs`, extension de
`infrastructure/teams/player_value_adapter.rs`.

### Migrations

`teams__phase_drafts`, `teams__treasury_ledger`, `team_event_store.tags` + index GIN.

## 6. Widgets existants — aucun réutilisable

`team_creation` a `cart_widget`, `player_table_widget` et `staff_table_widget` qui
répondent au même besoin. La règle 1 des conventions widgets interdit à `teams` de les
référencer : **à réécrire**. On reprend le pattern et la forme des VMs, pas le code.

## 7. Règles métier identifiées à cette étape

- **Le brouillon est évalué contre l'état du jour, pas celui de sa création.** Seules
  ses lignes sont persistées ; prix, effectif et trésorerie sont rechargés à chaque
  hydratation. Une ligne ajoutée hier peut donc devenir invalide — et le refus en bloc
  de la décision D5 s'applique à la validation.
- **Le numéro de maillot est attribué à l'application du lot**, pas à l'ajout au
  brouillon : deux lignes en attente ne peuvent pas réserver le même numéro, puisque
  le brouillon ne porte que des postes.
- **La trésorerie n'est vérifiée qu'en total.** Le brouillon compare la somme de ses
  lignes au solde courant, jamais ligne à ligne.

## 8. Points ouverts pour la phase 4

- Faut-il un `TeamDomainEvent` par ligne appliquée (un `PlayerRecruited` par joueur)
  ou un événement de lot ? Un événement par ligne garde l'event store lisible et
  réutilise l'existant ; un événement de lot rendrait la transaction plus simple à
  raisonner. Penche pour un par ligne.
- L'ordre d'application du lot compte-t-il ? Si la trésorerie est vérifiée en total en
  amont, non — mais il faut l'écrire.
