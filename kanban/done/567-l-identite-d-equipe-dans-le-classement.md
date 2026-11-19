# L'identité d'équipe dans le classement

**Priorité : moyenne**
**Épic :** aucune — livrable d'un bloc avec la 566
**Dépend de :** 566
**Maquette :** `assets/rawpages/html/app-competition-standings-teams.html`
**Fichiers :**
`src/app/ranking/ports.rs`, `src/app/ranking/context.rs`,
`src/app/ranking/io/web/builders.rs`,
`src/app/ranking/io/web/widgets/classement_widget.rs`,
`src/app/ranking/io/web/widgets/detailed_standings_widget.rs`,
`src/app/ranking/io/web/templates/widgets/classement-widget.html`,
`src/infrastructure/ranking/team_links_adapter.rs` *(nouveau)*,
`src/infrastructure/ranking/mod.rs`, `src/main.rs`,
`src/app/ranking/io/app_events/tests/test_match_report_published_pipeline.rs`,
`assets/static/css/widgets/ranking-classement-widget.css`,
`tests/e2e/test_classement_identite_equipe.py` *(nouveau)*,
`tests/e2e/test_manual_ranking_points.py`, `tests/impact-map.toml`

## L'objectif

Dans l'onglet Classement, chaque équipe s'affiche par le widget d'identité de
`teams` (carte 566) : logo, nom, badge de statut, roster · coach. `ranking` ne
sait pas que ce widget vient de `teams`.

## Ce qui l'a fait naître

Le classement ne montre que des noms, là où les Résultats montrent qui joue
quoi et qui coache.

Et `ranking` construisait déjà ses liens d'équipe par
`AppRoutes::default().teams.team_detail(...)` (`builders.rs`, deux endroits) :
il connaissait les routes de `teams`. Y ajouter l'adresse d'un widget aurait
violé la règle 1 des widgets — un BC ne référence pas le widget d'un autre.

## Le changement

**Les adresses d'équipe sont injectées.** `ranking/ports.rs` déclare
`IRankingTeamLinksPort` — `team_detail_url(space_id, team_id)` et
`team_identity_url(space_id, team_id)`. L'adapter vit dans
`src/infrastructure/ranking/team_links_adapter.rs`, seul à appeler
`AppRoutes.teams` ; `main.rs` l'injecte dans `RankingContext`.

**Les builders ne connaissent plus `AppRoutes.teams`.** Le `space_id` qu'ils
recevaient devient un `TeamUrls` — l'espace et le port — qui sert `team_link`
au classement simple comme au détaillé, et `team_identity_url` au simple.

**Le classement simple compose.** La case Équipe charge le widget
(`hx-get` + `hx-trigger="load"`) et affiche le nom de l'équipe en attendant :
jamais de case vide, pas de saut de ligne. Le trophée du premier passe dans la
case du rang.

**Le classement détaillé ne change pas d'aspect** : seul son `team_link` passe
par le port.

**Mobile (768px)** : toutes les colonnes restent. Le widget prend la première
ligne sur toute la largeur, les chiffres — Pts compris — passent dessous,
alignés sur les en-têtes ; le rang couvre les deux lignes.

## Ce que la carte ne couvre pas

- Le widget dans le classement détaillé : onglet de vérification, déjà large.
- Une requête groupée pour toutes les équipes : une requête par équipe suffit
  pour 8 à 16 équipes.

## Tests

Unitaires : les builders avec un port factice — `team_link` et
`team_identity_url` viennent du port, aucune adresse de `teams` n'est écrite
en dur dans `ranking`.

E2E, `test_classement_identite_equipe.py` : sur l'onglet Classement d'une
saison jouée, chaque ligne affiche le widget (logo ou initiales, coach, badge
de statut) ; à 375px de large, la page ne défile pas horizontalement et les six
colonnes chiffrées restent visibles.
`test_manual_ranking_points.py` lit désormais le nom dans le widget.

## Terminé quand

L'onglet Classement montre, pour chaque équipe, son logo, son nom, son statut
et son coach, sur desktop comme sur mobile, et `grep -rn "AppRoutes" src/app/ranking`
ne renvoie plus aucune route de `teams`.
