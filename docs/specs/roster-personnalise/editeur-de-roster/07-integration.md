# Éditeur de roster · Phase 7 : effets de bord

**Phase 6** : `06-domaine.md`

## Ce que cette phase révèle

Les six phases précédentes ont décrit un écran et ses règles. **La moitié du
travail est ici**, et elle est invisible depuis la maquette : `references` n'a
jamais écrit, jamais émis, jamais rien exposé en POST. Tout ce qu'un BC
ordinaire possède, il lui manque.

| Ce qui manque à `references` | Existe ailleurs |
|---|---|
| `use_cases/` | tous les autres BCs |
| `ports.rs` | tous |
| Un bus interne dans son contexte | tous |
| `domain/domain_event.rs` | tous |
| `io/app_events/app_event_publisher.rs` | `competitions`, `auth`, `spaces`, `team_creation` |
| `src/infrastructure/references/` | dix BCs sur onze |
| Une table | — |

Son contexte tient aujourd'hui en une ligne : `pub repository: Arc<dyn IReferenceRepository>`.

## Persistance

### La table

```sql
-- migrations/<date>_references_custom_rosters.sql
CREATE TABLE references__custom_rosters (
    uid         TEXT PRIMARY KEY,
    space_id    TEXT NOT NULL,
    definition  JSONB NOT NULL,
    created_by  TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON references__custom_rosters (space_id);
```

Le `Team` entier en JSONB : on le lit en bloc, on l'écrit en bloc, rien ne le
requête par poste, et **c'est déjà la forme du corpus** — la sérialisation
existe (phase 3).

### La colonne de `teams`, et son rattrapage

```sql
-- migrations/<date>_team_proj_roster_id.sql
ALTER TABLE team_proj ADD COLUMN roster_id TEXT;

UPDATE team_proj p
SET    roster_id = e.payload->>'roster_id'
FROM   team_event_store e
WHERE  e.team_id = p.team_id AND e.event_type = 'TeamCreated' AND p.roster_id IS NULL;

CREATE INDEX ON team_proj (roster_id);
```

**Du SQL et non le registre Rust** : celui-ci existe pour les rattrapages qui
ont besoin du corpus (`m001_bonus_elite`, `m002_recalcul_valeurs_equipe`). Ici
la donnée est déjà en base.

**L'index après le rattrapage.** Le contrôle d'après-passage est
`SELECT count(*) FROM team_proj WHERE roster_id IS NULL` — doit valoir zéro.

### Le dépôt gagne une écriture, et un cache

```rust
#[async_trait]
pub trait IReferenceWriteRepository: Send + Sync {
    async fn save_custom_roster(&self, space_id: &SpaceId, team: &Team, by: &CoachId)
        -> Result<(), RepositoryError>;
    async fn delete_custom_roster(&self, uid: &RosterUid) -> Result<(), RepositoryError>;
    async fn list_for_space(&self, space_id: &SpaceId) -> Result<Vec<Team>, RepositoryError>;
}
```

**Un trait distinct de `IReferenceRepository`**, qui est synchrone et en lecture
seule. Les fondre rendrait la lecture asynchrone — précisément ce que la phase 3
a évité.

L'implémentation écrit en base **puis** rafraîchit `custom_teams`. Si le
rafraîchissement échoue, la base fait foi, la ligne part en `ERROR`, et un
redémarrage remet tout d'aplomb.

> **Le précédent à ne pas répéter** : la carte 362, « le bundle CSS est gelé au
> démarrage » — un cache que rien ne rafraîchit et dont l'obsolescence ne se
> signale pas. Ici le rafraîchissement fait partie de l'écriture.

### Une signature de lecture change

`find_team_by_uid` rend `Option<Team>` au lieu de `Option<&Team>` (phase 3).
**Huit sites d'appel**, tous en retirant un `&` ; aucun ne conserve la référence
au-delà de l'expression.

```
league_selector.rs:49   special_rule_selector.rs:88   consistency.rs:65
reference_data_adapter.rs:20   ref_team_data_adapter.rs:96 et :112
journeyman_type_adapter.rs:29   roster_catalog_adapter.rs:61
```

## Événements

**Un seul, et à la suppression seulement.**

```
delete_custom_roster_use_case
    │  emettre()   ReferencesDomainEvent::CustomRosterDeleted { uid, space_id }
    ▼
references/io/app_events/app_event_publisher.rs        ← n'existe pas
    │  publier()   ReferencesAppEvent::CustomRosterDeleted { roster_uid }
    ▼
competitions/io/app_events/custom_roster_deleted_listener.rs   ← n'existe pas
    │
    ▼  retire l'uid des tiers de toutes les saisons
```

Le publisher se copie sur celui de `competitions`
(`app_event_publisher.rs:6`) : `spawn_listener`, désérialisation du domain
event, `to_app_event()`, `publier()` sous un span `app_event_publication` qui
porte `cause = %envelope.event_id`. **Copier-coller** (règle 5), pas
réécriture.

Câblage dans `main.rs`, à côté des cinq autres :

```rust
references::context::init_app_event_publisher(&event_bus, app_event_bus.clone());
competitions::context::init_custom_roster_listener(&app_event_bus, pool.clone());
```

### Le listener journalise son passage

Combien de saisons parcourues, combien de tiers modifiés. **Un listener
silencieux qui échoue laisse une incohérence que rien ne raconte** — et cette
incohérence-là est précisément celle que le `filter_map` de `builders.rs` avale
sans un mot (carte 438).

`init(app_event_bus: &EventBus, …)` : c'est la convention qui dit à l'axe 5 de
`check-arch` qu'il s'agit d'un listener **cross-BC**, exempté de la règle de
transaction unique.

## Handlers

```
references/io/web/roster_admin/
├── mod.rs
├── roster_list.rs        GET    …/admin/rosters
├── roster_editor.rs      GET    …/admin/rosters/new
│                         GET    …/admin/rosters/{roster_uid}
├── roster_create.rs      POST   …/admin/rosters
├── roster_update.rs      PUT    …/admin/rosters/{roster_uid}
└── roster_delete.rs      DELETE …/admin/rosters/{roster_uid}
```

Six routes sous `/app/{space_id}/admin/rosters`.

```rust
pub async fn post_roster(
    auth_session: AuthSession,
    Path(space_id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<CreateRosterPayload>,
) -> Response;
```

**`Json` et non `Form`** : la structure est imbriquée sur deux niveaux
(phase 4).

**Aucun identifiant de ressource hors du chemin.** C'est la leçon de la carte
416 : `delete_match` et ses voisins prennent leur cible dans le corps, hors de
portée de `space_scope`. Ici le `{roster_uid}` est dans l'URL, donc scopé.

### Le contrôle d'accès

`space_scope` couvre `{space_id}`, mais **pas** `{roster_uid}` — aucun résolveur
ne le déclare. Deux gestes :

1. **Ajouter un résolveur `ISpaceOwnership` pour `roster_uid`** dans
   `infrastructure/references/space_ownership.rs`, sur le modèle des six
   existants. Un roster d'un autre espace rend alors `404` **avant** le handler,
   ce qui satisfait la règle P2 sans une ligne de code métier.
2. **`is_space_admin` en première ligne des trois mutations.** La lecture, elle,
   est ouverte à tout membre : la liste est ce qu'il pourra choisir.

Le résolveur est le geste qui compte : il rend la règle structurelle plutôt que
répétée.

### Les sorties

| Cas | Réponse |
|---|---|
| GET nominal | la page |
| POST/PUT réussi | `HX-Redirect` vers la liste |
| `Invalid(DomainError)` | `422` + la page re-rendue, l'erreur nommant le poste fautif |
| `InUse { teams }` | `409` + le message qui **porte le nombre** |
| `Forbidden` | `403` |
| `NotFound` | `404` |
| `UsageUnavailable` | `503` — on ne sait pas, donc on refuse |

**`409` et non `422` pour `InUse`** : la requête est bien formée, c'est l'état du
système qui s'y oppose. Et **`503` pour `UsageUnavailable`** : le refus est
temporaire, réessayer a du sens — ce qu'un `500` ne dit pas.

## Templates et CSS

```
references/io/web/templates/
├── references-roster-list.html
└── references-roster-editor.html
```

Deux feuilles, portées par `.rl-page` et `.re-page`, **inscrites dans
`src/web/css_bundle.rs`** parmi les pages — l'axe 14 refuse une feuille absente
du bundle.

**Le JS de l'éditeur ne tient pas dans un `<script>` inline.** Trois sélecteurs,
un état à une centaine de champs, un pied de cohérence dérivé : c'est un
composant Alpine, dans un fichier servi comme `kreek-select.js` l'est déjà.

## Tests E2E

`tests/e2e/test_custom_roster.py`.

| Scénario | Ce qu'il prouve |
|---|---|
| `test_creer_un_roster_minimal` | un poste journalier, une espèce, un rôle — le chemin heureux |
| `test_le_roster_cree_apparait_a_la_creation_d_equipe` | **le test qui compte** |
| `test_un_roster_sans_journalier_est_refuse` | S2, bout en bout |
| `test_un_roster_utilise_n_est_pas_modifiable` | le verrou, et le bandeau qui dit la cause |
| `test_le_bouton_supprimer_n_existe_pas_sur_un_roster_utilise` | il n'est pas grisé, il est absent |
| `test_un_non_admin_ne_voit_pas_les_actions` | P1 |
| `test_le_roster_d_un_autre_espace_rend_404` | P2 |
| `test_supprimer_un_roster_le_retire_des_tiers` | la chaîne d'événements complète |

**`test_le_roster_cree_apparait_a_la_creation_d_equipe`** vaut le prix de la
suite. Il traverse tout : l'écriture en base, le rafraîchissement du cache,
l'aiguillage par préfixe dans `find_team_by_uid`, le port de `team_creation`, et
le sélecteur. **C'est aussi le seul qui prouve que le cache n'est pas gelé** —
sans redémarrage entre la création et la vérification.

**`test_supprimer_un_roster_le_retire_des_tiers`** est asynchrone par nature :
il attend que le listener soit passé. Pas de `sleep` — une condition sur l'état
observable, comme `cliquer_quand_cable` le fait pour htmx.

## Ce que la phase ne prévoit pas

- **Aucun import depuis un roster existant** (phase 2).
- **Aucune traduction** : le roster est saisi dans la langue de son auteur.
- **Aucun test de charge** : un espace aura une poignée de rosters, pas mille.

## Règles métier

**Aucune à préciser.** Les dix-sept de la phase 6 couvrent la fonctionnalité, et
cette phase n'en fait apparaître aucune — elle décrit des mécanismes, pas des
décisions.
