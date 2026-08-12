# BC `players` — Endpoint de sauvegarde du roster (POST)

**Priorité : haute**
**Dépend de :** `292-players-roster-edit-use-case.md`, `293-players-roster-edit-widget.md`
**Contexte :** `players` — controller HTTP

## Objectif

Exposer `update_roster_use_case` en HTTP : parsing du formulaire batch,
autorisation (réutilisation du garde-fou existant, pas de nouveau
mécanisme), traduction du résultat en réponse HTMX (succès ou échec avec
valeurs soumises réaffichées).

**Spec de référence :** `docs/specs/player-edition/team-detail/04-dtos.md`,
`07-integration.md`.

---

## Conception

### DTO brut (`players/io/web/roster_edition_controller.rs`)

```rust
#[derive(Deserialize)]
pub struct RosterUpdateForm {
    pub player_id: Vec<String>,
    pub personal_name: Vec<String>, // "" si vide → None
    pub jersey: Vec<String>,        // "" si vide → None
}
```

### Handler

```rust
pub async fn post_update_roster(
    Path((space_id, team_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    axum::Form(form): axum::Form<RosterUpdateForm>,
) -> impl IntoResponse
```

1. `auth_session.user` absent → `401`.
2. Autorisation — **réutilise** le garde-fou déjà en place
   (`purchase_skill_controller.rs::can_spend_spp` /
   `player_detail_controller.rs::check_admin_rights` : coach de l'équipe,
   admin d'espace, ou admin de compétition) → `403` si refusée.
3. Tableaux de longueurs différentes → `400`.
4. Construction de `UpdateRosterCommand` ligne par ligne (`display_order` =
   index) — un smart constructor qui échoue → `400`.
5. `update_roster_use_case::execute()`.
6. Succès → `PlayerTableTemplate { save_error: None, players: <effectif
   retourné>, .. }` + header `HX-Trigger: rosterEditSaved`.
7. `UnknownOrInactivePlayer`/`DuplicateJersey`/`DuplicateDisplayOrder`/
   `Repository(ConcurrentWrite)` → **200**, `PlayerTableTemplate { save_error:
   Some(message), players: <construits depuis la commande soumise, pas la
   DB> }` + header `HX-Trigger: rosterEditSaveFailed`.
8. `Domain(_)` / autre `Repository(_)` → log + `500`.

### Routes/router

`players/router.rs` : `.route(path::PLAYERS_ROSTER_UPDATE, post(post_update_roster))`.

---

## Checklist

- [x] `RosterUpdateForm`
- [x] `post_update_roster` — parsing + validation de longueur
- [x] Autorisation réutilisée (pas de nouveau mécanisme)
- [x] Construction `UpdateRosterCommand` avec smart constructors
- [x] Branche succès (`HX-Trigger: rosterEditSaved`)
- [x] Branche échec métier — 200, valeurs soumises réaffichées, pas la DB (`HX-Trigger: rosterEditSaveFailed`)
- [x] Branche erreur technique — 500 + log
- [x] Wiring `router.rs`

---

## Notes d'implémentation

**La carte prescrivait un extracteur qui ne pouvait pas fonctionner.**
`axum::Form` s'appuie sur `serde_urlencoded`, qui refuse les clés répétées :

```
Err(Error("invalid type: string \"a\", expected a sequence"))
```

Le handler aurait compilé et répondu **422 à chaque soumission**, sans que rien
ne le signale avant l'exécution. Résolu par `axum-extra` (feature `form`), dont
l'extracteur s'appuie sur `serde_html_form` — une dépendance ajoutée, le reste
de la carte inchangé.

**`build_player_rows` extrait du widget.** L'endpoint rend le même tableau que
le GET ; sans extraction, la résolution des caractéristiques et des compétences
aurait été dupliquée.

**Le réaffichage de la saisie est réel.** Après un refus, les lignes portent ce
que le coach a tapé, dans l'ordre du formulaire — pas ce que contient la base.
Les colonnes en lecture seule restent celles de la base : elles ne sont pas
éditables. Un joueur absent du formulaire reste affiché plutôt que masqué.

## Vérification navigateur

Parcours complet exercé sur une équipe « Prête à jouer » :

- **Succès** — renommage (apostrophe comprise) + maillot 42 : `HX-Trigger:
  rosterEditSaved`, sortie du mode édition, valeurs affichées et persistées.
- **Refus métier** — doublon posé en contournant la garde front (le cas de
  concurrence entre deux onglets) : **200**, `rosterEditSaveFailed`, message
  « deux joueurs portent le même numéro », **mode édition maintenu**, saisie
  conservée, doublon resurligné.
- **Diff par champ** — première sauvegarde : 1 `PlayerRenamed`, 1
  `PlayerJerseyChanged`, 11 `PlayerReordered` (aucun joueur n'avait encore de
  rang). Enregistrement correctif puis enregistrement à vide : **aucun nouvel
  événement**.
