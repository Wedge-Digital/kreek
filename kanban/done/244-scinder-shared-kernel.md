# Scinder `shared_kernel` : noyau d'identité / noyau Blood Bowl

**Priorité : moyenne**
**Dépend de :** 243 (le cycle doit être cassé avant de découper)
**Fichiers :** `src/app/shared_kernel/**`, `askama.toml` (non), tous les imports de `shared_kernel`

## Problème

`shared_kernel` est un fourre-tout. À côté de `sulid`, `email` et `space_name`,
il héberge `roster_definition`, `staff`, `staff_counts`, `tier`,
`inducement_definition`, `team`, `competition_name`, `season_name`,
`ranking_group_id`, `competition_profile` — c'est-à-dire tout le vocabulaire
Blood Bowl de kreek.

`common_types.rs` pousse le mélange plus loin : il déclare les identifiants des
**onze** BCs dans un seul fichier — `PlayerId`, `MatchReportId`, `PairingId`,
`RoundId`, `RosterId`, `PositionId`, `CompetitionId`, `SeasonId`, `ArticleId`,
`CommentId`… Auth et Spaces n'utilisent que `UserId`, `CoachId`, `SpaceId`,
`EventId`, `EntityId`/`SUlid` et `CloudinaryImage`.

Copier `shared_kernel` dans un autre projet, c'est aujourd'hui y importer le
modèle de données du Blood Bowl.

## Action

Séparer en deux sous-modules à l'intérieur de `shared_kernel` (pas de crate —
cf. décision de la carte 242) :

**Noyau d'identité** — ce que les BCs extraits emportent :

| Fichier | Note |
|---|---|
| `sulid.rs` | + `tests/test_sulid.rs` |
| `id_service.rs` | + `tests/test_id_service.rs` |
| `authorization.rs` | `SpaceProfile` |
| `coach_name.rs`, `coach_icon.rs`, `coach_initials.rs`, `coach_definition.rs` | |
| `email.rs` | |
| `space_name.rs`, `space_definition.rs` | |
| `cloudinary.rs` | + `CloudinaryImage` |
| `name_vo.rs` | brique de construction des VOs de noms |

**Noyau Blood Bowl** — ce qui reste à kreek : `roster_definition.rs`,
`staff.rs`, `staff_counts.rs`, `tier.rs`, `inducement_definition.rs`,
`team.rs`, `competition_name.rs`, `competition_profile.rs`, `season_name.rs`,
`ranking_group_id.rs`, `date_string.rs`, `timezone.rs`.

**`common_types.rs` est le point délicat** : il faut le scinder. Les alias
d'identifiants suivent leur BC (`PlayerId` → noyau Blood Bowl,
`SpaceId`/`CoachId`/`UserId` → noyau d'identité) ; `EntityId`, `EventId`,
`SUlid`, le trait `Entity` et `CloudinaryImage` vont dans le noyau d'identité.
C'est le fichier qui produira le plus de churn d'imports — le faire en un seul
passage mécanique.

**`app_events/`** : le dossier agrège les contrats des huit BCs. Chaque contrat
appartient à son émetteur. `auth_app_events.rs` et `spaces_app_events.rs` sont
la surface publique des BCs extraits — les regrouper avec le noyau d'identité,
ou les déplacer dans leur BC respectif si l'inventaire des consommateurs le
permet. À trancher à l'implémentation. Noter que `spaces` consomme
`AuthAppEvent` dans `user_created_listener` : c'est normal (un consommateur
connaît le contrat de son producteur) et ne pose pas de problème puisque les
deux BCs s'extraient en couple.

**Ménage au passage** : `shared_kernel/ports.rs` est un fichier vide (1 octet)
sans consommateur — le supprimer.

## Note

Aucun renommage de type n'est demandé : `CoachName` reste `CoachName`, même si
le vocabulaire est Blood Bowl (cf. verrues assumées, carte 242). Cette carte ne
fait que déplacer des fichiers et réparer des imports.

## Checklist

- [ ] Les deux sous-modules créés, chaque fichier classé
- [ ] `common_types.rs` scindé, alias d'identifiants rangés avec leur BC
- [ ] `shared_kernel/ports.rs` (vide) supprimé
- [ ] Décision prise et appliquée sur `app_events/`
- [ ] Aucun fichier de `auth/` ou `spaces/` n'importe un type du noyau Blood Bowl :
      `grep -rn "roster\|staff\|tier\|inducement\|competition_name\|season_name" src/app/auth src/app/spaces`
- [ ] `make check-arch` au vert, `make test` au vert
