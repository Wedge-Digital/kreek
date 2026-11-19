# BC `players` — Domaine : impact des rapports de match sur `Player`

**Priorité : haute**
**Dépend de :** rien (domaine pur, aucune dépendance externe)
**Contexte :** `players/domain` — agrégat, événements, méthodes de commande

## Objectif

Étendre l'agrégat `Player` pour qu'il puisse encaisser les faits d'un rapport de
match publié : stats de carrière, SPP gagné, blessures et statut de participation.
Spec complète : `docs/specs/player-match-impact/player-report-events/06-domaine.md`
(15 règles métier validées, BR1-BR15).

---

## Conception

### Nouveau fichier `src/app/players/domain/match_impact.rs`

Regroupe tous les nouveaux types : `MatchReportId`, `RoundId`, `MatchContext`,
`SppEarned` (nutype `>= 1`), `PlayerParticipationStatus`
(`Available | MissingNextGame | Retired | Dead`), `StatKind`
(`Ma|St|Ag|Pa|Av`), `InjuryType` (`Commotion|Amoche|BlessureSerieuse|Sequel{stat}|Mort`,
copie locale — ne jamais importer le type `match_report`, chaque BC a son propre
vocabulaire), `PlayerInjuryRecord`, `StatAdjustment`, et les 7 compteurs de carrière
(`TouchdownCount`, `PassCount`, `InterceptionCount`, `CasualtyCount`, `MvpCount`,
`FoulCount`, `PersistentInjuryCount` — struct tuple simple, même style que
`Spp`/`ValueKpo` dans `player.rs`, pas nutype).

### `Player` — nouveaux champs (`player.rs`)

```rust
pub participation_status:       PlayerParticipationStatus,
pub career_touchdowns:          TouchdownCount,
pub career_passes:              PassCount,
pub career_interceptions:       InterceptionCount,
pub career_casualties:          CasualtyCount,
pub career_mvps:                MvpCount,
pub career_fouls:                FoulCount,
pub career_persistent_injuries: PersistentInjuryCount,
pub injuries:                    Vec<PlayerInjuryRecord>,
pub stat_adjustments:            Vec<StatAdjustment>,
```

`PlayerCreated` (branche `apply()` existante) initialise tout à sa valeur par
défaut (`Available`, compteurs à 0, listes vides).

### 8 nouveaux `PlayerDomainEvent` (`events.rs`)

```
TouchdownScored { context, spp_earned }
PassCompleted { context, spp_earned }
InterceptionMade { context, spp_earned }
CasualtyInflicted { context, spp_earned }
MatchMvpNamed { context, spp_earned }
FoulCommitted { context }
InjurySustained { context, injury_type }
PlayerAvailabilityRestored { match_report_id }
```

Nommés en termes domaine (pas `PlayerPerformedX`, qui trahirait l'origine app event
— cf. règle CLAUDE.md sur le nommage des domain events).

### Méthodes de commande — infaillibles, pas de `Result`

Une par event (`record_touchdown`, `record_pass`, `record_interception`,
`record_casualty`, `record_mvp`, `record_foul`, `record_injury`,
`restore_availability`) — chacune ne fait que construire l'event. **Toute la
logique métier vit dans `apply()`**, jamais dans les méthodes de commande (séparation
event-sourcing stricte déjà en place dans le projet).

### `apply()` — règles métier par branche

| Event | Effet |
|---|---|
| `TouchdownScored`/`PassCompleted`/`InterceptionMade`/`CasualtyInflicted`/`MatchMvpNamed` | `spp += spp_earned`, incrémente le compteur de carrière correspondant |
| `FoulCommitted` | incrémente `career_fouls`, aucun SPP |
| `InjurySustained` | push dans `injuries` (**toujours**, y compris Commotion) puis, selon `injury_type` : `Commotion` → rien d'autre ; `Mort` → `participation_status = Dead` ; `BlessureSerieuse` → `participation_status = MissingNextGame` + `career_persistent_injuries += 1` ; `Amoche` → `participation_status = MissingNextGame` uniquement (pas de compteur, pas d'ajustement) ; `Sequel{stat}` → `participation_status = MissingNextGame` + push `StatAdjustment{stat, malus: 1}` (pas de compteur persistant) |
| `PlayerAvailabilityRestored` | si `participation_status == MissingNextGame` → repasse à `Available`, sinon no-op |

`Retired` n'est produit par **aucun** event de cette carte (déclenché ailleurs, par
la vente d'un joueur en phase de renvois post-match — hors périmètre).

### Aucune erreur domaine

`DomainError` (`error.rs`) reste l'enum vide actuel — ces méthodes décrivent des
faits déjà survenus et validés par `match_report`, aucune garde à ajouter.

---

## Checklist

- [ ] `src/app/players/domain/match_impact.rs` — tous les types listés ci-dessus
- [ ] Champs ajoutés à `Player` + initialisation dans `PlayerCreated`
- [ ] 8 nouveaux variants `PlayerDomainEvent`
- [ ] 8 méthodes de commande infaillibles sur `Player`
- [ ] 8 nouvelles branches `apply()` avec la logique du tableau ci-dessus
- [ ] Tests unitaires : touchdown crédite SPP + compteur, foul sans SPP, commotion journalisée sans effet, mort → Dead, blessure sérieuse → MissingNextGame + compteur persistant, amoché → MissingNextGame sans compteur ni ajustement, séquelle → MissingNextGame + stat_adjustment sans compteur persistant, restauration ne touche que MissingNextGame (no-op sur Available/Dead)
