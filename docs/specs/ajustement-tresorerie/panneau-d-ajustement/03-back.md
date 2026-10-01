# Panneau d'ajustement · Phase 3 : architecture back

**Entrée** : `02-front.md` validé.

## Une seule page est concernée

`team_page_treasury` (`team_detail.rs:446`) est le seul handler à toucher en
lecture, et un POST neuf s'y ajoute. Les onglets « Joueurs & Staff » et
« Matchs » ne changent pas.

## Aucune migration — et pourquoi

Le motif saisi et le nom du commissaire n'ont **pas besoin de colonne**.

`list_treasury_movements.sql` joint déjà chaque ligne du grand livre à
l'événement qui l'a produite :

```sql
SELECT l.event_version, l.direction, …, e.payload
FROM   teams__treasury_ledger l
LEFT JOIN team_event_store e ON e.team_id = l.team_id AND e.version = l.event_version
```

`detail_de()` (`treasury_statement_service.rs:203`) aiguille sur le motif et lit
cette charge utile — c'est ainsi que « Gwenn, Passeuse — n° 7 » et « Jet de 3 —
Fraude fiscale » sont fabriqués. Le motif et l'auteur voyagent donc dans
l'événement domaine.

L'écriture est générique de la même façon (`team_repository.rs:276-287`) : pour
tout événement dont `treasury_movement()` rend `Some`, la ligne de grand livre
est insérée **dans la transaction de l'append**. Un événement neuf traité par
cette méthode produit sa ligne sans qu'on touche au dépôt.

## Les deux gardes

**Au GET — on calcule, on ne refuse pas.** Le relevé reste lisible par tous
(règle 1, phase 1) ; seul le bouton dépend du droit.

```rust
// dans rendre_fiche, à côté du `peut_editer` existant
let est_admin = match auth_session.user.as_ref() {
    Some(user) => state.teams.access_port.is_space_admin(&user.id, &space_id).await,
    None => false,
};
```

`ITeamAccessPort` (`ports.rs:80`) est déjà câblé dans `TeamsContext`
(`context.rs:29`), implémenté par `infrastructure/teams/access_adapter.rs`, et
déjà appelé par `roster_edit_access_service.rs:49`. **Rien à créer.** C'est ce
chemin et pas une requête directe parce que `teams` n'interroge pas les tables
de `spaces` — souveraineté des données.

**L'extracteur `SpacePermissions` ne convient pas au GET** : il répond 403 à un
non-membre de l'espace, ce qui fermerait un relevé qu'on veut public.

**Au POST — on refuse.** Là, 403 est le but :

```rust
if !perms.is_admin() { return StatusCode::FORBIDDEN.into_response(); }
```

C'est `dismiss_team.rs:18`, repris tel quel. `SpacePermissions` est déjà importé
par `teams` depuis `spaces` (`dismiss_team.rs:3`).

### Le double appel, assumé

`peut_modifier_effectif` interroge déjà `is_space_admin` pour le même couple :
le port sera appelé deux fois par rendu de l'onglet. Le factoriser demanderait
de séparer admin et propriétaire dans `peut_modifier_effectif`, qui les mêle en
un seul booléen — un refactor hors de cette fonctionnalité. Noté, pas corrigé.

## La route

```
POST /app/{space_id}/teams/{team_id}/tresorerie/ajuster
```

Chemin en français, comme `TEAM_TREASURY` qu'il prolonge.

**Dans `routes_ouvertes()`, pas dans `routes_d_action()`.** Ce second groupe
porte `garde_action_equipe`, qui est la règle du **coach**
(`roster_edit_access_service`) — un commissaire n'est pas le coach, et le groupe
le refuserait avant d'atteindre le handler. `DISMISS_TEAM` est exactement dans
ce cas et se garde lui-même ; on suit cette forme.

## Les fichiers

| Fichier | Ce qu'il reçoit |
|---|---|
| `teams/routes.rs` | la constante, la méthode, **et l'entrée du test d'inventaire** (l. 279-316) |
| `teams/router.rs` | une ligne dans `routes_ouvertes()` |
| `teams/io/web/adjust_treasury_controller.rs` | **neuf** — garde admin, parsing, appel, réponse |
| `teams/use_cases/adjust_treasury_use_case.rs` | **neuf** — `#[tracing::instrument(skip_all, fields(cmd = ?cmd))]`, exigé par l'axe 11 |
| `teams/use_cases/commands.rs` | `AdjustTreasuryCommand` |
| `teams/domain/team.rs` | la variante d'événement, la méthode domaine, une branche dans `treasury_movement()` |
| `teams/domain/treasury.rs` | le 9ᵉ motif — **trois endroits** : `ALL`, `as_str`, `garde_d_exhaustivite` |
| `teams/use_cases/treasury_statement_service.rs` | une branche dans `detail_de()` |
| `teams/io/web/treasury_tab.rs` | `rendre_onglet` reçoit `space_id` et `est_admin` |
| `teams/io/web/templates/teams-treasury-tab.html` | le bouton et le panneau |
| `teams/io/web/templates/teams-treasury-adjust-error.html` | **neuf** — le pied du panneau, rendu seul en cas d'erreur |
| `teams/io/web/team_detail.rs` | calcule `est_admin`, le passe à `contenu_de_l_onglet` |
| `assets/static/css/pages/team-treasury.css` | les règles du bouton et du panneau |

`rendre_fiche` gagne un paramètre, donc les deux autres handlers d'onglets lui
passent `false` : un mot chacun, aucun changement de comportement.

Nom de fichier en `_controller.rs` et `_use_case.rs` — la convention s'applique
à tout fichier neuf.

## Ni port ni domain service à créer

Pas de port neuf : `ITeamAccessPort` existe. Pas de domain service : le handler
n'a aucun DTO de port à transformer en objet domaine — il construit une commande
et la passe au use case.

## L'événement ne sort pas du BC

`to_app_event()` n'a pas de joker, donc la variante neuve **cassera la
compilation** à cet endroit et forcera une décision. Elle rejoint la liste
nommée des variantes qui restent chez elles : aucun autre BC ne se soucie de la
caisse d'une équipe, et aucune notification n'a été demandée. Décidé le
2026-10-01.

L'axe 18 de `check-arch` reste satisfait : l'événement est émis par le use case,
ce n'est pas un fantôme.

## Émission et journalisation

**Le use case n'émet pas.** Dans `teams`, c'est le dépôt qui le fait, après le
commit (`team_repository.rs:400-408`) — « le seul point qui les couvre tous »,
deux des quatre chemins vers `ReadyToPlay` passant par des listeners. Déviation
assumée du patron de `players` et `match_report`, et le use case n'a donc qu'à
appeler `append`.

Corrigé en phase 5 : cette section affirmait d'abord le contraire.

Aucun publisher à toucher, puisque rien ne sort du BC. Le use case est
instrumenté (axe 11).

## Règles métier

**Aucune nouvelle à cette étape.** Question posée le 2026-10-01 : la phase 3
place des gardes et des fichiers, elle ne décide pas de ce qui est permis. Les
dix règles restent celles de `02-front.md`.
