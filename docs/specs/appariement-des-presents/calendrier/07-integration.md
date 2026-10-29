# Calendrier · appariement des présents — intégration

## Persistance

**Aucune migration** : `competition_match_day_pairings` et
`competition_match_display_proj` existent.

**`save_pairings`** — copié de la branche `sondage-presence` (sa carte 509) :

1. `SELECT id FROM competition_match_days WHERE id = $1 FOR UPDATE` — le verrou
   porte sur la journée, puisqu'il n'y a rien à verrouiller dans une table
   d'appariements vide ;
2. refus `PairingsAlreadyExist` si la journée porte déjà une rencontre ;
3. chaque appariement **et sa projection** dans la même transaction.

`save_pairing` reste pour l'ajout manuel et le déplacement ; les deux partagent
la fonction privée `write_pairing` (copiée de `ecrire_appariement`).

**Test d'intégration** : `test_save_pairings.rs`, copié — tout est écrit ; une
journée pourvue est refusée ; un refus n'écrit rien.

## Événements

Aucun nouveau. `PairingCreated` est émis par `emettre()` **après le commit**,
une fois par rencontre. Son passage en app event par le publisher, et sa reprise
par `match_report` qui crée le rapport de match, existent déjà. `to_app_event`
n'est pas modifié.

## Handlers — `io/web/admin/schedule_draw_widget.rs`, `schedule_draw_actions.rs`

Forme des handlers du Calendrier : retour `Response`, `require_admin_access`
puis `journee_de_la_saison(round_id)` en tête.

```rust
pub async fn get_draw_widget(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    Query(query): Query<DrawQuery>,
    State(state): State<AppState>,
) -> Response;

pub async fn post_draw_preview(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum_extra::extract::Form(form): axum_extra::extract::Form<PreviewDrawForm>,
) -> Response;

pub async fn post_draw_confirm(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum_extra::extract::Form(form): axum_extra::extract::Form<ConfirmDrawForm>,
) -> Response;
```

| Cas | Réponse |
|---|---|
| Succès d'un `GET` ou d'un aperçu | le fragment HTML, swappé dans le widget |
| Refus | 409 ou 422 et `{ error }`. HTMX ne swappe pas un 4xx ; `hx-on::after-request` le passe à `handleScheduleActionResponse`, qui affiche un toast d'erreur |
| Confirmation réussie | `HX-Trigger: scheduleChanged` et `{ success: "5 rencontres écrites · Plague Bringers exemptée · Dwarf Giants absente" }` — la phrase est composée par le handler : c'est de la présentation |

`handleScheduleActionResponse` (`templates/admin/schedule.html`) gagne deux
lignes : `data.success` devient un toast de succès. Elle ne savait dire que des
refus et des avertissements.

## Templates et CSS

| Fichier | Contenu |
|---|---|
| `admin/widgets/schedule-draw-presence.html` | racine `<div class="schedule-draw" hx-disinherit="*">` ; une case par équipe ; un `x-data` Alpine par poule compte les cases et affiche « impair » et « moins de deux présents » |
| `admin/widgets/schedule-draw-preview.html` | rencontres, exemptée, absentes ; champs cachés `pairings`, `exempted`, `team_ids` pour « Retirer au sort » et « Écrire » |
| `admin/widgets/schedule-round-detail.html` — modifié | « Générer une journée complète » et « Faire les appariements », seulement si `detail.fixtures` est vide ; le second en `hx-get` vers le widget, ciblant la zone de contenu de la journée |
| `admin/schedule.html` — modifié | le toast de succès |
| `assets/static/css/widgets/schedule-draw.css` | styles de la maquette, scopés sous `.schedule-draw`, inscrits dans `css_bundle.rs` |

**Routes** : trois entrées dans `routes.rs` et `router.rs`, à côté de
`generate-round`.

## Tests e2e — `tests/e2e/test_schedule_draw.py`

1. Journée vide : les deux boutons ; journée pourvue : aucun (R9).
2. Décocher une équipe met à jour le compteur et la note « impair » ; moins de
   deux présents partout désactive « Tirer au sort » (R2, R10).
3. L'aperçu n'écrit rien : après « Annuler », la journée est toujours vide.
4. « Modifier les présents » conserve les décochages.
5. Écrire : toast de succès, rencontres listées, absentes sans match, l'exemptée
   est celle qui a le plus joué (R8).
6. Une rencontre écrite se supprime, puis s'ajoute à la main (R13).
7. Journée remplie entre l'aperçu et l'écriture : toast d'erreur, rien n'est
   écrit (R11).
8. Deux équipes d'un même coach ne se rencontrent jamais (R4).
9. « Générer une journée complète » ne laisse aucune équipe sans match hors
   l'exemptée — régression du défaut de `generate_round_pairings`.

`tests/impact-map.toml` est mis à jour avec le fichier (skill `test-impact`).

## Règles métier ajoutées à cette étape

Aucune.
