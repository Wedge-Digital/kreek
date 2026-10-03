# Un seul service pour savoir qui est admin

**Priorité : haute — prérequis du workflow « phases manuelles »**
**Épic :** aucune — première de la série 570 à 573
**Dépend de :** rien
**Suivie de :** 571 (`ranking`, `match_report`), 572 (`players`,
`competitions`, menu), 573 (routes d'administration sans garde)
**Fichiers :**
`src/app/shared_kernel/bloodbowl/admin_access.rs` *(nouveau)*,
`src/app/shared_kernel/bloodbowl/mod.rs`,
`src/infrastructure/admin_access/mod.rs` *(nouveau)*,
`src/infrastructure/admin_access/admin_access_adapter.rs` *(nouveau)*,
`src/infrastructure/mod.rs`, `src/main.rs`,
`src/app/teams/context.rs`, `src/app/teams/ports.rs`,
`src/app/teams/use_cases/roster_edit_access_service.rs`,
`src/app/teams/io/web/team_detail.rs`, `src/app/teams/io/web/garde_action_equipe.rs`,
`src/app/teams/io/web/adjust_treasury_controller.rs`,
`src/app/teams/io/web/dismiss_team.rs`, `src/app/teams/router.rs`,
`src/infrastructure/teams/access_adapter.rs` *(supprimé)*,
`src/infrastructure/teams/mod.rs`, tests de `teams`, `CLAUDE.md`

## L'objectif

« Admin d'espace ou de compétition » se demande à **un seul service**, dans le
noyau partagé, que chaque BC interroge par le même port et que chaque test peut
remplacer par une doublure. Cette carte le crée et y fait passer `teams`.

## Ce qui l'a fait naître

La règle existe en **six copies**, dans six BCs, chacune avec son port et son
adapter — `teams`, `ranking`, `match_report`, `players`, `competitions`, et
`SpacePermissions::is_admin()` partout. Elles ne disent pas la même chose :
deux comparent aussi le **nom** du coach aux admins de la compétition, deux non ;
trois réservent une action à l'admin d'espace là où un admin de compétition peut
tout le reste.

Le workflow « phases manuelles » allait en écrire une septième (`est_commissaire`).

## Les règles, tranchées le 2026-10-03

1. **Admin de compétition : par identifiant seul.** Les admins ne s'enregistrent
   que par identifiant (`competitions_members.coach_id`) ; `admin_names` n'est
   que leur pseudonyme, obtenu par jointure. Le comparer au nom du visiteur
   n'apportait rien dans une base cohérente, et des cas bizarres dans une base
   désynchronisée — la comparaison est sensible à la casse, les comptes non.
2. **Le compte exploitant `Bagouze` a toujours les droits**, dans tout espace et
   toute compétition. L'exception vit **dans l'adapter**, qui reçoit pour cela le
   nom du demandeur en plus de son identifiant.
3. **Le propriétaire reste l'affaire de chaque BC.** Inclus pour l'effectif,
   exclu pour la customisation : la condition s'ajoute devant `est_admin`, elle
   n'y entre pas.
4. **Un port en erreur refuse**, comme toutes les copies d'aujourd'hui.
5. **Renvoi d'équipe et ajustement de trésorerie s'ouvrent aux admins de
   compétition** (décision du 2026-10-03). Le propriétaire en reste exclu.

## Le changement

**Le service, dans `shared_kernel::bloodbowl::admin_access`** — `bloodbowl` et
non `identity`, parce que la règle parle de compétition, que `auth` et `spaces`,
extractibles, ignorent. Eux gardent `SpacePermissions::is_admin()`.

```rust
/// Qui pose la question.
pub struct Demandeur<'a> {
    pub id: &'a CoachId,
    pub nom: &'a str,
}

#[async_trait]
pub trait IAdminAccessPort: Send + Sync {
    async fn is_space_admin(&self, demandeur: &Demandeur<'_>, space_id: &SpaceId) -> bool;
    async fn is_competition_admin(&self, demandeur: &Demandeur<'_>, competition_id: &CompetitionId) -> bool;
}

/// La règle, écrite une fois : admin de l'espace, ou de la compétition.
pub async fn est_admin(
    port: &dyn IAdminAccessPort,
    demandeur: &Demandeur<'_>,
    space_id: &SpaceId,
    competition_id: Option<&CompetitionId>,
) -> bool
```

L'espace d'abord, la compétition ensuite, chacune court-circuitant la suivante ;
pas de compétition, pas de seconde question.

**La doublure de test, dans le même module** (`#[cfg(test)]`) :
`FakeAdminAccess::new().admin_espace(coach, espace).admin_competition(coach,
competition)` — refuse tout ce qu'on ne lui a pas dit. Elle remplace les
doublures écrites BC par BC.

**L'adapter unique, `infrastructure/admin_access/`** — repris par copier-coller
de `infrastructure/teams/access_adapter.rs` (règle 5), moins la comparaison par
nom, plus l'exception exploitant. Il lit `spaces__user_space.profile` par
`ISpaceRepository::find_member_profile` et `competitions_members` par
`ICompetitionRepository::find_base_info`. Construit une fois dans `main.rs`.

**`teams` y passe** :
- `TeamsContext.access_port: Arc<dyn ITeamAccessPort>` devient
  `admin_access: Arc<dyn IAdminAccessPort>` ; `ITeamAccessPort` et
  `infrastructure/teams/access_adapter.rs` disparaissent, consommateurs listés
  avant (règle 4) ;
- `roster_edit_access_service` : `peut_modifier_effectif` = propriétaire, ou
  `est_admin` sur l'espace et la compétition de l'équipe ; un second service,
  `est_admin_de_l_equipe`, pour les actions de commissaire (sans propriétaire) ;
- `team_detail.rs` : le droit d'ajuster la trésorerie passe par
  `est_admin_de_l_equipe` (admin de compétition compris) ;
- `adjust_treasury_controller.rs` et `dismiss_team.rs` : `SpacePermissions`
  remplacé par `est_admin_de_l_equipe` ;
- les commentaires qui disaient « `SpacePermissions::is_admin()`, admin d'espace
  seul » (`router.rs:51`, `garde_action_equipe.rs:25`) sont corrigés.

**La règle entre dans le `CLAUDE.md`**, section « Qui est admin — un seul
service ».

## Ce que la carte ne couvre pas

- `ranking`, `match_report` : carte 571.
- `players`, `competitions`, le menu `⚙️ Espace` : carte 572.
- Les routes d'administration sans aucune garde : carte 573.

## Tests

Unitaires :
- `est_admin` sur la doublure : admin d'espace, admin de compétition, les deux,
  personne, pas de compétition ;
- l'adapter, sur une vraie base : admin d'espace par profil, simple membre
  refusé, admin de compétition par identifiant, **le même coach désigné par son
  seul nom refusé**, `Bagouze` admin partout, port en erreur refusé ;
- `roster_edit_access_service` sur la doublure. Le test « admin de compétition
  par nom » change de sens : il vérifie désormais que le nom seul **ne suffit
  plus** — c'est la décision 1, pas une régression.

E2E : un admin de compétition, non admin d'espace, ajuste la trésorerie d'une
équipe de sa compétition ; un simple membre est refusé. Les tests existants de
la fiche équipe, de la trésorerie, de l'édition d'effectif et de la garde
passent sans modification.

## Terminé quand

`teams` ne déclare plus aucun port d'administration, `grep -rn "ITeamAccessPort"
src` est vide, un admin de compétition ajuste la trésorerie de ses équipes, et
`make test`, `make e2e`, `make check-arch` passent.
