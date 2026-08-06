# Phase 7 — Effets de bord — team-detail

## Autorisation (lacune comblée à cette étape)

Non traitée dans les phases 1-6. Le GET (`player_table_widget`) reste tel
quel — lecture ouverte, aucune vérification, cohérent avec le reste de
l'application. Le **POST** (`post_update_roster`) doit en revanche être
gardé : réutilise le garde-fou déjà en place pour les autres mutations de ce
BC (`purchase_skill_controller.rs::can_spend_spp`,
`player_detail_controller.rs::check_admin_rights`) — coach de l'équipe, admin
d'espace, ou admin de compétition. Pas de nouveau mécanisme d'autorisation,
réutilisation à l'identique.

## Persistance

- **Migration** `migrations/<timestamp>_add_display_order_to_players_proj.sql`
  — `ALTER TABLE players_proj ADD COLUMN display_order INTEGER;` (nullable).
- **`IPlayerRepository::append_batch`** (nouvelle méthode, cf. 05-use-cases.md)
  — implémentation par défaut en boucle d'`append()` ; `PgPlayerRepository`
  la surcharge avec une transaction unique enveloppant `insert_player_event`
  + `upsert_player_projection` pour chaque événement du batch.
- **`insert_player_event`** (existant) : `event_type_name()` et
  `player_and_team_id()` gagnent chacun 3 nouvelles branches
  (`PlayerRenamed`/`PlayerJerseyChanged`/`PlayerReordered`) — mécanique,
  aucune nouvelle logique.
- **`upsert_player_projection`** (existant) : 3 nouvelles branches, chacune
  fait un `UPDATE players_proj SET personal_name = ... / jersey = ... /
  display_order = ... WHERE player_id = ...`.
- **`find_by_team_id`** (`projection_repository.rs:30`) : `SELECT` inchangé
  pour les colonnes déjà lues (`display_order` n'est pas nécessaire dans la
  `PlayerProjection` — cf. 02-front.md, l'ordre est porté par le tri SQL, pas
  par un champ affiché). Seul le tri change :
  `ORDER BY display_order NULLS LAST, jersey NULLS LAST, player_id`.

## Événements

Aucun app event, aucun nouveau listener cross-BC. `PlayerRenamed`/
`PlayerJerseyChanged`/`PlayerReordered` restent des faits purement internes
au BC `players` — rien d'autre dans l'application ne dépend du nom, du
numéro ou de l'ordre d'un joueur (contrairement à un achat de compétence, qui
change la valeur d'équipe). Mise à jour de projection dans la même
transaction que l'append (règle CLAUDE.md), pas de publisher à toucher.

## Handlers

### GET (existant, déplacé — cf. 03-back.md)

```rust
pub async fn player_table_widget(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse
```

Signature inchangée. Le template gagne juste `team_id` (déjà dans le `Path`,
simple passage supplémentaire) et `save_error: None`.

### POST (nouveau)

```rust
pub async fn post_update_roster(
    Path((space_id, team_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    axum::Form(form): axum::Form<RosterUpdateForm>,
) -> impl IntoResponse
```

Déroulé :
1. `auth_session.user` absent → `401`.
2. Autorisation (cf. ci-dessus) → `403` si refusée.
3. Tableaux `player_id`/`personal_name`/`jersey` de longueurs différentes →
   `400`.
4. Construction de `UpdateRosterCommand` ligne par ligne — un smart
   constructor qui échoue (nom invalide, numéro hors 1-99) → `400`.
5. Appel `update_roster_use_case::execute()`.
6. Succès → `PlayerTableTemplate { save_error: None, players: <effectif
   retourné>, .. }` + header `HX-Trigger: rosterEditSaved`.
7. `UpdateRosterError::UnknownOrInactivePlayer` /
   `DuplicateJersey` / `DuplicateDisplayOrder` / `Repository(ConcurrentWrite)`
   → `PlayerTableTemplate { save_error: Some(message), players: <construits
   depuis la commande soumise, pas la DB>, .. }` + header
   `HX-Trigger: rosterEditSaveFailed`, réponse **200** (fragment HTMX
   affichable, pas une erreur HTTP dure — le formulaire reste actionnable).
8. `UpdateRosterError::Domain(_)` / `Repository(_)` autre que
   `ConcurrentWrite` → log + `500`.

## Templates

`player-table-fragment.html` (existant, étendu) — pas de nouveau fichier
template. Ajouts (cf. 02-front.md pour le détail des événements) :
- `<form>` autour de `#roster-tbody`, `hx-post`, `hx-trigger="rosterEditSaveRequested from:body"`, `hx-target="this"`, `hx-swap="outerHTML"`.
- Racine `.player-table` : classe `edit-mode` conditionnelle
  (`{% if save_error.is_some() %}edit-mode{% endif %}`) — rendu server-side
  déjà en mode édition si on revient d'un échec, pas d'attente d'un toggle JS.
- Bandeau d'erreur inline si `save_error.is_some()`.
- Poignée de glisser-déposer, inputs nom/numéro — repris de la maquette
  (`app-team-detail.html`), adaptés au binding Askama (`p.jersey`,
  `p.personal_name`, `p.player_id` pour les attributs `name="jersey[]"` etc.).

## Tests E2E prévus (Playwright)

- Renommer un joueur, enregistrer, vérifier que le nom persiste après
  rechargement de la page.
- Changer un numéro de maillot, enregistrer, vérifier la persistance.
- Vider un numéro de maillot (le retirer), vérifier `— ` affiché en lecture.
- Réordonner deux joueurs par glisser-déposer, enregistrer, vérifier le
  nouvel ordre après rechargement.
- Saisir un numéro déjà pris par un autre joueur actif → Enregistrer
  désactivé, message de doublon visible (front, sans requête).
- Renvoyer un joueur puis attribuer son ancien numéro à un autre joueur actif
  → succès (règle 8, un `Dismissed` ne bloque rien).
- Changer l'état de l'équipe (sélecteur de démo hors périmètre e2e réel —
  ici : quitter la phase « Prête à jouer » par une vraie transition) pendant
  l'édition → mode édition fermé proprement, pas de perte d'état visible.
- Utilisateur sans droit (ni coach, ni admin) → bouton absent ou requête
  refusée (403).
