# Onglet Paramètres · Phase 7 : effets de bord

**Phase 6** : `06-domaine.md`

## Persistance

### Méthodes existantes, réutilisées telles quelles

| Port | Méthode | Panneau |
|---|---|---|
| `ICompetitionRepository` | `find_base_info`, `name_exists_in_space`, `update_base_info` | Général |
| `ISeasonRepository` | `find_base_info`, `find_rules`, `save_rules` | Général, Classement, Tiers |
| `ISeasonRepository` | `find_structure` | Poules |
| `ISeasonRepository` | `find_invitations`, `find_notifications`, `save_invitations` | Visibilité |
| `IGroupRepository` | `find_groups` | Poules (comptage d'affectations) |

### Méthodes à ajouter

```rust
// ISeasonRepository — une transaction, deux écritures (phase 5)
async fn save_structure_and_prune_groups(
    &self, season_id: &SeasonId, structure: &CompetitionStructure, kept_ids: &[String],
) -> Result<u64, SeasonRepositoryError>;

// IRankingRepository — le rejeu (phase 3)
async fn find_all_lines_for_season(&self, season_id: &str)
    -> Result<Vec<RankingLineRow>, RankingRepositoryError>;
async fn replace_lines_for_season(&self, season_id: &str, lines: &[RankingLine])
    -> Result<(), RankingRepositoryError>;
```

`find_all_lines_for_season` ordonne par `(recorded_at, match_report_id)` — c'est
l'ordre du cumul, et le second terme n'est là que pour rester déterministe quand
deux lignes partagent l'horodatage.

`replace_lines_for_season` supprime et réinsère **dans une transaction**. Le
`DELETE` porte sur `season_id`, pas sur une liste d'identifiants : une ligne
orpheline qui n'aurait pas été relue serait sinon conservée par le rejeu qui
prétend l'avoir remplacée.

### Aucune migration

Aucune table n'est créée, aucune colonne ajoutée. Les trois documents JSONB
gardent leur forme — `RankingGroupConfig` change d'encapsulation, pas de
sérialisation : les champs privés se sérialisent comme les publics, et le
`#[serde(try_from)]` ne lit que ce qui est déjà écrit.

**Un seul point de vigilance au déploiement** : si des saisons existantes
portaient deux poules de même nom, leur `find_structure` **échouerait** après
l'encapsulation — le `try_new` refuse ce que l'ancien `Deserialize` acceptait.
À vérifier avant de livrer :

```sql
select season_id, name, count(*) from competition_groups
group by 1,2 having count(*) > 1;
```

C'est le seul endroit où cette fonctionnalité peut casser des données en place,
et il se vérifie en une requête.

## Événements

**Aucun.** Ni domaine, ni applicatif. La démonstration est en phase 5 : les
quatre variantes de `CompetitionsDomainEvent` sont des faits de cycle de vie, le
classement se recalcule dans le même POST par un port, et les autres BCs
consultent les réglages au lieu d'en tenir copie.

Aucun listener à câbler, aucun publisher à modifier.

## Handlers

Onze, tous dans `io/web/admin/settings/`. Un fichier par panneau, plus
l'assemblage.

```
io/web/admin/settings/
├── mod.rs
├── settings_tab.rs              GET  …/admin/settings
├── general_controller.rs        GET + POST …/settings/general
├── ranking_controller.rs        GET + POST …/settings/ranking
├── pools_controller.rs          GET + POST …/settings/pools
├── tiers_controller.rs          GET + POST …/settings/tiers
└── visibility_controller.rs     GET + POST …/settings/visibility
```

### La signature commune

```rust
pub async fn get_settings_general(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response;

pub async fn post_settings_general(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Form(form): Form<GeneralSettingsForm>,     // ou Json / axum_extra::Form, cf. phase 4
) -> Response;
```

`Response` et non `impl IntoResponse` : chaque handler a plusieurs sorties —
fragment, 403, 422 — et le type opaque force alors des `into_response()`
dispersés.

**`AuthSession` en premier paramètre, et `require_admin_access` en première
ligne, sur les onze.** GET compris : le commentaire de la fonction
(`admin_page.rs:57`) le dit déjà — sans contrôle sur le chemin htmx, le
changement d'onglet contourne l'autorisation.

Le prédicat est **celui-là et pas un autre** : admin de l'espace
(`SpaceProfile::SpaceAdmin`), ou admin de la compétition — par identifiant
(`admin_ids`) **ou** par nom de coach (`admin_names`), les deux listes étant
consultées parce que les deux sont peuplées. Tout le reste est un `403`, le
propriétaire de l'équipe comme le simple participant.

**Aucun panneau n'a de règle propre.** L'autorisation est posée une fois, à
l'entrée ; les cinq panneaux s'ouvrent ensemble ou pas du tout. Une granularité
par panneau créerait un second modèle de droits pour un seul écran, et c'est
exactement le genre de modèle qu'on oublie de tenir à jour.

**Ni le rendu de l'onglet, ni l'onglet lui-même ne servent de garde.** Masquer
l'onglet à un non-admin est du confort, pas de la sécurité : les onze URL
restent atteignables directement, et ce sont elles qui refusent.

> ### Ce que les onze ne doivent pas imiter
>
> **Les treize routes de mutation admin existantes ne vérifient rien.**
> `groups_actions.rs` (3) et `schedule_actions.rs` (10) n'acceptent aucun
> `AuthSession` et n'appellent jamais `require_admin_access` : tirage au sort,
> réinitialisation des poules, génération et suppression de journées et de
> matchs sont ouverts à tout utilisateur connecté.
>
> Pire, plusieurs prennent leur cible **dans le corps de la requête** en
> ignorant les identifiants de chemin (`post_assign_team`, `delete_match`,
> `delete_round` : `Path((_space_id, _competition_id, _season_id))`). Le
> middleware `space_scope` ne résout que les paramètres **de chemin** — il ne
> voit donc pas ces cibles-là.
>
> C'est la **carte 416**, écrite à part. Cette fonctionnalité ne la corrige pas,
> mais elle ne reproduit pas le défaut : ses onze routes contrôlent, et ses
> POST n'acceptent aucun identifiant de ressource hors du chemin.

### Les sorties

| Cas | Réponse |
|---|---|
| GET nominal | le fragment du widget |
| POST réussi | **le widget entier re-rendu**, `hx-swap="outerHTML"` sur sa racine |
| Refus domaine (`DomainError`) | `422` + le widget re-rendu, `error` renseignée |
| Nom déjà pris | `422` + le widget, l'erreur sous le champ (phase 4) |
| Recalcul en échec | `200` + le widget, avec le message : **le barème est enregistré** |
| Non-admin | `403`, corps vide |
| Saison ou compétition introuvable | `404` |

**Le recalcul en échec rend `200` et non `422`**, et c'est délibéré :
l'enregistrement demandé a bien eu lieu. Un `422` dirait le contraire, et
l'utilisateur reprendrait un formulaire déjà sauvegardé.

**Aucun `HX-Trigger`.** Les cinq widgets sont indépendants (phase 2), aucun n'a
à en réveiller un autre. C'est ce qui distingue cet onglet de celui des poules,
dont les actions émettent `groupsChanged`.

### Les routes

```rust
// competitions/routes.rs — six constantes de plus
COMPETITION_ADMIN_SETTINGS             …/admin/settings
COMPETITION_ADMIN_SETTINGS_GENERAL     …/admin/settings/general
COMPETITION_ADMIN_SETTINGS_RANKING     …/admin/settings/ranking
COMPETITION_ADMIN_SETTINGS_POOLS       …/admin/settings/pools
COMPETITION_ADMIN_SETTINGS_TIERS       …/admin/settings/tiers
COMPETITION_ADMIN_SETTINGS_VISIBILITY  …/admin/settings/visibility
```

Chacune montée en `get(...).post(...)` dans `router.rs`.

**Deux constantes disparaissent** — `COMPETITION_ADMIN_DASHBOARD` et
`COMPETITION_ADMIN_RESULTS` — avec leurs méthodes `admin_dashboard()` et
`admin_results()`, leurs routes, et les branches correspondantes de
`admin_page.rs`.

## Templates

```
io/web/templates/admin/
├── settings.html                     l'assemblage : cinq conteneurs hx-get
└── widgets/
    ├── settings-general.html
    ├── settings-ranking.html
    ├── settings-pools.html
    ├── settings-tiers.html
    └── settings-visibility.html
```

Chaque widget porte `hx-disinherit="*"` sur sa racine (règle 3 des widgets) et
une classe racine nommée d'après sa feuille.

**Supprimés** : `admin/dashboard.html` et `admin/results.html`, et les deux
onglets de `admin-page.html`.

### CSS

Une feuille, `pages/competition-admin-settings.css`, portée par
`.competition-admin-settings`. À **inscrire dans `src/web/css_bundle.rs`** parmi
les pages — l'axe 14 de `check-arch` refuse toute feuille absente du bundle.

`pages/competition-admin-dashboard.css` en sort : elle est entièrement portée
par `.competition-admin-dashboard` et meurt avec l'onglet (phase 3).

**Deux composants ne sont pas réécrits** (phase 2) : les puces de coups de
pouce reprennent `widgets/inducement-grid.css`, les blocs de tier reprennent
`pages/new-competition-phase-2.css`. Rien à ajouter au bundle pour eux.

## Tests E2E

Fichier `tests/e2e/test_competition_admin_settings.py`.

| Scénario | Ce qu'il prouve |
|---|---|
| `test_onglet_parametres_charge_les_cinq_panneaux` | l'assemblage et les cinq `hx-get` |
| `test_renommer_la_competition` | le POST le plus simple, bout en bout |
| `test_nom_deja_pris_affiche_l_erreur_sous_le_champ` | l'emplacement d'erreur ajouté en phase 4 |
| `test_modifier_le_bareme_recalcule_le_classement` | **le scénario central** : deux matchs joués, victoire à 3 points au lieu de 2, le classement affiche le nouveau total |
| `test_retirer_une_poule_desaffecte_ses_equipes` | la cascade, vérifiée dans l'onglet Poules |
| `test_retirer_toutes_les_poules` | le cas que la projection paresseuse traite le plus mal |
| `test_modifier_les_coups_de_pouce_d_un_tier` | la collecte JS de l'événement du picker |
| `test_un_non_admin_est_refuse_sur_les_onze_routes` | **paramétré sur les onze**, `403` sur chaque GET et chaque POST |
| `test_un_admin_de_competition_ouvre_les_cinq_panneaux` | l'admin nommé dans la compétition, sans être admin d'espace |
| `test_un_admin_d_espace_ouvre_les_cinq_panneaux` | l'admin d'espace, sans être nommé dans la compétition |
| `test_le_calendrier_survit_a_l_enregistrement_des_poules` | la relecture du JSONB — le défaut le plus silencieux |

**Le quatrième et le dernier sont ceux qui valent le prix de la suite.** Le
recalcul ne se vérifie pas unitairement bout en bout, et l'effacement du
calendrier par un POST de poules ne produirait aucune erreur — juste un
calendrier vide, découvert des jours plus tard.

`cliquer_quand_cable` (`tests/e2e/htmx_helpers.py`) pour tout clic sur du
contenu fraîchement injecté — les cinq widgets arrivent par `hx-get`, ils sont
exactement dans la fenêtre où un élément est peint mais pas encore câblé.

### Un test à supprimer

`tests/e2e/test_competition_admin_dashboard.py` — ses trois cas testent l'onglet
qui disparaît. Il part avec lui, dans le même commit : un test qui référence une
route retirée échoue sans rien apprendre.
