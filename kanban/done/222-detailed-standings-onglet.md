# Classement détaillé — Onglet complet, sans mise en évidence

**Priorité : haute**
**Dépend de :** carte 221 (`split_into_groups`, `tiebreak_order_of` partagés)
**Contexte :** `src/app/ranking/io/web/`, `src/app/ranking/routes.rs`, `src/app/ranking/router.rs`, `src/app/competitions/io/web/competition_detail.rs`, `src/app/competitions/routes.rs`, `assets/static/css/widgets/`
**Spec :** `docs/specs/ranking/tiebreakers/detailed-standings/{02-front,03-back,04-dtos,07-integration}.md`

## Objectif

Livrer l'onglet « Classement détaillé » **sans la mise en évidence du critère décisif**
(carte 223). Il affiche déjà chaque nombre composant le total et les compteurs de
départage dans l'ordre de priorité : utile en l'état.

Carte **atomique** : la route, le handler, le template et l'onglet hôte n'ont aucun sens
livrés séparément.

## Conception

### Routes

| Constante | Chemin | BC |
|---|---|---|
| `COMPETITION_TAB_DETAILED_STANDINGS` | `/app/{space_id}/competitions/{competition_id}/{season_id}/detailed-standings` | `competitions` |
| `DETAILED_STANDINGS_WIDGET` | `/app/{space_id}/ranking/{competition_id}/{season_id}/detailed-widget` | `ranking` |

La coquille référence la widget via `AppRoutes`, jamais par un import direct des routes de
`ranking`.

### Handlers

`get_tab_detailed_standings` (`competitions`) — calqué sur `get_tab_standings` : fragment
si `HX-Request`, page complète avec `active_tab = "detailed-standings"` sinon, `400` sur
identifiant invalide.

`detailed_standings_widget` (`ranking`) — calqué sur `classement_widget` : `401` sans
utilisateur, `competition_id` ignoré, `build_vm` privé chargeant les quatre sources en
`tokio::join!`. **`build_vm` doit rester sous 20 lignes** : extraire la construction des
colonnes et celle des groupes en fonctions nommées.

### VMs (cf. `04-dtos.md`)

`DetailedStandingsVm { rules_missing, columns, groups }`, `TiebreakColumnVm { position,
short_label, long_label }`, `DetailedGroupVm`, `DetailedRowVm`, `TiebreakCellVm { value,
state }`, `CellState { Decisive, Tied, Neutral }`.

**`CellState` est introduit dès cette carte, avec `Neutral` partout.** La 223 se réduit
alors à le peupler et à ajouter deux classes CSS, au lieu de traverser VM, builder,
template et CSS d'un coup — ce qui irait contre l'intérêt d'une carte reportable.

`tiebreak_short_label` s'ajoute à `tiebreak_labels.rs` sans toucher aux libellés longs,
dont l'ACL du catalogue vers `competitions` dépend.

### Formatage

| Colonne | Format |
|---|---|
| Bonus | toujours signé — `+2`, `+0` |
| Δ TD | signé — `+14`, `−3` |
| Autres critères | brut |

**Signe moins typographique `−` (U+2212)**, pas le trait d'union ASCII.

### Template et états

Structure alignée sur `classement-widget.html` : feuille de style embarquée, racine en
`hx-disinherit="*"`, trois états repris à l'identique —

| Condition | Rendu |
|---|---|
| `rules_missing` | « Impossible d'afficher le classement détaillé : les règles de classement ne sont pas configurées pour cette saison. » |
| `!has_enrolled_teams` | « Aucune équipe dans la compétition. » |
| `rows.is_empty()` | « Aucun match n'a encore été joué — tous les compteurs sont à zéro. » |

Tableau enveloppé dans `.sd-scroll` en `overflow-x: auto` : de 1 à 7 colonnes de départage
s'ajoutent aux 8 fixes, c'est le conteneur qui défile, jamais le `body`.

**Trophée dans la cellule de l'équipe** (`🏆 Nom`), le rang reste nu — c'est la maquette
qui fait foi, alors que le widget Classement le place dans la cellule du rang. Divergence
assumée, cf. `07-integration.md`.

**Légende partielle** : cette carte n'explique que Bonus et Total. Les phrases sur la mise
en évidence et l'ex æquo arrivent avec la 223 — sinon l'onglet décrirait des couleurs qui
n'existent pas encore.

### CSS

`assets/static/css/widgets/detailed-standings-widget.css`, classes `.sd-*` reprises de la
maquette. Les deux classes de mise en évidence relèvent de la 223.

## Tests

- VMs : colonnes construites depuis l'ordre actif (nombre, ordre, numérotation)
- Formatage signé, dont le signe typographique et le `+0` du bonus
- Les trois états
- Découpage par poule : rangs repartant à 1 dans chaque poule (via `split_into_groups`)

## Checklist

- [ ] Routes et enregistrement dans les deux routers
- [ ] Les deux handlers, `build_vm` ≤ 20 lignes
- [ ] VMs avec `CellState::Neutral` partout
- [ ] `tiebreak_short_label` ajouté sans toucher aux libellés longs
- [ ] Template, trois états, `.sd-scroll`, trophée dans la cellule équipe
- [ ] CSS `.sd-*`, sans les classes de mise en évidence
- [ ] Légende limitée à Bonus et Total
- [ ] Onglet ajouté dans `competition-detail.html` + branche `active_tab`
- [ ] Aucun style inline dans le template
- [ ] `make test` + `make check-arch` passent
