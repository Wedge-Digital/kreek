# `players`, `competitions` et le menu passent par `est_admin`

**Priorité : haute**
**Épic :** aucune — série 570 à 573
**Dépend de :** 570
**Fichiers :**
`src/app/players/ports.rs`, `src/app/players/context.rs`,
`src/app/players/io/web/purchase_skill_controller.rs`,
`src/app/players/io/web/increase_stat_controller.rs`,
`src/app/players/io/web/roster_edition_controller.rs`,
`src/app/players/io/web/player_detail_controller.rs`,
`src/app/players/io/web/customisation_access.rs`,
`src/app/players/io/web/widgets/spp_spending_widget.rs`,
`src/infrastructure/players/competition_admin_adapter.rs` *(supprimé)*,
`src/infrastructure/players/space_member_adapter.rs` *(supprimé)*,
`src/app/competitions/ports.rs`,
`src/app/competitions/use_cases/competition_admin_access_service.rs`,
`src/app/competitions/io/web/admin/admin_page.rs`,
`src/app/competitions/io/web/competition_detail.rs`,
`src/app/competitions/io/web/resultats_view.rs`,
`src/app/competitions/io/web/latest_results_view.rs`,
`src/infrastructure/competitions/space_member_adapter.rs`,
`src/web/app_menu.rs`, `src/main.rs`, tests des BCs

## L'objectif

Les derniers droits d'administration écrits à part passent par `est_admin`. À
la fin de cette carte, la règle n'existe plus qu'en un endroit.

## Le changement

**`players`** :
- `can_spend_spp` = propriétaire, ou `est_admin` ; `can_customise` et
  `customisation_access::autoriser` = `est_admin` seul (le propriétaire reste
  exclu, c'est un geste de commissaire) ;
- `IPlayerCompetitionPort` et `IPlayerSpaceMemberPort`, qui ne servaient qu'à
  ça, disparaissent avec leurs adapters.

**`competitions`** :
- `peut_administrer` = `est_admin` ; `require_admin_access` garde sa cohérence
  saison/compétition (carte 416), qui n'est pas un droit ;
- `compute_authorization` (onglets Résultats et Calendrier, matchs d'une
  équipe) et `latest_results_view` (accueil) : coachs du match, ou `est_admin` ;
- `ICompetitionSpaceMemberPort` perd `find_member_profile` ; il garde
  `list_space_members` et `find_all_spaces`.

**Le menu `⚙️ Espace`** (`app_menu.rs`) : `est_admin` sans compétition — donc
admin d'espace, ou `Bagouze`. La constante `COMPTE_EXPLOITANT` quitte la couche
hôte : l'exception ne vit plus que dans l'adapter (carte 570).

**Ce qui change pour les utilisateurs** : rien dans une base cohérente. La
comparaison par nom de ces BCs disparaît (décision 1 de la carte 570) ; les
écarts entre le lien d'un rapport affiché à un admin et le rapport qu'il pouvait
ouvrir se referment.

## Ce que la carte ne couvre pas

Les routes d'administration sans garde : carte 573.

## Tests

Unitaires : `can_spend_spp` et `can_customise` gagnent leurs premiers tests, sur
`FakeAdminAccess` ; `peut_administrer` garde ses sept cas, réécrits sur la
doublure — celui « par nom » change de sens.

E2E : `test_player_spp_spending.py`, `test_player_customisation.py`,
`test_player_detail.py`, `test_roster_edition.py`,
`test_competition_admin_acces.py`, `test_competition_admin_settings.py`,
`test_competition_bouton_administration.py`, `test_team_matches.py`,
`test_accueil_derniers_resultats.py` passent sans modification ;
`src/web/tests/test_menu_administration.rs` aussi.

## Terminé quand

`grep -rn "fn is_space_admin\|fn is_competition_admin\|SpaceProfile::SpaceAdmin"
src/app src/web src/infrastructure` ne renvoie plus que l'adapter unique, le
port du noyau partagé et les BCs extractibles `spaces` et `auth`, et la suite
complète passe.
