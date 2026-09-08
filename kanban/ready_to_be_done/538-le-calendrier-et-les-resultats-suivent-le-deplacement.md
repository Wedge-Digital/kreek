# Le calendrier et les résultats suivent le déplacement

**Priorité : haute — sans elle, le déplacement ne se voit nulle part**
**Dépend de :** 537
**Épic :** E17 — Corriger le calendrier sans perdre la saisie
**Fichiers :** `src/app/match_report/io/app_events/app_event_publisher.rs`,
`src/app/competitions/io/app_events/match_report_round_changed_listener.rs` (nouveau),
`src/app/competitions/domain/match_day_repository_port.rs`,
`src/app/competitions/io/repository/match_day_repository.rs`

## Ce qu'il faut déplacer, et où

Une rencontre est rattachée à sa journée **par son appariement**, et l'affichage
la lit dans une projection qui recopie six colonnes de journée :

```
competition_match_day_pairings.match_day_id
competition_match_display_proj : round_id, round_name, round_position,
                                 round_date_start, round_date_end, round_day_type
```

C'est cette projection que lisent `list_resultats.sql`, `list_calendrier.sql` et
`list_team_matches.sql` : **l'onglet Résultats, l'onglet Calendrier et l'onglet
Matchs d'une équipe suivent tous de là.** Les six colonnes se réécrivent
ensemble, faute de quoi la rencontre s'affiche sous l'ancien nom de journée avec
la nouvelle position.

## La chaîne existe déjà en trois exemplaires

`competition_match_display_proj` n'est jamais écrite en SQL direct depuis
ailleurs. Trois listeners de `competitions` la mettent à jour sur app event :

| Listener | Ce qu'il écrit |
|---|---|
| `match_report_confirmed_listener` | `match_status = 'in_progress'` |
| `match_report_published_listener` | `match_status = 'completed'` |
| `match_report_cancelled_listener` | `match_status = 'upcoming'` |

**Un quatrième s'y ajoute**, sur le même patron : domain event → publisher → app
event → listener. Pas de SQL hors de cette chaîne.

## Le dépôt gagne une méthode

`IMatchDayRepository` sait créer, chercher et supprimer un appariement —
**aucune méthode ne change sa journée**.

```rust
async fn move_pairing(&self, pairing_id: &str, round_id: &str)
    -> Result<(), MatchDayRepositoryError>;
```

Transactionnelle : la ligne d'appariement et les six colonnes de projection
changent ensemble, ou pas du tout. C'est la règle du projet — table et
projection dans la même transaction — et le listener s'appuie dessus plutôt que
d'enchaîner deux écritures.

## Le BC `ranking` n'a rien à faire, et c'est mesuré

| Constat | Preuve |
|---|---|
| Les écrans lisent l'**état final** | `SELECT DISTINCT ON (team_id) … ORDER BY team_id, sequence DESC` |
| L'ordre ne change pas les totaux | les cumuls sont des sommes |
| `ranking_lines.round_id` n'est **jamais relu** | écrit à la publication, aucun lecteur |

La seule lecture ordonnée (`sequence ASC`) n'a qu'un appelant, le rejeu, qui ne
s'affiche pas. Déplacer un match ne modifie donc aucun classement.

## Checklist

- [ ] `RoundChanged` converti en app event par le publisher de `match_report`
- [ ] `move_pairing` au port et au dépôt, en une transaction
- [ ] Le listener de `competitions`, sur le patron des trois existants
- [ ] Les **six** colonnes de journée réécrites, pas seulement `round_id`
- [ ] Test d'intégration : après déplacement, `list_resultats` et
      `list_calendrier` rendent la rencontre sous la **nouvelle** journée, avec
      ses dates et sa position
- [ ] Test : un appariement déplacé garde son identifiant — les rapports qui le
      désignent ne sont pas orphelins
- [ ] `make lint`, `make check-arch`, `make test`
