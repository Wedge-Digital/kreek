# Phase 6 — Domaine — team-detail

## Récapitulatif exhaustif des règles métier (validé par l'utilisateur)

**Périmètre et accès**
1. Disponible uniquement quand l'équipe est en état « Prête à jouer ».
2. Seuls les joueurs `membership = Active` sont éditables.

**Validation des champs**
3. Numéro de maillot : `1..99` (VO existant `JerseyVo`, resserré).
4. Nom du joueur (`PersonalName`, nouveau VO) : `trim`, 1-50 caractères,
   `^[\p{L}0-9 '-]+$`. Absence de nom = `Option::None`, jamais une chaîne
   vide validée par le VO.
5. Un champ numéro vidé dans le formulaire retire le maillot
   (`jersey → None`) — symétrique à `personal_name`.
6. `display_order` (nouveau VO) : aucune contrainte au-delà du typage —
   assigné par index de soumission, jamais saisi par l'utilisateur.

**Unicité et cohérence du batch**
7. Unicité du numéro de maillot **et** du display_order vérifiée au niveau
   **use case**, pas domaine — `Player` est individuellement event-sourcé,
   pas d'agrégat `Roster` englobant.
8. Un joueur `Dismissed` n'entre jamais dans le calcul d'unicité — ni son
   numéro ni son display_order ne bloquent un joueur actif. Déjà garanti par
   construction : le use case ne charge que les joueurs `Active`.
9. L'unicité (numéro **et** display_order) est vérifiée sur l'**état
   résultant complet de l'effectif actif** — batch soumis fusionné aux
   joueurs actifs non soumis (valeurs inchangées) — pas seulement au sein du
   batch brut. Nécessaire pour couvrir un batch partiel (règle 11) sans
   risque de collision avec un joueur non touché.
10. Un `player_id` soumis inconnu ou non-`Active` rejette **tout le batch**
    (rien n'est persisté).
11. Un joueur actif absent du batch soumis est laissé inchangé (pas une
    erreur).
12. Un conflit d'unicité (numéro ou display_order) rejette **tout le batch**.
13. Le batch est atomique (tout ou rien) via `append_batch`.

**Émission d'événements**
14. Un champ inchangé ne produit aucun événement — diff fait par le use case
    avant d'appeler le domaine.

**Garde-fou domaine**
15. Chaque méthode de domaine (`rename`/`change_jersey`/`reorder`) vérifie
    elle-même `membership == Active`, retourne `DomainError::PlayerNotActive`
    sinon — indépendant du filtre déjà fait en amont par le use case.

**Concurrence**
16. `RepositoryError::ConcurrentWrite` (contrainte unique
    `players_events_player_version`) est propagé tel quel par le use case ;
    le controller le traduit en `rosterEditSaveFailed` avec un message
    convivial.

## Value objects

```rust
// players/domain/value_objects.rs

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = r"^[\p{L}0-9 '-]+$"),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct PersonalName(String);

#[nutype(
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)
)]
pub struct DisplayOrder(u32);
```

`JerseyVo` (existant, ligne 40) : `validate(less_or_equal = 999)` devient
`validate(greater_or_equal = 1, less_or_equal = 99)`.

## Agrégat `Player` — nouveaux champs

```rust
pub struct Player {
    // ... champs existants inchangés ...
    pub personal_name: Option<PersonalName>, // nouveau
    pub jersey: Option<JerseyVo>,            // existant, inchangé
    pub display_order: Option<DisplayOrder>, // nouveau
}
```

## Événements domaine (nouvelles variantes)

```rust
// players/domain/events.rs
PlayerRenamed {
    player_id: PlayerId,
    team_id: TeamId,
    personal_name: Option<PersonalName>,
},
PlayerJerseyChanged {
    player_id: PlayerId,
    team_id: TeamId,
    jersey: Option<JerseyVo>,
},
PlayerReordered {
    player_id: PlayerId,
    team_id: TeamId,
    display_order: DisplayOrder,
},
```

Noms en termes de faits domaine (pas d'origine externe), conforme à la
convention CLAUDE.md. `apply()` gagne une branche par variante, folding le
nouveau champ correspondant sur l'agrégat replayé.

## Méthodes de domaine

```rust
impl Player {
    pub fn rename(&self, personal_name: Option<PersonalName>) -> Result<PlayerDomainEvent, DomainError> {
        self.guard_active()?;
        Ok(PlayerDomainEvent::PlayerRenamed {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            personal_name,
        })
    }

    pub fn change_jersey(&self, jersey: Option<JerseyVo>) -> Result<PlayerDomainEvent, DomainError> {
        self.guard_active()?;
        Ok(PlayerDomainEvent::PlayerJerseyChanged {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            jersey,
        })
    }

    pub fn reorder(&self, display_order: DisplayOrder) -> Result<PlayerDomainEvent, DomainError> {
        self.guard_active()?;
        Ok(PlayerDomainEvent::PlayerReordered {
            player_id: self.id.clone(),
            team_id: self.team_id.clone(),
            display_order,
        })
    }

    fn guard_active(&self) -> Result<(), DomainError> {
        if self.membership != RosterMembership::Active {
            return Err(DomainError::PlayerNotActive);
        }
        Ok(())
    }
}
```

Aucune méthode ne mute `self` en place (cohérent avec `increase_stat` déjà
existant) : elle calcule et retourne l'événement, le use case l'append avec
la version calculée localement.

## `DomainError` — nouvelle variante

```rust
// players/domain/error.rs
PlayerNotActive, // "joueur non actif"
```

## Tests prévus (un par règle métier)

**Domaine (`player.rs`, module `tests`)**
- `rename_produces_player_renamed_event_with_new_name` (règle 4)
- `rename_allows_clearing_to_none` (règle 4)
- `change_jersey_produces_player_jersey_changed_event` (règle 3)
- `change_jersey_allows_clearing_to_none` (règle 5)
- `reorder_produces_player_reordered_event` (règle 6)
- `rename_change_jersey_reorder_reject_dismissed_player` (règle 15,
  `DomainError::PlayerNotActive` sur les trois méthodes)

**Value objects**
- `personal_name_rejects_over_50_chars` (règle 4)
- `personal_name_allows_apostrophe` (règle 4)
- `personal_name_rejects_empty_string` (règle 4 — l'absence passe par
  `Option::None`, jamais par le VO)
- `jersey_vo_rejects_zero_and_above_99` (règle 3)

**Use case (`update_roster_use_case.rs`)**
- `update_roster_rejects_unknown_or_inactive_player_and_persists_nothing`
  (règle 10)
- `update_roster_rejects_duplicate_jersey_against_untouched_active_player`
  (règle 9 — collision batch/hors-batch, pas seulement intra-batch)
- `update_roster_rejects_duplicate_display_order_against_untouched_active_player`
  (règle 9)
- `update_roster_ignores_dismissed_player_when_checking_uniqueness`
  (règle 8)
- `update_roster_only_emits_events_for_changed_fields` (règle 14)
- `update_roster_leaves_players_absent_from_batch_untouched` (règle 11)
- `update_roster_propagates_concurrent_write_as_is` (règle 16)
