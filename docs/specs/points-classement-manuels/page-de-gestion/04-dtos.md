# Points de classement manuels · Phase 4 : contrats de données

**Phase 3** : `03-back.md`

## Entrée

Une seule mutation porte un corps : l'attribution.

```rust
#[derive(Deserialize)]
pub struct AwardPointsForm {
    pub team_id: String,
    /// « bonus » ou « penalty » — jamais un signe saisi au clavier.
    pub direction: String,
    /// Toujours positif. Le handler compose avec `direction`.
    pub points: u32,
    #[serde(default)]
    pub reason: String,
}
```

*Émis par* le formulaire (`axum::Form` — trois champs plats, aucune liste)
· *consommé par* `post_manual_points`.

### Le sens et la valeur sont séparés, et c'est délibéré

La maquette pose une bascule « + Bonus » / « − Pénalité » plutôt qu'un champ
signé. Le DTO suit : `direction` d'un côté, `points` positif de l'autre.

**Un `-3` tapé au clavier se tape aussi bien par erreur qu'exprès.** Séparer le
sens de la valeur rend l'intention explicite dans la requête elle-même — et un
`direction` inconnu est un `400`, là où un signe accidentel serait accepté sans
un mot.

Le handler compose : `points_signed = if direction == "penalty" { -(p as i32) } else { p as i32 }`.

### `reason` est une `String` et non un `Option<String>`

Le formulaire envoie toujours le champ, vide ou non. Le handler le `trim`, et
c'est **la commande** qui porte l'`Option` — une chaîne vide n'est pas un motif.

La suppression n'a pas de corps : `DELETE …/manual-points/{point_id}`, la cible
dans le chemin (carte 416).

## La commande

```rust
pub struct AwardManualPointsCommand {
    pub season_id: SeasonId,
    pub competition_id: CompetitionId,
    pub team_id: TeamId,
    pub points: ManualPoints,
    pub reason: Option<ManualPointsReason>,
    pub awarded_by: CoachId,
}
```

### Deux value objects à créer

```rust
/// Signé — une pénalité est un point manuel négatif, pas une autre nature de
/// chose. Zéro est refusé : une ligne qui ne change rien n'a pas à exister.
#[nutype(
    validate(predicate = |n| *n != 0 && n.abs() <= 100),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct ManualPoints(i32);

/// Facultatif (règle 6), mais borné quand il est là.
#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 200, regex = TEXTE_SAISI),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct ManualPointsReason(String);
```

**`ManualPoints` est signé**, cohérent avec la colonne de la phase 3 : une
seule nature de chose, le signe porte le sens.

**Zéro est refusé.** Une ligne à zéro point n'ajoute rien au classement, occupe
une ligne du relevé et demande à être expliquée. Ce n'est pas un ajustement,
c'est du bruit.

**La borne à 100** est arbitraire et doit être dite comme telle : elle attrape
la faute de frappe — un `300` pour un `3` — sans prétendre à une règle du jeu.
Aucune ligue n'attribue cent points manuels à une équipe ; si l'une le fait, la
borne se desserre.

**`ManualPointsReason` utilise `TEXTE_SAISI`**, la constante unique de tout
texte saisi (`shared_kernel/identity/charset.rs`). Ne pas redéfinir un charset :
c'est exactement le défaut que la carte du charset unique a corrigé, onze
expressions dont neuf refusaient l'apostrophe.

**200 caractères et non 50** : un motif est une phrase — « Forfait des Griffons
d'Argent à la journée 3, rencontre non jouée » — là où un nom est un libellé.

## Sortie — la page de gestion

```rust
#[derive(Template)]
#[template(path = "widgets/manual-points-form.html")]
pub struct ManualPointsFormWidget {
    pub routes: AppRoutes,
    pub space_id: String, pub competition_id: String, pub season_id: String,
    pub teams: Vec<TeamOptionVm>,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "widgets/manual-points-list.html")]
pub struct ManualPointsListWidget {
    pub routes: AppRoutes,
    …,
    pub can_manage: bool,
    pub groups: Vec<ManualTeamGroupVm>,
    pub line_count: u32,
    pub team_count: u32,
}

pub struct ManualTeamGroupVm {
    pub team_id: String,
    pub team_name: String,
    /// Déjà formaté et signé — « +5 », « −3 ».
    pub total_label: String,
    pub total_is_positive: bool,
    pub line_label: String,        // « 2 lignes », « 1 ligne »
    pub lines: Vec<ManualLineVm>,
}

pub struct ManualLineVm {
    pub id: i64,
    pub points_label: String,      // « +3 », signe compris
    pub is_positive: bool,
    pub reason: Option<String>,
    pub date_label: String,        // « 19 août »
    pub awarded_by: String,
}
```

**`can_manage` porte le contrôle d'accès dans le VM.** La page est consultable
par tous (règle 3) ; seules les actions sont réservées. Le gabarit n'a donc pas
à interroger quoi que ce soit — il rend ou non la colonne de suppression.

**`is_positive` et non une classe CSS.** Le VM dit ce que la ligne est ; le
gabarit choisit `mp-pts--plus`. Un `css_class: String` figerait la présentation
dans le Rust — la leçon de `RowKind` sur l'onglet Trésorerie.

**`line_label` est formaté côté serveur** parce que le pluriel en dépend, et
qu'Askama ne sait pas le faire. C'est aussi l'un des six pluriels bricolés que
la carte 395 recense — celui-ci naît juste, avec la règle française `n > 1`.

## Sortie — le classement

Les deux VM existants gagnent un champ chacun.

```rust
// classement_widget.rs — le classement simple
pub struct ClassementRowVm {
    …,
    pub points: u32,               // inchangé : les points de match
    pub manual: Option<i32>,       // ← None si l'équipe n'en a aucun
    pub total: i32,                // ← le total affiché, points + manuel
}
```

```rust
// detailed_standings_widget.rs — le détaillé
pub struct DetailedRowVm {
    …,
    pub bonus: String,             // inchangé, signé
    pub manual: Option<String>,    // ← « −1 », signé ; None si aucun
    pub total: i32,                // ← était u32
}
```

### Trois choix, et le troisième corrige un type existant

**`manual: Option<…>` et non un zéro par convention.** Le gabarit doit
distinguer « aucun point manuel » — un tiret, non cliquable — de « zéro point
manuel », qui n'existe pas puisque `ManualPoints` refuse zéro. L'`Option` rend
cette impossibilité dans le type.

**`manual` est déjà signé et formaté dans le détaillé**, comme `bonus` l'est
déjà : les deux se lisent comme des contributions au total, jamais comme des
états. Dans le simple, c'est un `i32` que le gabarit signe — la colonne y est
plus étroite.

**`total` passe de `u32` à `i32`** dans les deux VM. Le total peut devenir
négatif (règle 4), et c'est le seul changement de type que la fonctionnalité
impose au code existant.

## Les DTOs de dépôt

```rust
pub struct ManualPointRow {         // DTO de lecture — primitives assumées
    pub id: i64,
    pub team_id: String,
    pub points: i32,
    pub reason: Option<String>,
    pub awarded_by: String,
    pub awarded_at: DateTime<Utc>,
}
```

Et le total par équipe, qui n'a pas besoin d'un type :
`HashMap<String, i32>` — `team_id` vers total.

## Le port d'autorisation

```rust
// ranking/ports.rs
#[async_trait]
pub trait IRankingAdminPort: Send + Sync {
    async fn is_competition_admin(&self, user_id: &str, competition_id: &str) -> bool;
    async fn is_space_admin(&self, user_id: &str, space_id: &str) -> bool;
}
```

*Implémenté par* `infrastructure/ranking/admin_adapter.rs`, à côté de
`competition_info_adapter.rs` · *consommé par* les deux use cases et les
handlers, pour `can_manage`.

**Deux méthodes et non une `is_admin` unique** : les deux autorisations viennent
de deux sources — la compétition porte ses `admin_ids`, l'espace son
`SpaceProfile` — et les fondre cacherait laquelle a répondu.

## Le formatage, et où il se décide

| Ce qui est formaté | Où |
|---|---|
| « +3 », « −1 », « +5 » | `builders.rs` |
| « 19 août » | `builders.rs` |
| « 2 lignes » / « 1 ligne » | `builders.rs` |
| le total du classement | `builders.rs` |

**Rien ailleurs.** Ni dans le service, qui rend des types, ni dans les gabarits,
qui n'ont aucune logique.

## Règles métier tranchées

1. **Zéro point est refusé.** Une ligne qui ne change rien au classement mais
   occupe le relevé et demande à être expliquée est du bruit, pas un
   ajustement.

2. **La borne est ±100**, et elle reste ce qu'elle est : un garde-fou contre la
   faute de frappe — un `300` pour un `3` — et non une règle du jeu. Si une
   ligue avait un jour besoin de davantage, c'est la borne qui bouge, pas le
   principe.
