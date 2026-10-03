# `ranking` et `match_report` passent par `est_admin`

**Priorité : haute**
**Épic :** aucune — série 570 à 573
**Dépend de :** 570
**Fichiers :**
`src/app/ranking/ports.rs`, `src/app/ranking/context.rs`,
`src/app/ranking/use_cases/manual_points.rs`,
`src/app/ranking/use_cases/award_manual_points_use_case.rs`,
`src/app/ranking/use_cases/revoke_manual_points_use_case.rs`,
`src/app/ranking/io/web/manual_points/controller.rs`,
`src/app/ranking/io/web/widgets/classement_widget.rs`,
`src/app/ranking/io/web/widgets/detailed_standings_widget.rs`,
`src/infrastructure/ranking/admin_adapter.rs` *(supprimé)*,
`src/app/match_report/ports.rs`, `src/app/match_report/context.rs`,
`src/app/match_report/use_cases/match_report_access_service.rs`,
`src/app/match_report/io/web/recap_controller.rs`,
`src/app/match_report/io/web/match_selection_controller.rs`,
`src/infrastructure/match_report/space_admin_adapter.rs` *(supprimé)*,
`src/infrastructure/match_report/competition_data_adapter.rs`, `src/main.rs`,
tests des deux BCs

## L'objectif

Les droits d'administration de `ranking` et de `match_report` se demandent à
`est_admin` (carte 570). Les ports qui ne servaient qu'à ça disparaissent.

## Ce qui l'a fait naître

Les deux BCs posaient la question par leurs propres ports
(`IRankingAdminPort`, `ISpaceAdminPort` et `ICompetitionDataPort::is_competition_admin`),
avec leur propre doublure de test chacun.

## Le changement

**`ranking`** :
- `manual_points::autorise` = `est_admin` sur l'espace et la compétition du
  chemin ;
- `RankingContext.admin_port` devient `admin_access: Arc<dyn IAdminAccessPort>` ;
  `IRankingAdminPort` et `infrastructure/ranking/admin_adapter.rs` disparaissent ;
- l'attribution, le retrait, le formulaire, la liste et le bouton « ⚖️ Gérer
  les points manuels » (classement simple et détaillé) suivent.

**`match_report`** :
- `est_administrateur` = `est_admin` ; `is_authorized` = coachs du match, ou
  `est_admin` — la condition des coachs reste au BC (`ITeamDataPort::is_coach_of_team`) ;
- `ISpaceAdminPort` et son adapter disparaissent ; `ICompetitionDataPort` perd
  sa seule méthode `is_competition_admin` (il garde tout le reste) ;
- le récapitulatif, la publication, la correction, la sélection figée et le
  bouton « déplacer le match » suivent.

**Ce qui change pour les utilisateurs** : rien dans une base cohérente — ces
deux BCs comparaient déjà par identifiant seul. Le compte `Bagouze` y gagne
l'accès partout, par l'exception de l'adapter.

## Ce que la carte ne couvre pas

`players`, `competitions`, le menu : carte 572.

## Tests

Unitaires : les use cases d'attribution et de retrait, `is_authorized` et
`est_administrateur` sur `FakeAdminAccess`, à la place des `FakeAdmin` locaux.
`est_administrateur` gagne enfin ses tests — il n'en avait aucun.

E2E : `test_manual_ranking_points.py`, `test_match_report_recap.py`,
`test_match_report_correction.py`, `test_competition_hors_calendrier.py`,
`test_deplacer_un_match.py` passent sans modification.

## Terminé quand

`grep -rn "IRankingAdminPort\|ISpaceAdminPort\|fn is_competition_admin" src/app`
ne renvoie plus que le port du noyau partagé, et la suite complète passe.
