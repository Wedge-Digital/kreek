# Page « Imprimer » — Phase 5 : use cases

Entrée : `04-dtos.md`. La fonction **ne mute rien** : aucune commande, aucun
agrégat modifié, aucun événement émis. Il n'y a donc pas de use case au sens du
workflow (un par mutation), mais des **services de lecture** dans `use_cases/`,
qui orchestrent ports et domaine pour produire les VMs.

Chacun porte `// arch:no-instrument — service de lecture : assemble une vue
d'impression, sans intention métier` (axe 11). Aucun ne connaît HTTP ; les
contrôleurs traduisent leurs erreurs en réponses.

## teams

### `team_print_access_service::check(team_id) -> Result<Team, TeamPrintError>`

1. Charger l'agrégat `Team` (`ITeamRepository::find_by_id`).
2. Demander à l'agrégat s'il peut être imprimé (R1, méthode domaine en phase 6).
3. Rendre l'agrégat, que la page et l'aperçu réutilisent.

Appelé par la page « Imprimer » et par l'aperçu.

### `team_sheet_head_service::load(team_id, role, treasury) -> Result<TeamSheetHeadVm, TeamPrintError>`

1. `ITeamRepository::find_by_id`.
2. `IRosterCatalogPort::find_catalog(roster_id)` — prix de la relance et du
   staff (point E).
3. `ISquadPort::find_squad(team_id)` — alignables (`presence.alignable()`) et
   effectif.
4. `builders::team_sheet_head(...)`.

## players

### `player_print_service::rows(team_id) -> Vec<PlayerPrintRowVm>`

Réutilise `build_player_rows`, qui transmet en plus `membership` (R6).
Le service ajoute les lignes vides et applique le mode des indisponibles ;
l'appréciation « indisponible » reste celle du domaine (`absence`).

### `skills_annex_service::collect(team_ids, layout) -> SkillsAnnexVm`

1. Lire les joueurs de chaque équipe (`find_alive_by_team_id`).
2. Réunir leurs compétences de base (par le poste) et acquises.
3. Les résoudre par `ISkillCatalogPort::find_skill` — nom, catégorie,
   description.
4. Déléguer au domaine le dédoublonnage, le tri et le découpage en pages
   (phase 6).

## competitions

### `next_opponent_service::find(team_id) -> Result<NextOpponentOptionVm, NextOpponentError>`

1. `ITeamInfoPort::find_team_enrollment(team_id)` — aucune inscription :
   `Unavailable` (« L'équipe n'est inscrite dans aucune compétition »).
2. `IMatchDayRepository::list_team_matches(season_id, team_id)`.
3. Demander au domaine le prochain match et l'adversaire (phase 6) — aucun :
   `Unavailable` (« Aucun match à venir pour mon équipe dans cette compétition »).

## match_report

### `inducements_print_service::build(team_ids, layout, flags) -> Result<InducementsPrintVm, PrintError>`

Pour chaque équipe :

1. `ITeamDataPort::find_team_info` — nom, `roster_id`.
2. `ITeamDataPort::find_team_season` — saison d'inscription.
3. `ICompetitionDataPort::find_tier_rules_for_roster(season_id, roster_id)` —
   palier, prix par roster, disponibilité.

Puis :

4. `IReferenceCatalogPort::find_inducement_labels` sur l'union des uids ;
   `find_star_players` par roster.
5. Assembler une ligne par coup de pouce, un prix par équipe (R7).

Une équipe sans saison ou sans palier : sa colonne est imprimée vide, avec la
mention « palier introuvable », plutôt qu'un catalogue inventé.

### `match_sheet_service::build(own_id, other_id, home, round_id) -> Result<MatchSheetVm, PrintError>`

Pour chaque équipe :

1. `ITeamDataPort::find_team_info`, `find_team_value` — VE courante (R4).
2. `IPlayerDataPort::list_alignable_players` — journaliers compris.
3. `IReferenceCatalogPort::find_spp_scale(roster_id)`.

Puis :

4. Statut financier par les **fonctions pures** du domaine (R10, R11).
5. `round_id` : `ICompetitionDataPort::find_round_context(season_id, round_id)`
   — inconnu dans la saison de mon équipe : ignoré.
6. `IReferenceCatalogPort::find_match_reminders` (R9).

## references

### Chargement du corpus

`load_from_dir` lit en plus `match_reminders_fr.json` par une lecture
**optionnelle** : absent, `MatchRemindersFile::default()` ; mal formé, erreur
de démarrage nommant le fichier.

## Erreurs

| Enum | BC | Variantes | Réponse HTTP (contrôleur) |
|---|---|---|---|
| `TeamPrintError` | teams | `TeamNotFound`, `NotPrintable`, `Port(String)` | 404, page d'explication (cf. ci-dessous), 500 |
| `NextOpponentError` | competitions | `Port(String)` | 500 — une absence de match n'est **pas** une erreur |
| `PrintError` | match_report | `TeamNotFound`, `Port(String)` | 404, 500 |

`NotPrintable` (équipe qui n'est pas prête à jouer, R1) : la page « Imprimer »
répond une page courte qui l'explique, avec un lien vers la fiche, plutôt
qu'un 404 — on y arrive par une URL partagée ou mise en favori.
