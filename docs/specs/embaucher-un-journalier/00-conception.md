# Embaucher un journalier — la conception

**Issue d'un grilling du 2026-08-28.** Ce fichier fige les quinze décisions
prises, avant la maquette. Il précède les phases du workflow : les phases 2 à 8
s'y appuient.

## La règle du LRB

Séquence d'après-match, **étape 5**, après les recrutements et les renvois :

> Embauchez si vous le souhaitez un ou plusieurs des Journaliers qui ont joué
> pour votre équipe à ce match. Embaucher un Journalier coûte ses Frais
> d'Embauche, plus toute Hausse de Valeur éventuelle gagnée en vertu de ses
> améliorations. Un Journalier embauché perd le Trait Solitaire (X+) et conserve
> les PSP gagnés pendant le match. Dès lors, il est un joueur parmi les autres.
> **Tout Journalier qui n'est pas embauché de façon permanente est perdu.**

Et page 3851 : les journaliers **génèrent des SPP**. C'est ce qui rend
l'embauche intéressante.

## Le renversement — un journalier est un joueur dès sa naissance

La première approche envisagée gardait le journalier hors de `players` jusqu'à
son embauche, et reconstruisait un joueur à ce moment-là. Elle a été
**abandonnée** : elle ne savait pas porter les hausses de valeur, puisqu'un
joueur qui n'existe pas ne peut pas s'améliorer.

**La décision retenue** : le journalier est créé dans `players` **au tout début
du rapport de match**, avant le choix des coups de pouce. Il participe comme un
joueur ordinaire — il agit, il gagne des SPP, il peut prendre une amélioration à
la phase prévue.

Ce qui reste de son statut provisoire tient en une chose : **il doit être
embauché en phase de recrutement pour rester**, sinon il est renvoyé.

Ce renversement supprime trois problèmes d'un coup : plus besoin d'enrichir
`PlayerRefPayload`, plus besoin de porter une liste dans l'agrégat `Team`, plus
besoin de reconstruire un joueur à l'embauche.

## Le cycle de vie

```
DÉBUT DU RAPPORT                          PHASE DE RECRUTEMENT
match_report                              teams
  init_temp_players                         panier, 3ᵉ variante de ligne
  │ JourneymenFielded                       │ JourneymanHired { player_id, cost }
  │   { team_id, players: [...] }           ▼
  ▼                                       players
teams  ── PlayerRecruited ──►  players      membership : Journeyman → Active
  (le seul chemin de naissance)
                                          SORTIE DE LA PHASE
                                          players écoute le changement de phase
                                          et passe les restants en Dismissed
```

## Les quinze décisions

### Identité et naissance

| # | Décision |
|---|---|
| 1 | **`match_report` frappe le `PlayerId`** — la règle du projet veut que l'émetteur frappe, et ça donne la protection contre le doublon par la contrainte d'unicité. `TempPlayer.id` devient un `PlayerId`. |
| 2 | **Le journalier reçoit un maillot dès sa naissance** — il joue le match, il porte un numéro. Un non-embauché libère le sien en partant. |
| 3 | **`match_report` émet un fait, `teams` en tire la conséquence.** `JourneymenFielded` dit « ces journaliers ont été alignés » — un fait de match. `teams` reste **le seul** à faire naître des joueurs. |

### Statut

| # | Décision |
|---|---|
| 4 | **`RosterMembership` gagne une troisième variante** : `Journeyman`, entre `Active` et `Dismissed`. Le type existe déjà, une colonne le porte, 49 262 joueurs sont `Active`. |
| 5 | **Les journaliers sont visibles dans l'effectif et comptent dans la VE** pendant le match. C'est le principe même de l'approche. |
| 6 | **`SquadMemberDto` gagne `is_temporary`** — et **non** `is_journeyman`, qui existe déjà sur `RosterPositionDto` avec un tout autre sens (« ce poste est la ligne journalière du roster »). Deux homonymes contradictoires seraient une confusion assurée. |

### Embauche

| # | Décision |
|---|---|
| 7 | **L'embauche se fait en phase `Recruitment`**, avec les autres recrutements — le budget est commun, deux écrans laisseraient croire à chacun qu'il a toute la trésorerie. |
| 8 | **Une section à part, au-dessus du catalogue.** Les journaliers sont éphémères : un coach qui ne les voit pas les perd, alors qu'un poste du catalogue sera là au prochain match. |
| 9 | **Le prix est la valeur courante du joueur.** La formule du LRB — « frais d'embauche + hausse de valeur » — **est** la définition de `value_kpo`. Rien à calculer, `SquadMemberDto` le porte déjà. |
| 10 | **Une troisième variante de `BasketLine`** : `Journeyman { id, player_id, price }`, avec son pendant dans `AppliedLine` et son bras dans `validate_all`. |
| 11 | **La limite de 16 ne compte que les permanents.** Sinon un coach plein de journaliers ne pourrait embaucher personne — alors que les embaucher est ce qui le sortirait de l'impasse. |
| 12 | **L'embauche ne crée rien** : `JourneymanHired { player_id, cost_kpo }`, et `players` bascule le `membership`. Ni SPP ni améliorations à transmettre — elles sont déjà sur le joueur. |

### Disparition et gardes

| # | Décision |
|---|---|
| 13 | **`players` fait le ménage lui-même**, en écoutant la sortie de la phase `Recruitment`. `teams` n'a pas à tenir une liste qu'il ne fait que transmettre. |
| 14 | **Deux gardes, pas un.** Dans le panier, un journalier absent de l'effectif chargé est une `RejectedLine` — l'écran affiche l'erreur comme les autres. Et dans `players`, un `JourneymanHired` sur un joueur `Dismissed` est ignoré avec un `WARN` : le débit précède la réception, et cette ligne de journal est ce qui permettra de rembourser à la main. |
| 15 | **Annulation du rapport** → les journaliers sont **supprimés**, pas `Dismissed` : ils n'ont jamais joué, les garder polluerait l'effectif d'une trace de rien. `players` doit écouter `MatchReportCancelled`, ce qu'il ne fait pas aujourd'hui. |

## Ce que la dépublication ne défait pas

**Une embauche survit à une dépublication.** Le coach a payé, la décision lui
appartient. Ce qui se défait, ce sont les SPP et les blessures — que
`TeamMatchImpactReverted` gère déjà, et qui s'appliquent au journalier comme aux
autres puisqu'il est un joueur ordinaire.

Un journalier **non encore embauché** sur un rapport dépublié reste embauchable,
avec sa valeur recalculée : la correction a simplement changé ce qu'il vaut.

## La collision à documenter — `journeymen_value`

`team_value.rs:95` calcule aujourd'hui la valeur des journaliers **par
déduction** :

```rust
let missing = MATCH_SQUAD_SIZE.saturating_sub(available_count(players));
missing * journeyman_price.0
```

Dès que les journaliers sont de vrais joueurs, `available_count` les compte,
`missing` tombe à zéro, et la fonction **rend zéro**. Le résultat est juste — ils
sont comptés par `players_value` — mais par accident.

**La fonction est conservée** : hors match, aucun journalier n'existe, et la
déduction donne la VE théorique de l'équipe si elle jouait maintenant — ce que
le LRB exige (« les journaliers comptent toujours dans la Valeur d'Équipe »).

**Ce qui manque est un commentaire disant pourquoi elle rend zéro pendant un
match.** Sans lui, quelqu'un la croira morte et la supprimera, cassant la VE des
équipes hors match.

## Ce qui n'est pas traité, et pourquoi

**La fenêtre asynchrone entre l'alignement et l'existence du joueur.** Le
journalier naît au tout début du rapport ; l'écran des actions arrive deux
écrans plus loin. Le temps d'action d'un humain dépasse de loin l'émission d'un
événement en mémoire, et un rafraîchissement rattraperait le cas limite.

Le rapport continue d'afficher ses `TempPlayer` : **il ne dépend pas de
`players` pour se dérouler.** Les deux mondes coexistent le temps du match, liés
par le `player_id` commun — ce n'est pas un doublon, c'est la même entité vue
par deux BCs.
