# `match_report` — compter les joueurs réellement disponibles

**Priorité : haute**
**Dépend de :** rien — indépendante et parallélisable
**À livrer avant ou avec :** 251
**Fichiers :** `src/infrastructure/match_report/player_data_adapter.rs`,
`src/app/players/ports.rs`,
`src/app/players/io/repository/projection_repository.rs`,
`tests/e2e/`

## Problème

`count_available_players` ne compte pas les joueurs disponibles.

```rust
// src/infrastructure/match_report/player_data_adapter.rs:24-29
async fn count_available_players(&self, team_id: &str) -> Result<usize, String> {
    let players = self.player_projection_repo.find_by_team_id(&tid).await…?;
    Ok(players.len())          // ← l'effectif total, sans filtre
}
```

`participation_status` n'est jamais consulté, alors que la colonne existe depuis
la migration `20260714131949` et qu'elle est correctement alimentée :
`player_match_impact_listener` pose `MissingNextGame` à la publication d'un
rapport, `team_match_concluded_listener` le lève à la conclusion du match suivant
(BR12).

Conséquence directe : le nombre de journaliers vaut aujourd'hui
`11 − effectif total` au lieu de `11 − joueurs disponibles`
(`init_temp_players_use_case.rs:201-205`). **Une équipe amoindrie par les
blessures ne reçoit aucun renfort** : elle a 13 joueurs sur la feuille, donc
`11 − 13 = 0` journalier, alors que 4 d'entre eux ratent le match.

## Pourquoi c'est bloquant pour la TV

La carte 251 fait compter les indisponibles à zéro **et** ajoute des journaliers
à la TV. Si `match_report` continue de se croire au complet, la TV annonce une
équipe renforcée qui n'existera pas sur le terrain, et les coups de pouce sont
calculés sur un effectif fictif. Les deux chiffres se contrediraient de façon
visible dès le premier match d'une équipe diminuée.

## Action

### 1. Une requête qui filtre

Exposer sur le repository de projection de `players` un comptage filtré sur
`participation_status = 'Available'`, et le consommer depuis l'adapter.

Ne pas filtrer en mémoire après avoir chargé tout l'effectif : le port ne renvoie
qu'un nombre, la base sait le calculer.

### 2. Renommer si nécessaire

Si le comptage total reste utile ailleurs, séparer les deux méthodes plutôt que
de changer le sens de celle-ci sous ses appelants existants. Vérifier les
consommateurs de `count_available_players` avant de trancher.

## Duplication assumée, temporairement

Après cette carte, la règle « 11 − disponibles » est implémentée deux fois : dans
`match_report` (`collect_journeymen`) et dans `teams` (fonction pure de la carte
250). Les deux comptent la même chose, avec la même définition.

Le lot de **déplacement des journaliers vers `teams`** supprimera ce doublon —
avec, au passage, la duplication qui existe déjà entre
`match_report::ITeamDataPort::find_journeyman_position` (async, `position_uid`)
et `teams::IJourneymanTypePort::journeyman_type_for_roster` (sync, nom
d'affichage), deux ports pour la même règle « le poste le plus nombreux du
roster ».

## Attention — flux publié

`init_temp_players_use_case` est au cœur de l'étape 1 du rapport de match, un
flux publié et couvert en e2e par la série 227-236 (correction de rapport). La
couverture e2e de cette carte n'est pas optionnelle.

## Checklist

- [ ] Comptage filtré sur `participation_status` exposé par le repository de projection
- [ ] `count_available_players` consomme ce comptage, plus `players.len()`
- [ ] Consommateurs existants vérifiés avant renommage ou changement de sens
- [ ] Test unitaire : 13 joueurs dont 4 indisponibles → 2 journaliers
- [ ] E2E : une équipe diminuée reçoit le bon nombre de journaliers à l'étape 1
- [ ] E2E : la série 227-236 (correction de rapport) passe toujours
- [ ] `make check-arch` au vert, `make test` au vert
