# Appartenance à l'effectif — un axe distinct dans `players`

**Priorité : haute**
**Dépend de :** rien
**Bloque :** 270
**Croise :** cartes 250 et 253, qui touchent deux des sept chemins de lecture
**Spec :** `docs/specs/phases-recrutement-renvois/renvois/04-dtos.md` §1
**Fichiers :** `src/app/players/domain/player.rs`,
`src/app/players/domain/events.rs`,
`src/app/players/io/repository/player_repository.rs`,
`src/app/players/io/repository/projection_repository.rs`, migration

## Problème

`players` n'a **aucun moyen d'exprimer qu'un joueur a quitté l'équipe**. Ses statuts
sont `Available`, `MissingNextGame`, `Retired`, `Dead` ; ses quatorze événements
domaine ne couvrent que la création, les compétences, les faits de match et les
blessures.

Sans traitement, un joueur renvoyé resterait `Available` : compté par le port
d'effectif, donc dans la **valeur d'équipe** et dans le **nombre de journaliers**. Le
renvoi serait sans effet réel.

`PlayerParticipationStatus` ne convient pas pour l'accueillir : il décrit des
**conséquences de match** — `player.rs:70` porte littéralement le commentaire « Impact
des rapports de match ». `Available` et `MissingNextGame` y sont posés par l'impact de
match, `Dead` par une blessure. *(`Retired` n'est posé nulle part : jalon de la carte
39.)* Un renvoi est une **décision de coach**.

## Action

### 1. Un axe distinct

```rust
// domain/player.rs
pub enum RosterMembership { Active, Dismissed }

pub struct Player {
    …
    pub membership: RosterMembership,        // ← hors du bloc « impact des matchs »

    // ── Impact des rapports de match ───────────────────────────────────────
    pub participation_status: PlayerParticipationStatus,
    …
}
```

Nouvel événement domaine `PlayerDismissed`, qui pose `membership = Dismissed`.

**Le joueur n'est pas supprimé** : `players` est event-sourcé. Il garde ses SPP, ses
compétences et son historique ; il cesse simplement d'appartenir à l'effectif.

### 2. Migration

`players_proj` gagne `membership TEXT NOT NULL DEFAULT 'Active'`. Aucun joueur n'a
jamais été renvoyé : le défaut suffit, pas de reprise de données.

### 3. Le filtre vit dans le repository — point critique

Sept chemins lisent l'effectif d'une équipe, et **tous les sept excluent les
renvoyés**, y compris l'affichage du roster : le coach n'a pas besoin de voir ses
joueurs renvoyés.

| Chemin | |
|---|---|
| `players/io/app_events/player_match_impact_listener.rs:178` | actifs seuls |
| `players/io/app_events/team_match_concluded_listener.rs:31` | actifs seuls |
| `infrastructure/match_report/player_data_adapter.rs:27` | actifs seuls — **carte 253 y touche** |
| `infrastructure/match_report/player_data_adapter.rs:64` | actifs seuls |
| `infrastructure/teams/player_count_adapter.rs:19` | actifs seuls — **carte 259 le supprime** |
| `players/io/web/player_table.rs:99` | actifs seuls |
| `ISquadPort` | actifs seuls — **carte 259** |

**`find_by_team_id` filtre donc à la source**, et il n'y a **pas** de variante
`…_including_dismissed` : elle serait du code mort le jour de sa création. Aucun
appelant n'a de filtre à écrire, donc aucun ne peut l'oublier.

L'agrégat reste lisible par `find_by_id` : un renvoyé n'est pas effacé.

### 4. Vocabulaire

Le projet dit déjà « dismissal » partout — phase `Dismissals`,
`validate_dismissals_phase`, `DismissalsPhaseValidated`. Le nom retenu est
`Dismissed`, identique de `teams` jusqu'à `players` en passant par le bus.

Nommer le même fait pareil des deux côtés ne contrevient pas au CLAUDE.md : la règle
interdit de nommer un domain event d'après son origine externe
(`PlayerDismissedReceived`), pas l'homonymie.

## Ce que cette carte ne règle pas

`Retired` **ne se débloque pas** pour autant. La carte 39 parle de retraite
*temporaire*, ce qui n'est pas une fin d'appartenance — probablement un troisième
état, ou une suspension. À ne pas trancher ici.

## Checklist

- [ ] `RosterMembership` sur `Player`, hors du bloc « impact des rapports de match »
- [ ] Événement `PlayerDismissed`, projection mise à jour dans la même transaction
- [ ] Migration `players_proj.membership`
- [ ] `find_by_team_id` filtre sur `membership = 'Active'`
- [ ] **Aucune** variante `…_including_dismissed` créée
- [ ] Les sept chemins vérifiés un par un
- [ ] Test : un joueur renvoyé disparaît de `find_by_team_id` mais reste dans `find_by_id`
- [ ] Test : la valeur d'équipe l'exclut
- [ ] `make check-arch` au vert, `make test` au vert
