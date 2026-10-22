# Page « Imprimer » — Phase 3 : architecture back

Entrée : `02-front.md`. Toutes les routes sont des `GET` : aucune commande,
aucun événement, aucune projection.

## Qui fournit quoi

| Fragment | BC | Route | Réutilise | À créer |
|---|---|---|---|---|
| Page « Imprimer » | teams | `/app/{space_id}/teams/{team_id}/print` | `GamePhase::ReadyToPlay` (R1) | contrôleur, gabarit |
| Aperçu (assemblage) | teams | `…/teams/{team_id}/print/preview` · `…/print/preview/with/{other_team_id}` | `AppRoutes` | contrôleur qui fabrique les URLs des fragments |
| En-tête de fiche | teams | `…/teams/{team_id}/print/sheet-head` | agrégat `Team`, VE persistée (`team.team_value`), `IRosterCatalogPort` | VM d'impression, prix du staff **lus dans le catalogue** |
| Joueurs | players | `…/players/by-team/{team_id}/print` | `build_player_rows`, `spp_remaining` (R5) | gabarit en lecture seule, marque « journalier » (R6) |
| Annexe des compétences | players | `…/players/print/skills/{team_id}` · `…/with/{other_team_id}` | `ISkillCatalogPort` (nom, catégorie, description) | service de regroupement sans doublon |
| Prochain adversaire | competitions | `…/competitions/teams/{team_id}/print/next-opponent` | `ITeamInfoPort::find_team_enrollment`, `list_team_matches` | fragment de l'option |
| Coups de pouce et star players | match_report | `…/match-report/print/inducements/{team_id}` · `…/with/{other_team_id}` | `ICompetitionDataPort::find_tier_rules_for_roster` (palier, prix, disponibilité) | port vers references (libellés, descriptions, fiches de star players) |
| Feuille de match | match_report | `…/match-report/print/sheet/{team_id}` · `…/with/{other_team_id}` | `ITeamDataPort` (VE, fans, roster) | fonctions pures favori / petite monnaie ; liste des joueurs alignables ; barème SPP ; port vers les rappels |
| Rappels du corpus | references | — | chargeur du corpus | lecture **optionnelle** de `match_reminders_fr.json` |

Chaque fragment vit dans son BC : `widgets/<nom>_widget.rs`,
`templates/widgets/<nom>.html`, route dans `routes.rs`, câblage dans
`router.rs`. Les URLs inter-BC passent par `AppRoutes`.

## Décisions

1. **Sécurité.** Toute route qui reçoit une seconde équipe la prend **dans le
   chemin**, sous le nom `{other_team_id}`. Un nouveau résolveur
   `ISpaceOwnership` pour ce paramètre (`src/infrastructure/teams/space_ownership.rs`,
   enregistré dans `main.rs`) fait vérifier par `space_scope` son appartenance à
   l'espace — 404 sinon. Aucun fragment ne reçoit d'identifiant d'équipe en
   query.
2. **Favori et petite monnaie** : extraits de l'agrégat `MatchReportPreMatch`
   (`topdog_team_id`, `petty_cash`) en **fonctions pures du domaine**, par
   copier-coller (règle 5). L'agrégat et l'impression les appellent : ils ne
   peuvent pas diverger. Les `unwrap()` actuels deviennent des cas explicites.
3. **Disponibilité des star players** : le filtre `availableForRosters`, privé
   au contrôleur `inducement_selector_controller.rs:183-188`, rejoint le
   domaine de references à côté de `est_disponible_pour`, par copier-coller.
   Le sélecteur et l'impression appliquent la même règle.
4. **Masquer le chrome à l'impression** : c'est `layout-app.css`, feuille
   globale du layout, qui s'efface sous `@media print` quand la page porte une
   vue d'impression (`.app-layout:has(.team-print)`). *Révisé en
   implémentation (carte 595)* : la décision initiale — un
   `{% block body_class %}` sur `<body>` — ne tenait pas, la navigation htmx ne
   remplaçant que `#app-content` : la classe n'aurait suivi qu'un chargement
   complet. Le layout efface son propre chrome, et le gabarit du layout reste
   intact.
5. **Lecture seule** : les services de lecture de `use_cases/` portent
   `// arch:no-instrument — <motif>`.
6. **Corpus** : `match_reminders_fr.json` est lu par un helper de lecture
   optionnelle (absent → aucun rappel, pas d'erreur de démarrage) ; version
   fictive dans `assets/references.example/`. `docs/reference-data-schema.md`
   documente le fichier **et** corrige ses lignes fausses : le chargeur exige
   douze fichiers et non neuf, `spp_rules.json` compris.

## Ce que le code a corrigé dans la spec

- **R2 réécrite** : une équipe n'est inscrite qu'à une compétition et une
  saison (`Team.competition_id` et `season_id` uniques). Le prochain
  adversaire est celui du prochain match de cette saison.
- **Pas de soulignement des caractéristiques réduites** : la projection
  cumule séquelles, achats et personnalisations dans un seul delta. La colonne
  « Bless. » imprime le nombre de blessures persistantes, comme la fiche.
- **La trésorerie est déjà publique** sur la fiche : l'option R8 est de
  présentation, pas de confidentialité.

## Règles métier ajoutées à cette phase

| # | Règle |
|---|---|
| R10 | Le favori est l'équipe à la VE la plus forte. À égalité, c'est l'équipe à domicile du prochain match ; pour une « autre équipe », sans match connu, c'est mon équipe. |
| R11 | La petite monnaie imprimée est l'écart de VE ; ce que dépensera le favori n'est pas connu à l'impression, la feuille le rappelle en texte. |
