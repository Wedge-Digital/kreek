# Phase 3 — Architecture back (`detailed-standings`)

Organisation des traitements de l'onglet « Classement détaillé ». Entrée : `02-front.md`
validé.

## Mapping widget → BC

| Widget | BC fournisseur | Justification |
|---|---|---|
| Coquille d'onglet | `competitions` | Le BC possède la page de détail et sa barre d'onglets |
| Tableau détaillé | `ranking` | Les lignes de classement, les compteurs de départage et la logique de comparaison lui appartiennent |

Le handler d'onglet reprend le motif de `get_tab_standings`
(`competitions/io/web/competition_detail.rs:476`) : fragment seul si l'en-tête
`HX-Request` est présent, page complète avec `active_tab` sinon — ce qui permet
l'accès direct à l'URL comme la navigation HTMX.

## Routes

| Constante | Chemin | BC |
|---|---|---|
| `COMPETITION_TAB_DETAILED_STANDINGS` | `/app/{space_id}/competitions/{competition_id}/{season_id}/detailed-standings` | `competitions` |
| `DETAILED_STANDINGS_WIDGET` | `/app/{space_id}/ranking/{competition_id}/{season_id}/detailed-widget` | `ranking` |

La coquille référence la widget via `AppRoutes` (`app_routes.ranking.detailed_standings_widget(...)`),
jamais par un import direct du module de routes de `ranking`.

## Plan de fichiers

### À créer

| Fichier | Contenu |
|---|---|
| `ranking/io/web/widgets/detailed_standings_widget.rs` | Handler `detailed_standings_widget`, template struct, VMs |
| `ranking/io/web/templates/widgets/detailed-standings-widget.html` | Tableau, en-têtes à deux niveaux, légende, états vides et erreur |
| `assets/static/css/widgets/detailed-standings-widget.css` | Classes `.sd-*` reprises de la maquette |
| `competitions/io/web/templates/competition-tab-detailed-standings.html` | Conteneur `hx-get` vers la widget |

### À modifier

| Fichier | Modification |
|---|---|
| `ranking/domain/standings.rs` | `TiedGroup` + `tied_groups` (décision A) |
| `ranking/io/web/builders.rs` | Extraction de `split_into_groups` (décision B) + VMs du tableau détaillé |
| `ranking/io/web/tiebreak_labels.rs` | `tiebreak_short_label` (décision C) |
| `ranking/routes.rs`, `ranking/router.rs` | Route et enregistrement de la widget |
| `ranking/io/web/widgets/mod.rs` | Déclaration du module |
| `competitions/routes.rs`, `competitions/router.rs` | Route de l'onglet |
| `competitions/io/web/competition_detail.rs` | `get_tab_detailed_standings` + template struct |
| `competitions/io/web/templates/competition-detail.html` | Onglet dans la barre + branche `active_tab` |

## Ports

**Aucun port nouveau.** `IRankingCompetitionPort` couvre déjà les trois besoins :

| Besoin | Méthode existante |
|---|---|
| Noms des équipes inscrites | `find_enrolled_teams` |
| Poules et leurs équipes | `find_groups` |
| Barème et configuration de départage | `find_ranking_rules` (champ `tiebreakers`) |

## Domain services

**Aucun service nouveau.** `standings_service` est réutilisé tel quel :
`to_tiebreak_order` pour construire l'ordre, `build_ordered_standings` pour obtenir les
équipes ordonnées avec leur rang. Les deux onglets s'appuient sur les mêmes fonctions —
leur ordre d'affichage est donc identique par construction, et non par coïncidence.

## Décisions d'organisation

### A — R21 et R22 vivent dans le domaine

> ⚠️ **L'API décrite ci-dessous a été remplacée en phase 6** — voir `06-domaine.md`.
> `TiedGroup` attache le résultat à un groupe, ce qui désigne un critère décisif sur des
> lignes qu'il n'a pas départagées dès qu'un groupe de trois équipes en compte deux
> encore à égalité. `RowTiebreak`, attaché à **chaque ligne**, lui succède. Seule la
> décision de placer ces règles **dans le domaine** reste valide — c'est l'objet de cette
> section, l'esquisse d'API qui l'illustre ne l'est plus.

« Quel critère départage ce groupe ? » est une question métier, au même titre que « qui
est devant ? ». Elle ne descend ni dans le builder ni dans le service.

```rust
// ranking/domain/standings.rs

/// Équipes consécutives à égalité de points de classement, et le critère qui les
/// départage. `decisive: None` ⇒ tous les critères actifs donnent la même valeur,
/// c'est l'ex æquo total (règle 22).
pub struct TiedGroup {
    pub from: usize,
    pub len: usize,
    pub decisive: Option<TiebreakCriterion>,
}

/// Règle 21. Ne retourne que les groupes de **2 équipes ou plus**.
pub fn tied_groups(ordered: &[TeamStanding], order: &TiebreakOrder) -> Vec<TiedGroup>;
```

Deux points de conception :

- **Seuls les groupes de 2+** sont retournés. Une équipe seule n'a rien à départager, et
  un `decisive: None` sur un singleton se confondrait avec l'ex æquo total que R22 doit
  précisément rendre visible.
- **Aucune évolution de `compare`.** Les équipes à égalité de points sont déjà
  consécutives dans le tableau ordonné, les points étant la clé de tri primaire :
  `tied_groups` est un parcours de groupes appuyé sur `TiebreakCriterion::value_of`, sur
  des données déjà produites.

Le domaine dit quel critère tranche ; la présentation décide d'une couleur.

### B — Le découpage par poule est extrait

`build_classement_groups` porte aujourd'hui le découpage (poule unique ou absente, une
poule par groupe, section « Non assignées ») **mélangé au rendu** des lignes du classement
simple. Le nouvel onglet a besoin du même découpage avec un rendu différent.

```rust
// ranking/io/web/builders.rs

/// Une poule et les données qui la concernent — le découpage seul, sans rendu.
struct GroupSlice {
    title: Option<String>,
    lines: Vec<RankingLineRow>,
    teams: Vec<EnrolledTeamInfo>,
}

fn split_into_groups(
    lines: &[RankingLineRow],
    teams: &[EnrolledTeamInfo],
    groups: &[RankingGroupInfo],
) -> Vec<GroupSlice>;
```

Consommé par les deux constructeurs de groupes. Sans cette extraction, la règle « chaque
poule est un classement autonome » serait implémentée deux fois et pourrait diverger.

C'est une **refacto de code testé** : les six tests de découpage de `builders.rs`
(poule unique, poules multiples, poule vide, équipes non assignées, absence de section
non assignées) couvrent exactement ce comportement et doivent rester verts **sans
modification** — c'est le meilleur filet possible pour ce déplacement.

Les VMs du tableau détaillé restent dans `builders.rs` : ils dépendent des DTOs du port
pour les noms d'équipes, ce que le CLAUDE.md range explicitement dans ce fichier.

### C — Libellés courts des colonnes

Les en-têtes de la maquette sont courts (`Δ TD`, `TD+`, `Bl.`, `TD−`), le libellé long
passant en attribut `title`. `tiebreak_labels.rs` ne fournit aujourd'hui que les libellés
longs, et son unique consommateur est l'ACL du catalogue vers `competitions`
(`infrastructure/competitions/tiebreak_catalog_adapter.rs`).

`tiebreak_short_label(criterion) -> &'static str` s'ajoute à côté, sans toucher à
l'existant : le formulaire de règles continue d'afficher les libellés longs.

## Règles métier

**Aucune règle nouvelle à cette étape.** La phase 3 n'a fait que placer R21 et R22 dans
une couche — le domaine.

## Reporté en phase 4

Le formatage signé (`+2` / `+0` pour les bonus, `+14` / `−3` pour la différence de TD)
est de la présentation : il sera porté par le VM, pas par le domaine, dont `value_of`
retourne un `i64` nu.
