# BC `players` — Widget joueurs : mode édition (front)

**Priorité : haute**
**Dépend de :** `290-players-roster-edit-domain.md`
**Contexte :** `players` — widget HTMX

## Objectif

Étendre le widget joueurs existant pour porter le mode édition (nom,
numéro, ordre) maquetté et validé dans `app-team-detail.html` : bascule
lecture/édition, glisser-déposer, validation de doublon en direct, bandeau
d'erreur. Comble aussi la dette de convention sur ce fichier (fortement
modifié → doit suivre `_widget.rs` sous `widgets/`).

**Spec de référence :** `docs/specs/player-edition/team-detail/02-front.md`,
`03-back.md`, `04-dtos.md`. **Maquette de référence (copier-coller
obligatoire, règle 5 CLAUDE.md) :**
`assets/rawpages/html/app-team-detail.html`.

---

## Conception

### Renommage

`players/io/web/player_table.rs` → `players/io/web/widgets/player_table_widget.rs`
(copié-collé intégral, pas de réécriture). Mettre à jour l'import dans
`players/router.rs`.

### `PlayerTableTemplate` — extension

```rust
pub struct PlayerTableTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub team_id: String,            // nouveau
    pub players: Vec<PlayerRowVm>,  // inchangé
    pub save_error: Option<String>, // nouveau
}
```

`PlayerRowVm` : aucun changement — `jersey`/`personal_name` déjà présents.

### Route (`players/routes.rs`)

```rust
pub const PLAYERS_ROSTER_UPDATE: &str = "/app/{space_id}/players/by-team/{team_id}/roster";
pub fn update_roster(&self, space_id: &str, team_id: &str) -> String { ... }
```

Uniquement la constante + le builder d'URL dans cette carte — le handler
POST et son wiring dans `router.rs` sont pour la carte suivante (294). La
constante doit exister pour que le template compile
(`app_routes.players.update_roster(...)`).

### Template (`player-table-fragment.html`)

Repris **tel quel** de la maquette validée (règle 5 CLAUDE.md — copier-coller,
pas de réécriture de mémoire), adapté au binding Askama :
- `<form>` autour de `#roster-tbody`, `hx-post="{{ app_routes.players.update_roster(space_id, team_id) }}"`, `hx-trigger="rosterEditSaveRequested from:body"`, `hx-target="this"`, `hx-swap="outerHTML"`.
- Racine `.player-table` : `{% if save_error.is_some() %}edit-mode{% endif %}` — déjà en mode édition au premier rendu si on revient d'un échec.
- Bandeau d'erreur inline si `save_error.is_some()`.
- Poignée de glisser-déposer, cellules `#`/Nom en `display-value`/`edit-value` (input), colonnes restantes `cell-readonly` — markup et JS (`toggleRosterEdit`, `onJerseyInput`, `enableRosterDragAndDrop`, événements DOM) copiés de la maquette, adaptés pour écouter `rosterEditRequested`/`rosterEditCancelRequested from:body` au lieu des fonctions globales de démo.
- Boucle Askama sur `players` pour générer les lignes (`name="jersey[]"`/`name="personal_name[]"`/`name="player_id[]"` en hidden input par ligne).

---

## Checklist

- [x] Renommage `player_table.rs` → `widgets/player_table_widget.rs` (import `router.rs` mis à jour)
- [x] `PlayerTableTemplate` : `team_id` + `save_error`
- [x] `PLAYERS_ROSTER_UPDATE` + `Routes::update_roster()`
- [x] Template : formulaire + listeners `hx-trigger` DOM
- [x] Template : classe `edit-mode` conditionnelle sur `save_error`
- [x] Template : bandeau d'erreur inline
- [x] Template : glisser-déposer, inputs nom/numéro (repris maquette)
- [x] Vérifier au navigateur : mode édition identique à la maquette validée
- [x] **Hors carte initiale** — bandeau d'état (BC `teams`) : boutons
      Modifier/Annuler/Enregistrer et publication des trois événements DOM

---

## Notes d'implémentation

**Le bandeau d'état a été fait dans cette carte.** Il n'était couvert par aucune
des six cartes 290-295 ni par `08-cards.md`, alors que `02-front.md` lui confie
l'émission des trois déclencheurs. Sans lui, le widget savait écouter mais
personne ne parlait : le mode édition était inatteignable. Nouveau variant
`BannerCtaVm::RosterEdit`, sans URL — le bandeau ne connaît ni la route ni le
DOM du widget, il publie sur `body` et écoute ce que le widget y publie en
retour. Les deux BCs ne se référencent jamais.

**Les `style="…"` de la maquette sont devenus des classes CSS.** La règle 5
impose le copier-coller depuis la maquette, mais le CLAUDE.md interdit
totalement les styles inline dans les templates. Les deux règles se
contredisaient ; l'interdiction l'emporte, et le rendu est identique.

**Le `<form>` enveloppe le tableau entier, pas le seul `<tbody>`** comme le
disait la spec : un `<form>` enfant direct de `<table>` est du HTML invalide
que les navigateurs déplacent hors du tableau, emportant les champs avec lui.
Les inputs des cellules appartiennent au formulaire de la même façon.

**`hx-target="closest .players-widget"`** plutôt qu'un id : le widget ne doit
rien savoir du DOM de la page hôte (règle 4 des widgets). Le `<link>` CSS est
passé **à l'intérieur** de la racine, faute de quoi il s'accumulerait à chaque
échange `outerHTML`.

## Vérification navigateur

Faite sur une équipe « Prête à jouer » de l'espace E2E : bascule lecture ↔
édition, poignées et champs conformes à la maquette, doublon de numéro signalé
en rouge sur les deux lignes concernées avec « Enregistrer » désactivé,
correction rétablissant l'état valide, et annulation restaurant la saisie
d'origine en sortant du mode édition.

L'enregistrement lui-même n'a pas été exercé : l'endpoint POST est la carte 294.
