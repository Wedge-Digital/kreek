# Onglet Paramètres · Phase 4 : contrats de données

**Phase 3** : `03-back.md`

## Onze routes, trois extracteurs

| Route | Extracteur | Pourquoi |
|---|---|---|
| `GET …/admin/settings` | — | l'onglet d'assemblage, aucun paramètre |
| `GET` des cinq panneaux | — | tout vient du chemin |
| `POST …/settings/general` | `axum::Form` | trois champs plats |
| `POST …/settings/visibility` | `axum::Form` | trois champs plats |
| `POST …/settings/pools` | `axum_extra::extract::Form` | listes parallèles |
| `POST …/settings/ranking` | `Json` | agrégat imbriqué |
| `POST …/settings/tiers` | `Json` | agrégat imbriqué |

**La règle : formulaire quand les champs sont plats, JSON quand la cible est un
agrégat déjà sérialisable.** Les deux panneaux qui écrivent dans `rules` visent
`CompetitionRules`, dont chaque champ est un nutype qui **valide à la
désérialisation** — le README de nutype le garantit explicitement, « it works
this way even with serde deserialization ». Un barème hors bornes est donc
rejeté par serde avant d'atteindre la moindre ligne de code. C'est ce que fait
déjà le magicien (`new_competition.rs:445`,
`#[serde(flatten)] rules: CompetitionRules`), et il n'y a aucune raison de
réécrire à la main ce que le type sait faire.

**La garantie ne vaut que pour les nutypes.** `TiebreakConfig` est un newtype
écrit à la main, et il ne tient que par un `#[serde(try_from = "Vec<TiebreakSetting>")]`
posé exprès — son commentaire le dit : sans lui, « un `Deserialize` nu
reconstruirait le newtype sans passer par `try_new`, et n'importe quel payload
JSON contournerait les invariants ». Tout type de commande écrit à la main dans
ce chantier doit porter la même annotation.

**`axum_extra::extract::Form` et non celui d'axum** dès qu'il y a une clé
répétée : celui d'axum s'appuie sur `serde_urlencoded`, qui refuse les
répétitions et rend un 422 (`invalid type: string, expected a sequence`). Le
précédent, et son commentaire, sont dans `roster_edition_controller.rs:20`.

---

## 1 · `#settings-general`

### Entrée

```rust
#[derive(Deserialize)]
pub struct GeneralSettingsForm {
    pub name: String,          // nom de la compétition
    pub season_name: String,   // nom de la saison
    pub logo_url: String,
}
```

*Émis par* le formulaire du panneau (`hx-post`, macro `cmp::cloudinary_upload`
pour le logo, reprise de `new-competition-phase-1.html:78`) · *consommé par*
`post_settings_general`.

Le handler construit **deux** commandes : `UpdateCompetitionIdentityCommand`
(`name`, `logo`, **`admin_ids` relus**) et `SaveCompetitionRulesCommand`
(`season_name`, **`rules` relues**). C'est la double relecture décrite en
phase 3 ; elle n'apparaît pas dans le DTO d'entrée, et c'est précisément ce qui
la rend facile à oublier.

### Sortie

```rust
#[derive(Template)]
#[template(path = "admin/widgets/settings-general.html")]
pub struct SettingsGeneralWidget {
    pub routes: AppRoutes,
    pub space_id: String, pub competition_id: String, pub season_id: String,
    pub vm: GeneralVm,
    pub saved: bool,                 // pour l'état « enregistré » au retour
    pub error: Option<String>,
}

pub struct GeneralVm {
    pub name: String,
    pub season_name: String,
    pub logo_url: String,
    pub admins: Vec<AdminRowVm>,     // affichage seul
}

pub struct AdminRowVm { pub coach_name: String, pub is_owner: bool }
```

`GeneralVm::from_domain(&CompetitionBaseInfo, &SeasonBaseInfo)` — purement
domaine, constructeur co-localisé.

---

## 2 · `#settings-ranking`

### Entrée

```rust
#[derive(Deserialize)]
pub struct RankingSettingsPayload {
    #[serde(flatten)]
    pub ranking_rules: RankingRules,   // le domaine, validé par nutype
}
```

*Émis par* le JS du panneau (il sérialise ses champs, y compris l'ordre des
critères de départage tel que le glisser-déposer l'a laissé) · *consommé par*
`post_settings_ranking`.

`RankingRules` porte déjà tout : `win_points`, `draw_points`, `lose_points`
(`RankingPoints`, ≤ 100 000), les trois bonus avec leur `Activated`, et
`tiebreakers: TiebreakConfig` — dont le smart constructor refuse la liste vide,
les doublons de code et l'absence de tout critère actif.

**Le handler n'a donc aucune validation à écrire.** Il relit `tiers`, assemble
`CompetitionRules { ranking_rules, tiers }`, et appelle le use case.

### Sortie

```rust
pub struct SettingsRankingWidget {
    …,
    pub vm: RankingVm,
    pub recompute: Option<RecomputeVm>,   // présent au retour du POST
}

pub struct RankingVm {
    pub win_points: u32, pub draw_points: u32, pub lose_points: u32,
    pub offensive: BonusVm, pub defensive: BonusVm, pub aggressive: BonusVm,
    pub tiebreakers: Vec<TiebreakRowVm>,
}

pub struct BonusVm { pub activated: bool, pub threshold: u32, pub points: u32 }

pub struct TiebreakRowVm {
    pub code: String,
    pub label: String,      // vient du catalogue, pas du domaine
    pub activated: bool,
}

pub struct RecomputeVm { pub matches_replayed: u32, pub teams: u32 }
```

`TiebreakRowVm` **dépend du catalogue** (`ITiebreakCatalogPort::all()`) autant
que du domaine : il se construit donc dans `builders.rs`, pas par un
`from_domain()`. C'est la règle du `CLAUDE.md` sur les VMs qui dépendent d'un
port.

Sa construction est une **jointure ordonnée** : l'ordre vient de la
`TiebreakConfig` enregistrée, les libellés du catalogue, et les critères du
catalogue absents de la configuration s'ajoutent à la fin, désactivés. Sans
cette dernière règle, un critère ajouté au catalogue serait invisible pour
toutes les compétitions existantes.

---

## 3 · `#settings-pools`

### Entrée

```rust
#[derive(Deserialize)]
pub struct PoolsSettingsForm {
    #[serde(default)] pub use_pools: bool,
    #[serde(default)] pub pool_id: Vec<String>,     // vide = poule nouvelle
    #[serde(default)] pub pool_name: Vec<String>,
}
```

*Émis par* le formulaire du panneau — les poules marquées « à retirer » ne sont
**pas** soumises, c'est ce qui exprime le retrait (phase 2) · *consommé par*
`post_settings_pools`.

**Listes parallèles, donc invariant à vérifier dans le handler** :
`pool_id.len() == pool_name.len()`. Un écart est un 400, jamais un `zip` qui
tronque en silence — `zip` s'arrête sur la plus courte et perdrait une poule
sans rien dire.

### Sortie

```rust
pub struct SettingsPoolsWidget {
    …,
    pub vm: PoolsVm,
    pub unassigned: Option<u32>,   // affectations défaites, au retour du POST
}

pub struct PoolsVm {
    pub use_pools: bool,
    pub pools: Vec<PoolRowVm>,
}

pub struct PoolRowVm {
    pub id: String,
    pub name: String,
    pub assigned_teams: u32,   // ce que le retrait coûterait
}
```

`assigned_teams` vient de `competition_group_teams`, pas du JSONB — c'est le
seul endroit qui sait qui joue où. `PoolRowVm` se construit donc dans
`builders.rs` : structure + comptages du dépôt des poules.

C'est ce compteur qui alimente le pied de panneau — « 6 équipes à réaffecter » —
et il serait faux s'il était lu dans la déclaration.

---

## 4 · `#settings-tiers`

### Entrée

```rust
#[derive(Deserialize)]
pub struct TiersSettingsPayload {
    pub tiers: Vec<TierRule>,   // le domaine, validé par nutype
}
```

*Émis par* le JS du panneau, qui tient une carte `instanceId → selected[]`
alimentée par l'événement `inducementPickerChanged` du widget de `references`
· *consommé par* `post_settings_tiers`.

Le widget de sélection **n'a pas de champ caché** : il garde sa sélection dans
son état Alpine et n'émet qu'un événement
(`inducement-picker.html:15`). Sans ce JS de collecte, le panneau enverrait des
tiers aux listes vides sans qu'aucune erreur ne le signale — c'est le point le
plus facile à rater de tout l'onglet.

**Le panneau ne modifie ni le nom, ni le budget, ni l'XP de départ, ni les
rosters** (phase 2). Ils traversent le POST inchangés parce que `TierRule` est
un tout ; le handler doit **vérifier qu'ils n'ont pas bougé** et refuser sinon.
Sans ce contrôle, une requête forgée modifierait un budget par une porte que
l'écran n'ouvre pas.

### Sortie

```rust
pub struct SettingsTiersWidget {
    …,
    pub vm: Vec<TierVm>,
    pub picker_url: String,        // app_routes.references.inducement_picker()
}

pub struct TierVm {
    pub index: u8,                 // pour la teinte : .tier-block--1, --2, …
    pub name: String,
    pub budget_kpo: u32,           // affichage seul
    pub starting_xp: u32,          // affichage seul
    pub roster_names: Vec<String>, // affichage seul
    pub inducements: Vec<ChipVm>,
    pub star_players: Vec<ChipVm>,
    pub picker_instance_id: String,
}

pub struct ChipVm { pub uid: String, pub label: String }
```

`ChipVm` et `roster_names` résolvent des uid en libellés par
`ICompetitionReferencePort` — donc `builders.rs`. Un uid non résolu **s'affiche
tel quel** plutôt que de disparaître : un coup de pouce retiré du corpus doit se
voir, pas s'évaporer du tier.

---

## 5 · `#settings-visibility`

### Entrée

```rust
#[derive(Deserialize)]
pub struct VisibilitySettingsForm {
    pub access_mode: String,             // "invitation" | "open"
    pub requires_validation: String,     // "manual" | "automatic"
    #[serde(default)] pub max_participants: Option<u32>,
}
```

*Émis par* le formulaire du panneau · *consommé par*
`post_settings_visibility`.

Deux chaînes et non deux booléens : ce sont des `<select>` à deux options, et un
libellé futur (« sur candidature ») ne doit pas exiger de changer le type.
Le handler les traduit en `AccessMode` et `RequiresValidation`, et **une valeur
inconnue est un 400** — pas un repli silencieux sur le défaut, qui ouvrirait une
compétition fermée.

`max_participants` : `0` vaut « illimité » à l'écran, donc `Some(0)` devient
`None` dans la commande. La traduction est faite **une fois, dans le handler**.

### Sortie

```rust
pub struct SettingsVisibilityWidget {
    …,
    pub vm: VisibilityVm,
}

pub struct VisibilityVm {
    pub access_mode: String,
    pub requires_validation: bool,
    pub max_participants: u32,      // 0 = illimité
    pub invited_count: u32,         // affichage seul, rappelle ce qu'on ne touche pas
}
```

`invited_count` existe pour une raison : le panneau réécrit `invitations`, dont
`invited_coaches` fait partie. Afficher « 12 coachs invités » rend visible ce
que le POST doit préserver — la relecture cesse d'être une précaution invisible.

---

## DTOs de port

### Deux méthodes sur `IRankingRepository` (BC `ranking`)

```rust
async fn find_all_lines_for_season(&self, season_id: &str)
    -> Result<Vec<RankingLineRow>, RankingRepositoryError>;

async fn replace_lines_for_season(&self, season_id: &str, lines: &[RankingLine])
    -> Result<(), RankingRepositoryError>;
```

`RankingLineRow` existe déjà — DTO de lecture, primitives assumées.

### Une méthode sur le port des poules (BC `competitions`)

```rust
/// Rend le nombre d'affectations défaites par la cascade.
async fn delete_groups_absent_from(&self, season_id: &str, kept_ids: &[String])
    -> Result<u64, GroupRepositoryError>;
```

*Consommé par* le use case des poules · *alimente* `PoolsWidget.unassigned`.

### Le port de commande vers `ranking`

```rust
// competitions/ports.rs
#[async_trait]
pub trait IRankingRecomputePort: Send + Sync {
    async fn recompute_season(&self, season_id: &str) -> Result<RecomputeReportDto, String>;
}

pub struct RecomputeReportDto { pub matches_replayed: u32, pub teams: u32 }
```

*Implémenté par* `infrastructure/competitions/ranking_recompute_adapter.rs`
· *consommé par* `post_settings_ranking` · *alimente* `RecomputeVm`.

Il rend un compte-rendu et non `()` : « recalculé » sans chiffre ne se distingue
pas de « rien à recalculer », et c'est justement ce que l'écran doit dire.

---

## Le piège des cases à cocher dans des listes parallèles

Trois panneaux mêlent des cases à cocher à des listes : les bonus et le
départage côté barème, `use_pools` côté poules.

**Une case décochée n'est pas soumise.** Deux `Vec` parallèles dont l'un vient
de cases à cocher se désynchronisent donc dès la première case décochée : la
troisième valeur du second `Vec` ne correspond plus au troisième code du
premier, et l'enregistrement écrit un réglage sur le voisin du bon — sans
erreur, sans 422, sans rien.

Le barème y échappe : il part en **JSON**, où `activated` est un booléen
toujours présent.

Le panneau des poules doit s'en garder autrement : `use_pools` porte
`#[serde(default)]`, et **aucune liste parallèle du panneau ne vient d'une case
à cocher** — `pool_id` et `pool_name` sont des champs texte, toujours soumis.
C'est une contrainte sur le gabarit, pas seulement sur le DTO : y ajouter plus
tard une case « poule active » en champ parallèle rouvrirait le piège.

---

## Règles métier tranchées

1. **Un tier sans aucun coup de pouce est valide.** `TierRule.inducements` est
   un `Vec` sans borne basse, et il le reste : vider un tier de ses coups de
   pouce est un enregistrement ordinaire, pas une erreur à rattraper.

2. **On peut retirer la dernière poule.** Toutes les équipes repassent alors en
   attente d'affectation, et il faudra qu'une poule existe à nouveau pour les
   affecter. C'est le comportement voulu, pas un état dégradé.

   Ce que ça impose au back : la suppression explicite décrite en phase 3 est
   **obligatoire**, elle n'est pas un raffinement. La projection paresseuse
   (`ensure_groups_from_structure`) est gardée par `if !struct_groups.is_empty()`
   et ne ferait donc strictement rien sur une liste vide — sans la suppression,
   « retirer toutes les poules » laisserait la table intacte et les équipes
   affectées à des poules que plus rien ne déclare.

   `delete_groups_absent_from(season_id, &[])` couvre ce cas sans être un cas
   particulier : la liste conservée est vide, tout part, et la cascade sur
   `competition_group_teams` désaffecte tout le monde.

3. **Le nom de compétition reste unique dans l'espace**, et l'écran doit savoir
   le dire. `update_draft_competition` rend déjà
   `CompetitionNameAlreadyTaken` ; la maquette reçoit un emplacement d'erreur
   **sous le champ Nom**, alimenté par `SettingsGeneralWidget.error`.

   Le retour du POST rend le widget entier : l'erreur s'affiche donc sans
   traitement client, et le nom refusé reste dans le champ pour être corrigé
   plutôt que ressaisi.
