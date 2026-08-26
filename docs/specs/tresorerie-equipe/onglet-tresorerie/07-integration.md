# Onglet Trésorerie · Phase 7 : effets de bord

**Phase 6** : `06-domaine.md`

## Persistance

### Rien à créer, rien à migrer

Aucune table, aucune colonne, aucun index. `teams__treasury_ledger` existe et
est alimenté depuis l'origine, dans la transaction de l'append
(`team_repository.rs:310`).

**Une seule méthode à ajouter**, sur `ITeamRepository` :

```rust
async fn list_treasury_movements(&self, team_id: &TeamId)
    -> Result<Vec<TreasuryMovementRow>, RepositoryError>;
```

```sql
-- repositories/sql/teams/list_treasury_movements.sql
SELECT l.event_version, l.direction, l.amount_kpo, l.reason,
       l.balance_after_kpo, l.occurred_at, e.payload
FROM   teams__treasury_ledger l
LEFT   JOIN team_event_store e
       ON e.team_id = l.team_id AND e.version = l.event_version
WHERE  l.team_id = $1
ORDER  BY l.event_version
```

Le fichier `.sql` dédié suit la convention du `CLAUDE.md`. `query_as!` de
préférence à `query_as` — la macro vérifie la requête à la compilation, et
c'est la seule requête neuve de la fonctionnalité.

### L'index qui manque peut-être

`teams__treasury_ledger` porte une contrainte d'unicité sur
`(team_id, event_version)` — c'est elle qui permet le `ON CONFLICT DO NOTHING`
de l'écriture. **Elle sert aussi de couverture à cette lecture** : le `WHERE
team_id = $1 ORDER BY event_version` s'appuie exactement dessus.

Rien à ajouter, donc. À vérifier une fois avec `EXPLAIN` plutôt qu'à supposer.

## Événements

**Aucun.** Ni domaine, ni applicatif, ni listener. L'écran ne mute rien.

C'est le troisième écran d'affilée dont la phase 7 le dit ; il vaut la peine de
préciser **pourquoi** ici : le grand livre est déjà un dérivé, écrit en réaction
aux événements de l'équipe. Le lire n'en produit pas de nouveaux, et rien
ailleurs n'a à savoir qu'un coach a consulté sa trésorerie.

## Handlers

Deux, dans un fichier neuf.

```
teams/io/web/treasury_tab.rs
```

```rust
/// Le fragment, cible du clic sur l'onglet.
pub async fn treasury_tab(
    auth_session: AuthSession,
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response;
```

Et l'aiguillage de la page complète, dans `team_detail.rs` :

```rust
pub async fn team_detail(…)   // active_tab = "squad"
pub async fn team_page_treasury(…)  // active_tab = "treasury", même page, autre onglet actif

let content = match active_tab {
    "treasury" => render_treasury(…).await?,
    _          => render_squad(…),
};
```

**Un seul gabarit de fragment pour les deux chemins** (phase 4) : le clic htmx
et le chargement direct de l'URL rendent le même bloc, l'un nu, l'autre
enveloppé dans la page.

### Les sorties

| Cas | Réponse |
|---|---|
| Nominal | le fragment |
| Équipe sans mouvement au-delà de la dotation | le même fragment, bloc « Aucun mouvement pour l'instant » |
| `MissingOpeningEntry` | `500` + journal `ERROR` — une base incohérente, pas une erreur d'usage |
| `UnknownReason(motif)` | `500` + journal `ERROR` portant le motif — c'est lui qu'on cherchera |
| Équipe inexistante ou d'un autre espace | `404`, rendu par `space_scope` avant le handler |
| Non connecté | `401`, rendu par `require_auth` |

**`500` et non `422` pour les deux refus de cohérence.** Un `422` dirait au
coach qu'il a mal fait quelque chose ; il n'y est pour rien. Ces deux cas
décrivent une base qui ne devrait pas exister, et la seule action utile est
qu'ils apparaissent dans le journal avec leur `rid`.

**Aucun contrôle d'accès nouveau.** Celui de la fiche équipe s'applique tel
quel : `require_auth`, puis `space_scope` sur `{team_id}`, dont `teams` déclare
déjà le résolveur (`infrastructure/teams/space_ownership.rs`). La trésorerie
n'est pas plus sensible que la valeur d'équipe, déjà affichée dans l'en-tête à
qui voit la page.

### Les routes

```rust
// teams/routes.rs — une constante de plus
TEAM_TREASURY  "/app/{space_id}/teams/{team_id}/tresorerie"
```

```rust
// teams/router.rs
.route(path::TEAM_DETAIL,   get(team_detail))
.route(path::TEAM_TREASURY, get(team_page_treasury))
```

Le fragment n'a **pas de route à lui** : c'est `TEAM_TREASURY` que l'onglet
appelle en `hx-get`, et le handler distingue les deux chemins à l'en-tête
`HX-Request` — comme le fait l'administration de compétition. Une seconde route
`/tresorerie/fragment` doublerait la surface pour la même réponse.

## Templates

```
teams/io/web/templates/
├── teams-team-detail.html          modifié : #team-tab-content, onglets câblés
└── teams-treasury-tab.html         neuf : le fragment
```

### Ce que `teams-team-detail.html` perd et gagne

Les trois `<div class="tab">` inertes deviennent des `<a>` porteurs de
`hx-get` / `hx-target` / `hx-push-url` (phase 2). **« Matchs » reste inerte** —
hors périmètre, et une route vide se lirait comme une panne.

Tout ce qui suit les onglets — `#players-widget` et `.staff-panel` — entre dans
un `#team-tab-content`. **Copier-coller, pas réécriture** : règle 5 du
`CLAUDE.md`.

### CSS

Une feuille, `pages/team-treasury.css`, portée par `.treasury`, à inscrire dans
`src/web/css_bundle.rs` **juste après `pages/team-page.css`** (ligne 112) :
l'ordre du bundle est imposé, et les deux feuilles s'appliquent à la même page.

Elle ne reprend rien : l'en-tête et les onglets vivent déjà dans
`team-page.css`, le relevé n'a aucun équivalent ailleurs.

Le style des onglets, lui, **est déjà là** — `.tabs` et `.tab` existent dans
`team-page.css`, puisque les trois `<div>` inertes s'affichent correctement
aujourd'hui. Les rendre cliquables ne demande aucune règle nouvelle, sauf le
`cursor: pointer` que des `<a>` apportent d'eux-mêmes.

## Tests E2E

Fichier `tests/e2e/test_team_treasury_tab.py`.

| Scénario | Ce qu'il prouve |
|---|---|
| `test_l_onglet_tresorerie_s_ouvre_et_affiche_le_releve` | le câblage des onglets, qui n'existait pas |
| `test_l_url_de_tresorerie_se_charge_directement` | `hx-push-url` et le rendu de la page complète — un lien collé doit marcher |
| `test_le_solde_du_releve_egale_celui_de_l_en_tete` | **le test qui compte** |
| `test_une_equipe_neuve_affiche_le_bloc_sans_mouvement` | l'état vide, avec sa dotation |
| `test_le_relevé_montre_le_recrutement_qui_vient_d_etre_fait` | la jointure vers l'événement, de bout en bout |
| `test_l_onglet_joueurs_reste_accessible_apres_un_aller_retour` | la régression que le découpage en onglets peut créer |

**`test_le_solde_du_releve_egale_celui_de_l_en_tete`** est celui qui vaut le
prix de la suite. L'en-tête affiche `treasury_kpo`, lu depuis l'agrégat ; le
relevé affiche `balance_after_kpo`, lu depuis le grand livre. **Ce sont deux
chemins vers la même vérité**, et le seul endroit de l'application où ils sont
côte à côte à l'écran.

Une divergence signifierait que le grand livre a décroché de l'agrégat — ce que
la transaction commune est censée empêcher. Ce test est donc, accessoirement,
le premier contrôle continu de cette garantie.

**`cliquer_quand_cable`** (`tests/e2e/htmx_helpers.py`) pour le clic sur
l'onglet : le contenu arrive par `hx-get`, et le second clic — retour sur
« Joueurs & Staff » — vise du contenu fraîchement injecté, exactement la fenêtre
où un élément est peint mais pas encore câblé. **Pas de `sleep`.**

## Ce que la phase ne prévoit pas

- **Aucun test unitaire de handler** : il n'orchestre rien qu'un service ne
  fasse déjà, et les tests du service (phase 5) couvrent la logique.
- **Aucune migration, aucun script de reprise** : le grand livre est complet
  depuis l'origine — 3 258 équipes, toutes avec leur dotation (phase 5).
- **Aucun changement du chemin d'écriture.**
