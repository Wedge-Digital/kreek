# Le journalier naît avec le rapport

**Épic :** E15 — Recruter un journalier
**Ordre :** 2 · **Dépend de :** 454
**Conception :** `docs/specs/embaucher-un-journalier/00-conception.md`

## Objectif

Qu'un journalier aligné dans un rapport de match **existe comme joueur**, avec
son maillot et son statut provisoire. C'est le renversement qui rend toute la
fonctionnalité possible.

## Pourquoi ce renversement

La première approche gardait le journalier hors de `players` jusqu'à son
recrutement. Elle a été abandonnée : **un joueur qui n'existe pas ne peut pas
s'améliorer**, et le LRB veut que son prix inclue « toute Hausse de Valeur
gagnée en vertu de ses améliorations ».

En naissant tôt, il agit, gagne des SPP et prend ses améliorations **comme un
joueur ordinaire**. Le recrutement ne fait plus que basculer son statut.

## Conception

### 1. `TempPlayerId` devient un `PlayerId`

`init_temp_players_use_case:254` engendre déjà un identifiant
(`ulid::Ulid::new()`). Il frappe désormais un vrai `PlayerId`.

**L'émetteur frappe**, comme le commentaire de `PlayerRecruited` l'exige :

> l'event store devient la source d'identité. Rejouer le flux redonne les mêmes
> joueurs, et un app event reçu deux fois est rejeté par la contrainte d'unicité
> au lieu de créer un doublon.

Les actions du rapport pointent alors directement le joueur réel.

### 2. `match_report` émet un fait

```rust
MatchReportAppEvent::JourneymenFielded {
    event_id, match_report_id, team_id, space_id,
    players: Vec<FieldedJourneyman>,   // player_id, roster_line_id
}
```

**Un fait de match, pas une décision d'effectif.** `match_report` constate qu'on
a aligné des journaliers ; il ne crée pas de joueurs — ce n'est pas son rôle.

**`FieldedJourneyman` ne porte pas de maillot** : `players` l'attribue à la
création, par `premier_libre`, qui seul connaît les numéros pris.

### 3. `teams` en tire la conséquence

Un listener écoute `JourneymenFielded` et émet `PlayerRecruited` pour chacun.

**`teams` reste le seul BC à faire naître un joueur.** Le chemin
`teams → players` demeure unique, et l'event store de `teams` raconte l'histoire
complète de son effectif — journaliers compris.

### 4. `players` crée en `Journeyman`

`player_creation.rs` fait déjà ce travail pour `TeamCreated` et
`PlayerRecruited`. Il gagne le statut :

```rust
starting_membership: RosterMembership::Journeyman
```

Le maillot vient de `premier_libre`, qui lit `jerseys_by_team_id` — la requête
que la carte 454 a élargie. **Sans la 454, un journalier prendrait un numéro
déjà occupé par un autre journalier.**

## La fenêtre asynchrone, et pourquoi on ne la traite pas

Le journalier naît à l'ouverture du rapport ; l'écran des actions arrive **deux
écrans plus loin**. Le temps d'action d'un humain dépasse de loin l'émission
d'un événement en mémoire, et un rafraîchissement rattraperait le cas limite.

**Le rapport ne dépend pas de `players` pour se dérouler** : il garde ses
`TempPlayer` et les affiche. Les deux mondes coexistent le temps du match, liés
par le `player_id` commun — ce n'est pas un doublon, c'est la même entité vue
par deux BCs.

## Ce que la carte ne fait pas

- **Aucun changement au déroulé du match.**
- **Aucune disparition** : c'est la carte 456.
- **Aucun recrutement** : c'est la 457.

Livrée seule, elle produit des journaliers qui s'accumulent — d'où l'ordre.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `un_journalier_aligne_devient_un_joueur` | la chaîne complète |
| `il_recoit_un_maillot_libre` | et non celui d'un coéquipier |
| `deux_journaliers_recoivent_deux_maillots` | `premier_libre` les voit l'un l'autre |
| `il_nait_en_membership_journeyman` | pas `Active` |
| `l_evenement_recu_deux_fois_ne_cree_qu_un_joueur` | la contrainte d'unicité |
| `les_actions_du_rapport_pointent_le_joueur_reel` | l'identifiant partagé |

## Checklist

- [ ] `TempPlayerId` frappe un `PlayerId`
- [ ] `JourneymenFielded` et `FieldedJourneyman`
- [ ] Le listener de `teams`, qui émet `PlayerRecruited`
- [ ] `player_creation` gagne le statut de naissance
- [ ] Les six tests
- [ ] `make lint && make test && make check-arch`
