# Phase 4 — Contrats de données — team-detail

## DTO d'entrée

### Formulaire brut (handler uniquement)

```rust
// players/io/web/roster_edition_controller.rs
#[derive(Deserialize)]
pub struct RosterUpdateForm {
    pub player_id: Vec<String>,
    pub personal_name: Vec<String>, // "" si vide
    pub jersey: Vec<String>,        // "" si vide
}
```

**Émetteur** : navigateur — soumission du `<form>` du players-widget
(déclenchée par `hx-trigger="rosterEditSaveRequested from:body"`, cf.
02-front.md). **Consommateur** : `post_update_roster` (handler) — jamais vu
par le use case.

### Commande (`use_cases/commands.rs`, fichier existant BC `players`)

```rust
pub struct UpdateRosterCommand {
    pub team_id: TeamId,
    pub space_id: SpaceId,
    pub rows: Vec<RosterRowCommand>,
}

pub struct RosterRowCommand {
    pub player_id: PlayerId,
    pub personal_name: Option<PersonalName>, // None = pas de nom personnalisé
    pub jersey: Option<JerseyVo>,            // VO existant, resserré 1..99
    pub display_order: DisplayOrder,         // nouveau VO, Phase 6
}
```

**Émetteur** : `post_update_roster` (handler) — construit une ligne par
triplet `(player_id[i], personal_name[i], jersey[i])`, `display_order = i`.
Chaîne vide → `None` avant d'appeler le smart constructor (`personal_name`
comme `jersey` : c'est l'`Option` qui porte l'absence, pas le VO). Rejette la
requête (`AppError::BadRequest`) si les trois tableaux n'ont pas la même
longueur, ou si un smart constructor échoue (nom invalide, numéro hors
1..99). **Consommateur** : `update_roster_use_case::execute()`.

## DTO de sortie

Aucun nouveau template : `PlayerTableTemplate` (existant) sert à la fois au
GET et à la réponse du POST, succès et échec.

```rust
#[derive(Template)]
#[template(path = "player-table-fragment.html")]
pub struct PlayerTableTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub team_id: String,            // nouveau — construit l'URL du POST batch
    pub players: Vec<PlayerRowVm>,  // inchangé, aucun nouveau champ nécessaire
    pub save_error: Option<String>, // nouveau
}
```

`PlayerRowVm` ne change pas : `jersey: Option<i16>` et
`personal_name: String` (déjà présents) portent déjà tout le nécessaire pour
préremplir les inputs d'édition.

**Émetteurs** :
- `player_table_widget` (GET) — `save_error: None`, `players` lu depuis la
  projection (comportement inchangé).
- `post_update_roster` (POST, succès) — recharge depuis la DB à jour,
  `save_error: None`, header `HX-Trigger: rosterEditSaved`.
- `post_update_roster` (POST, échec métier — ex. doublon détecté en
  concurrence) — `save_error: Some(message)`, et surtout : **les valeurs de
  `players` viennent de la commande soumise (rejetée), pas de la DB**, pour
  ne pas faire perdre la saisie. Header `HX-Trigger: rosterEditSaveFailed`.

**Consommateur** : `player-table-fragment.html` — `save_error.is_some()`
détermine si la racine `.player-table` porte la classe `edit-mode` au premier
rendu (pas d'attente d'un toggle JS) et si le bandeau d'erreur s'affiche.

## DTO de port

Aucun — confirmé Phase 3, inchangé.

## Règles métier confirmées à cette étape

- `PersonalName` : même règle que `PositionNameVo` (`sanitize(trim)`,
  `validate(not_empty, len_char_max = 50, regex = r"^[\p{L}0-9 '-]+$")`),
  avec l'apostrophe ajoutée à la classe de caractères. L'absence de nom
  personnalisé est portée par `Option::None`, pas par une chaîne vide
  acceptée par le VO — même pattern que `jersey: Option<JerseyVo>` déjà en
  place.
