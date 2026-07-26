# Phase 7 — Effets de bord (`competition-rules-form`)

## Persistance — aucune migration

Les règles vivent dans `competition_seasons.rules JSONB` (nullable, migration
`20260525000002_competition_seasons.sql:5`). **Seule la forme du JSON change**, pas le
schéma : aucune migration à écrire.

| Élément | État |
|---|---|
| `sql/seasons/update_rules.sql` | **inchangé** — `SET rules = $2::jsonb, status = 'rules_selected'` |
| `sql/seasons/select_rules.sql` | **inchangé** |
| `ISeasonRepository::save_rules` / `find_rules` | **inchangés** — aucune méthode nouvelle |

**Un effet de bord à connaître** : `find_rules` (`season_repository.rs:73`) désérialise
la colonne avec `serde_json::from_str`. La validation passant désormais par
`#[serde(try_from = …)]`, une configuration invalide en base ferait échouer la lecture
en `SeasonRepositoryError::Database`. Le cas ne peut survenir que par écriture manuelle
en base — c'est le comportement souhaité (échouer bruyamment plutôt que charger un
agrégat invalide), mais il faut le savoir en debug.

## Événements — aucun

`save_competition_rules` n'émet aucun événement aujourd'hui et n'en émettra aucun : dans
ce parcours, seul `post_new_competition` publie sur l'`event_bus`
(`new_competition.rs:273`).

C'est cohérent avec le critère de choix du CLAUDE.md (« Consultation vs propagation
d'effet ») : `ranking` a besoin de **lire** la configuration au moment de calculer un
classement, pas de réagir à un fait qui vient de se produire. C'est une consultation →
port + adapter, déjà en place (`infrastructure/ranking/competition_info_adapter.rs:34`).
Aucun app event, aucun listener, aucune projection locale de la configuration.

## Handlers

### `get_new_competition_phase_2` (`new_competition.rs:48`)

Signature inchangée. Ajout : lecture du catalogue via
`state.competitions.tiebreak_catalog_port`, projection en JSON, passage au template.

**Le handler fait aujourd'hui 38 lignes** (48-85) — il est déjà hors de la limite des
20 lignes du CLAUDE.md, et l'ajout l'aggraverait. Puisqu'on le modifie, il est découpé :

```rust
async fn load_existing_rules_json(repo: &dyn ISeasonRepository, sid: &SeasonId) -> String
async fn load_season_name(repo: &dyn ISeasonRepository, sid: &SeasonId) -> String
fn tiebreak_catalog_json(catalog: &dyn ITiebreakCatalogPort) -> String
```

Le `tokio::join!` des deux lectures est conservé dans le handler — c'est lui qui porte la
concurrence, les helpers ne font que projeter chaque résultat.

### `post_competition_rules` (`new_competition.rs:418`)

**45 lignes aujourd'hui** (418-462), essentiellement du mapping d'erreurs, et trois
variantes s'y ajoutent. Le mapping est extrait :

```rust
fn map_save_rules_error(e: SaveCompetitionRulesError) -> Response
```

Le handler se réduit alors à : parser `SeasonId`, construire la commande, appeler le use
case avec le repository **et le port catalogue**, puis rediriger ou déléguer l'erreur.

Nouvelles réponses, dans le style français explicite déjà en place pour
`RosterInMultipleTiers` :

| Erreur | HTTP | Message |
|---|---|---|
| `NoActiveTiebreaker` | 422 | « Au moins un critère de départage doit être actif. » |
| `DuplicateTiebreakCode { code }` | 422 | « Le critère de départage « … » est présent plusieurs fois. » |
| `UnknownTiebreakCriterion { code }` | 422 | « Le critère de départage « … » est inconnu. » |

Le front affiche déjà ces messages tels quels dans `#rules-error-banner`
(`new-competition-phase-2.html:409-411`) — rien à ajouter côté bannière.

## Templates

### `new-competition-phase-2.html`

| Changement | Détail |
|---|---|
| `TIEBREAK_CRITERIA` (ligne 164) | **supprimée** — remplacée par `JSON.parse` du catalogue injecté |
| `criteriaOrder` (ligne 174) | Liste de `{ code, label, activated }` au lieu de `{ id, label }` |
| `renderTiebreaks()` (ligne 177) | Ligne en `<label>`, case à cocher, classe `is-off`, rang « — » pour les inactifs, numérotation sur les **actifs seulement** |
| Drag & drop (lignes 186-214) | Inchangé dans son principe — l'ordre porte sur la liste complète, inactifs compris |
| `buildJSON()` (ligne 373) | Produit le tableau `tiebreakers: [{ code, activated }]` au lieu de la map `{ id: priorité }` |
| `initFromExistingRules()` (ligne 446) | Hydrate ordre **et** activation ; complète depuis le catalogue les critères absents, actifs |
| Garde-fou règle 1 | Bouton « Enregistrer & continuer » désactivé + message inline dès que zéro critère est actif |

Aucun template nouveau, aucun fragment : la section n'est pas un widget (cf.
`02-front.md`, D1).

### CSS — `assets/static/css/pages/new-competition-phase-2.css`

| Changement | Détail |
|---|---|
| `.tiebreak-check` | **ajout** — calqué sur `.bonus-check`, même `accent-color` |
| `.tiebreak-row.is-off` | **ajout** — opacité réduite, pastille de rang en contour gris |
| `.tiebreak-remove` / `:hover` (lignes 71-72) | **suppression** — code mort : le template n'a jamais eu de bouton ✕ (vérifié par grep sur `tiebreak-remove` dans `src/`) |

Reprend à l'identique ce qui a été validé sur la maquette `app-league-rules.html`.

## Tests E2E

Nouveau fichier `tests/e2e/test_competition_rules_tiebreakers.py`, calqué sur
`test_competition_rules_bonus.py` — le test round-trip de la phase 2 livré par la
feature `ranking-bonus-points`, qui fournit déjà les helpers de navigation jusqu'au
formulaire de règles.

| Scénario | Vérifie |
|---|---|
| Round-trip complet | Décocher deux critères, réordonner par drag & drop, enregistrer, revenir sur la phase 2 → l'ordre **et** l'activation sont restitués (règles 1 à 3) |
| Renumérotation | Décocher le critère de rang 1 → le suivant actif affiche 1, l'inactif affiche « — » |
| Garde-fou | Tout décocher → bouton d'enregistrement désactivé, message inline visible, aucune requête envoyée |
| Catalogue | Les 7 libellés attendus sont présents, et `Nombre de cartons rouges` **absent** (règle 10) |

Le drag & drop en Playwright se pilote avec `drag_to()` sur les `.tiebreak-row` ; en cas
d'instabilité, repli sur une réorganisation via `dispatch_event` des événements
`dragstart`/`drop`, comme cela se pratique déjà ailleurs dans la suite si besoin.

## Règles métier — état

Aucune règle nouvelle. La phase confirme que la règle 4 ne demande aucun code : les
statuts de saison (`draft` → `rules_selected` → `structure_selected` →
`invitations_configured` → `ready`) n'offrent aucune route de retour sur les règles après
la sortie du tunnel de création. Le jour où une page d'admin des règles existera, le
garde-fou naturel sera un refus dès que le statut dépasse le tunnel.
