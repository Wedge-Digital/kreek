# Accueil — Derniers résultats — Spec index

Rendre fonctionnelle la section "Derniers résultats" de la page d'accueil d'un
espace (BC `news`), aujourd'hui du HTML statique avec des données fictives.
Le BC `competitions` fournit un widget affichant les derniers matchs terminés
de l'espace, toutes compétitions/saisons confondues.

## Contexte

- Page d'accueil : `/app/{space_id}/home`, handler `get_news_feed`
  (`src/app/news/io/web/news_feed.rs:131`), template `news-feed.html:137-202`
  (bloc `.matches-panel` codé en dur, 4 résultats fictifs).
- Le BC `competitions` a déjà un modèle de lecture proche : projection
  `competition_match_display_proj` + `list_resultats.sql`, mais **scopée par
  saison** (`season_id`), sans `space_id` ni date réelle de publication — donc
  pas directement utilisable pour "tous les derniers résultats d'un espace,
  toutes compétitions confondues, triés chronologiquement".
- L'app event `MatchReportPublished` porte déjà `space_id` et `published_at`
  (`src/app/shared_kernel/app_events/match_report_app_events.rs:48-59`) — la
  projection doit être enrichie de ces deux colonnes.
- Le lien vers le rapport de match doit respecter la même autorisation que
  l'onglet Résultats existant (`resultats_view.rs:36-60` et
  `recap_controller.rs:235-249`) : admin d'espace, admin de la compétition, ou
  coach de l'une des deux équipes.

## Règles métier validées (référence partagée)

- Seuls les matchs de statut **completed** apparaissent (pas `in_progress`).
- **4 résultats** maximum affichés.
- Tri par **date réelle de publication** (`published_at`), décroissant, toutes
  compétitions/saisons de l'espace confondues.
- Chaque résultat est cliquable vers `match_report_url`, **uniquement si**
  l'utilisateur courant est autorisé (même règle que l'onglet Résultats).
- Cas d'égalité (match nul) : aucun score ne reçoit le highlight `winner`.
- Échec de chargement du widget : dégradation silencieuse — le serveur logue
  et renvoie le même fragment que l'état vide, pas d'erreur visible.

## Découpage en unités

| Unité | Portée | UI |
|---|---|---|
| `widget-derniers-resultats` | Widget `competitions` + intégration dans la page d'accueil `news` | Oui |

## Progression

| Unité | Mockup | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|---|
| widget-derniers-resultats | ✅ | ✅ | ✅ | ✅ | n/a | n/a | ✅ | ✅ |
