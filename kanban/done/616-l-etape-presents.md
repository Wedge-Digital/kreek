# L'étape « Présents »

**Épic :** E18 — Appariement des présents · **Ordre :** 5 · **Dépend de :** 615
**Conception :** `docs/specs/appariement-des-presents/calendrier/02-front.md`,
`04-dtos.md` (§ 1), `07-integration.md`
**Maquette :** `assets/rawpages/html/app-competition-admin-schedule-draw.html`, état 2

## Objectif

Sur une journée vide du Calendrier, « Faire les appariements » ouvre la liste
des équipes inscrites, poule par poule, toutes cochées (R2). L'organisateur
décoche les absentes. Le bouton « Tirer au sort » est rendu, désactivé : la carte
617 le branche.

## Conception

**Service de lecture** — `use_cases/admin/draw/draw_candidates_service.rs` :

```rust
// arch:no-instrument — service de lecture : assemble l'étape « Présents », sans intention métier
pub async fn load(round_id: &MatchId, season_id: &SeasonId, deps: &DrawDeps<'_>)
    -> Result<DrawCandidates, DrawCandidatesError>;
```

`load_draw_context` (615), `can_receive_draw()`, puis par poule les inscrites
avec leur nombre de matchs joués (`build_matches_played`).

**Handler** — `io/web/admin/schedule_draw_widget.rs` : `get_draw_widget`,
`Query<DrawQuery>` (`round_id`, `absent` en ids séparés par des virgules),
`require_admin_access` puis `journee_de_la_saison`. Refus → 409/422 et
`{ error }`.

**VMs** — `DrawPresenceVm`, `PresenceGroupVm`, `PresenceTeamVm`, avec
`DrawPresenceVm::from_candidates(&candidates, &absent)`.

**Gabarit** — `admin/widgets/schedule-draw-presence.html` : racine
`<div class="schedule-draw" hx-disinherit="*">`, **une équipe par ligne** (la
grille à deux colonnes de la première maquette se lisait comme des
appariements), case `name="team_ids"`, nom, coach · roster, nombre de matchs.
Un `x-data` Alpine par poule compte les cases et affiche « Nombre impair : une
équipe sera exemptée — celle qui a joué le plus de matchs » et « Moins de deux
équipes présentes : aucune rencontre ne sera tirée dans cette poule ».
« Toutes présentes » / « Toutes absentes » par poule. « Annuler » recharge le
détail de la journée.

**Point d'entrée** — `schedule-round-detail.html` : « 🙋 Faire les
appariements », seulement si `detail.fixtures` est vide, en `hx-get` ciblant la
zone de contenu de la journée — l'en-tête et les dates restent visibles.

**CSS** — `assets/static/css/widgets/schedule-draw.css`, styles de la maquette
scopés sous `.schedule-draw`, inscrit dans `src/web/css_bundle.rs` (axe 14).

**Route** — `admin_schedule_draw` dans `routes.rs`, branchée dans `router.rs`.

## Checklist

- [x] Service, handler, VMs, gabarit, route
- [x] Bouton d'entrée sur journée vide seulement
- [x] Feuille scopée et inscrite au bundle ; `scripts/check-css-collisions.sh`
- [x] Aucun `style=""`, aucun `id` global
- [x] Tests unitaires : le service refuse repos et journée pourvue ; une poule
      sans inscrite n'apparaît pas ; `from_candidates` restitue les absentes
- [x] `make lint`, `make check-arch`, `make test`
