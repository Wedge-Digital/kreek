# Classement — Phase 2 : Architecture front

## Page hôte

`competition_detail.rs` (BC `competitions`) reste propriétaire :
- de la route de deep-link `COMPETITION_TAB_STANDINGS` (`/competitions/{cid}/{sid}/standings`)
- du shell de page (tab bar, `full_page()` pour le chargement direct, fragment pour le htmx)

Elle n'affiche plus aucune donnée de classement elle-même. Elle héberge le widget fourni par `ranking`, exactement comme `teams-team-detail.html` héberge déjà le widget joueurs de `players` (`vm.players_widget_url`, `hx-get` + `hx-trigger="load"` + placeholder de chargement) :

```html
<!-- competition-tab-standings.html — après cette feature -->
<div id="ranking-widget"
     hx-get="{{ app_routes.ranking.classement_widget(space_id, competition_id, season_id) }}"
     hx-trigger="load"
     hx-target="this"
     hx-swap="outerHTML">
  <div class="loading-placeholder">Chargement du classement…</div>
</div>
```

La page hôte n'émet et n'écoute aucun événement DOM pour ce widget — assemblage pur, zéro logique.

## Widgets

| Widget | BC | Endpoint GET | Trigger | Mode |
|---|---|---|---|---|
| Classement | ranking | `/app/{space_id}/ranking/{competition_id}/{season_id}/widget` | `load` (onglet actif par défaut) + reclic sur l'onglet Classement | Lecture seule |

Un seul widget pour cette feature. Aucune communication DOM avec d'autres widgets (lecture seule, rien à synchroniser).

## Séparation front / back

Rien de géré côté front hormis le chargement `hx-get` initial :
- Pas de tri interactif — toujours points de classement décroissants (confirmé Phase 1)
- Pas de filtre
- Pas de pagination / scroll infini — le classement complet est rendu en un seul fragment (contrairement à Résultats/Calendrier qui paginent par journée ; le nombre d'équipes d'une compétition reste borné)

## Widgets existants réutilisables

Aucun — première fonctionnalité du BC `ranking`. Le pattern de host cross-BC (`hx-get` + `hx-trigger="load"` + placeholder, isolation via `hx-disinherit="*"` sur la racine du widget) est repris de `teams-team-detail.html` → widget `players`.

## Ce que `competitions` doit céder

- Suppression de `mock_standings()`, de la struct `StandingRow`, et de toute génération de tableau de classement dans `competition_detail.rs`
- Le handler `get_tab_standings` devient un simple host : il ne calcule plus rien, il rend uniquement le wrapper `hx-get` ci-dessus (full page ou fragment selon `hx-request`)
- Referme la partie "Classement" de la carte kanban `13-mock-data-competition-detail.md`

## Règles métier identifiées

- L'onglet Classement reste l'onglet actif par défaut de la page détail compétition (déjà acté dans `docs/specs/competition-matchs/onglets-matchs/02-front.md`)
- Tri fixe par points de classement décroissants, aucun tri utilisateur (Phase 1)
- Colonne "Bonus" : **masquée dans cette feature**, quelles que soient les règles de la compétition. Le calcul des points bonus est hors scope feature 1 (seuls les points de classement victoire/nul/défaite sont calculés) — afficher une colonne à `0` serait trompeur (laisserait croire qu'aucune équipe n'a de bonus alors que le calcul n'existe simplement pas encore). La colonne sera introduite avec la feature qui livrera le calcul réel des points bonus (`CompetitionRules.ranking_rules.offensive_bonus.activated` / `.defensive_bonus.activated`, consultés via le port), pas avant.
- 2 états vides distincts : "Aucune équipe dans la compétition." (aucune équipe inscrite à la saison) vs "Aucun match n'a encore été joué." (équipes inscrites, zéro ligne de classement)
- 1 état d'erreur distinct : les règles de classement (`RankingRulesInfo`) ne sont pas configurées pour la saison → message d'erreur explicite (style `.table-error-zone`/`.table-error`, cf. `finalize-team.html`), pas un état vide. Le widget ne doit jamais afficher un classement à 0 partout si les règles manquent — mieux vaut une erreur visible qu'une donnée silencieusement fausse.
- **Une équipe n'a qu'une seule ligne "active" par saison à tout instant** : c'est la ligne de classement la plus récente par ordre d'enregistrement (pas par journée) qui fait foi pour l'affichage. Une équipe peut jouer plusieurs matchs dans la même journée (plusieurs lignes générées pour cette journée) ; une correction future de rapport de match ajoutera aussi une nouvelle ligne plutôt que de modifier l'ancienne. Le widget prend toujours la dernière ligne par équipe, sans déduplication ni notion d'idempotence à gérer.
