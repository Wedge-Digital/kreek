# Recrutement — Phase 7 : persistance, événements, réponses

**Entrée** : `06-domaine.md` validé.

Ce document couvre les effets de bord **communs aux deux pages**, puis ceux du
recrutement. `renvois/07-integration.md` consigne ses écarts.

## 1. Persistance

### Migrations

| Migration | Contenu |
|---|---|
| `teams__phase_drafts` | brouillon des deux phases, `PRIMARY KEY (team_id, phase)`, colonne `version` |
| `teams__treasury_ledger` | grand livre, une ligne par mouvement |
| `team_event_store` — `ALTER` | colonne `tags JSONB NOT NULL DEFAULT '[]'` + index GIN |
| `players_proj` — `ALTER` | colonne `membership TEXT NOT NULL DEFAULT 'Active'` |

```sql
CREATE TABLE teams__treasury_ledger (
    id                BIGSERIAL   PRIMARY KEY,
    team_id           TEXT        NOT NULL,
    event_version     BIGINT      NOT NULL,   -- version de l'événement source
    direction         TEXT        NOT NULL,   -- 'Credit' | 'Debit'
    amount_kpo        INT         NOT NULL,
    reason            TEXT        NOT NULL,
    balance_after_kpo INT         NOT NULL,
    occurred_at       TIMESTAMPTZ NOT NULL
);
CREATE INDEX teams__treasury_ledger_team ON teams__treasury_ledger (team_id, id);
CREATE UNIQUE INDEX teams__treasury_ledger_source
    ON teams__treasury_ledger (team_id, event_version);
```

L'unicité sur `(team_id, event_version)` rend l'alimentation **idempotente** : rejouer
un événement ne duplique pas sa ligne.

### Méthodes de repository

| Méthode | Statut |
|---|---|
| `ITeamRepository::append_batch(team_id, &[event], expected_version)` | **nouvelle** — une transaction, N événements, projection **et** grand livre |
| `ITeamRepository::append(...)` | inchangée |
| `IPhaseDraftRepository::load(team_id, phase)` | nouvelle |
| `IPhaseDraftRepository::save(draft, expected_version)` | nouvelle — `UPDATE … WHERE version = $` |
| `IPhaseDraftRepository::delete(team_id, phase)` | nouvelle |
| `IPlayerProjectionRepository::find_by_team_id(...)` | **modifiée** — filtre `membership = 'Active'` à la source |

`append_batch` écrit dans une seule transaction : les N événements à versions
croissantes, la projection `team_proj` pour chacun, et le grand livre pour ceux dont
`treasury_movement()` retourne `Some`. La règle des projections event-sourcées est
respectée de bout en bout.

## 2. Événements et listeners

### Domain events de `teams`

| Événement | Émis par | Mouvement de trésorerie |
|---|---|---|
| `PlayerRecruited` | `Team::recruit_player` | **débit** du coût |
| `StaffBought` | `Team::buy_staff` | **débit** du coût |
| `RecruitmentPhaseValidated` | `Team::validate_recruitment_phase` | aucun |

### App events sortants

| App event | Origine | Consommateur |
|---|---|---|
| `PlayerRecruited` | publisher `teams`, depuis le domain event | listener `players` → `PlayerCreated` |

Le listener de `players` **réutilise la logique de `team_created_listener`** —
résolution des compétences de base, valeur de départ, attribution du premier maillot
disponible. À factoriser dans une fonction partagée plutôt qu'à dupliquer.

### Listeners intra-BC de `teams`

| Listener | Abonné à | Rôle |
|---|---|---|
| `phase_draft_purge_listener` | bus interne, 4 entrées en `ReadyToPlay` | supprime les deux brouillons (D6) |

Signature `init(event_bus: &EventBus, …)` — convention que `check-arch` (axe 5)
utilise pour reconnaître un listener intra-BC.

**Prérequis : carte 251**, qui crée le bus interne de `teams` et la publication depuis
`TeamRepository::append`. `append_batch` devra publier de la même façon.

## 3. Handlers

Tous retournent `Result<impl IntoResponse, AppError>` et ne contiennent aucune logique
métier : ils parsent, construisent la commande, appellent le use case, rendent.

```rust
// io/web/recruitment.rs
pub async fn recruitment_page(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError>

// io/web/widgets/recruitment_catalog_widget.rs
pub async fn recruitment_catalog(Path(..), State(..)) -> …           // GET
pub async fn add_player(Path(..), State(..), Form(AddPlayerBody))    // POST
pub async fn add_staff(Path(..), State(..), Form(AddStaffBody))      // POST

// io/web/widgets/recruitment_cart_widget.rs
pub async fn recruitment_cart(Path(..), State(..)) -> …              // GET
pub async fn remove_line(Path(..), State(..), Form(RemoveLineBody))  // POST
```

### Réponses

| Cas | Réponse |
|---|---|
| mutation réussie | fragment du widget cliqué + `HX-Trigger: draftChanged` |
| `ConcurrentWrite` | **200** + fragment reconstruit + bandeau de resynchronisation + `HX-Trigger: draftChanged` |
| erreur domaine | 422 + `draft-error.html` |
| phase incorrecte | 422 |
| équipe inconnue | 404 |
| validation de phase | `HX-Refresh: true` |

`ConcurrentWrite` répond **200 et non une erreur** : le geste n'est pas appliqué, mais
l'utilisateur reçoit une page cohérente. C'est la première fois dans ce projet qu'un
`ConcurrentWrite` remonte jusqu'à l'interface.

## 4. Templates

| Template | VM consommé |
|---|---|
| `templates/recruitment.html` | page d'assemblage — deux conteneurs `hx-get`, aucune logique |
| `templates/widgets/recruitment-catalog.html` | `RecruitmentCatalogVm` |
| `templates/widgets/recruitment-cart.html` | `RecruitmentCartVm` |
| `templates/widgets/draft-error.html` | `DraftErrorVm` — **partagé** avec les renvois |

Conventions à respecter, reprises des maquettes validées : `hx-disinherit="*"` sur la
racine de chaque widget, CSS embarqué, version cuite dans les `hx-vals`, aucun
`style=` inline, `kreek-select` si un sélecteur apparaît.

Le seul JavaScript est le repli du panier sous 768px, en `x-data` Alpine avec
`init()`/`destroy()`.

## 5. Tests e2e prévus

Fichier `tests/e2e/test_recruitment_phase.py`, à déclarer dans
`tests/impact-map.toml` — BCs traversés : `teams`, `players`, `references`,
`team_creation` (fixture), `competitions` (fixture).

| # | Scénario |
|---|---|
| 1 | Depuis la bannière de phase, « Recruter → » ouvre la page ; le catalogue liste les postes du roster avec leurs prix |
| 2 | Ajouter un joueur : la ligne apparaît au panier, le reste de trésorerie diminue, le quota affiche `+1` |
| 3 | Retirer la ligne : le panier se vide, la trésorerie et le quota reviennent à l'état initial |
| 4 | Trésorerie insuffisante : le bouton affiche « Trésorerie » et **rien n'est débité** |
| 5 | Quota de poste atteint : le bouton affiche « Quota atteint » |
| 6 | Roster sans apothicaire : la ligne affiche le motif, le bouton est inactif |
| 7 | Relance affichée au double du prix de base, avec le prix de base rappelé |
| 8 | Valider les achats : la trésorerie est débitée **du total**, les joueurs existent, l'équipe passe en phase de renvois |
| 9 | Quitter la page sans valider, y revenir : **le panier est toujours là** — c'est la propriété que le panier serveur achète |
| 10 | Le grand livre de trésorerie contient une ligne par achat, avec le solde après |
| 11 | Mobile 390px : le panier est la barre du bas, repliable, ses `×` sont atteignables |

Le scénario 9 est le plus important : il vérifie la décision D1. Le scénario 4 vérifie
qu'aucun débit n'a lieu avant validation — la propriété qui rend le brouillon sûr.

## 6. Points ouverts pour la phase 8

- Le test 10 lit le grand livre en base : passe-t-il par `db_helpers.py` ou par un
  écran d'historique qui n'existe pas encore ? Une page de trésorerie n'est pas au
  périmètre de cette feature, donc probablement par la base.
- Le découpage en cartes doit ordonner **carte 251 avant tout** : sans le bus interne
  de `teams`, ni le listener de purge ni la publication de `append_batch` n'ont de
  support.
