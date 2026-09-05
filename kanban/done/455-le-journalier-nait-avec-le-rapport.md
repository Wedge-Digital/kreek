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

### 3. `teams` en tire la conséquence — par `JourneymanFielded`, pas `PlayerRecruited`

Un listener écoute `JourneymenFielded` et émet, pour chacun, un **nouvel
événement domaine** :

```rust
TeamDomainEvent::JourneymanFielded { player_id, roster_line }
```

**`PlayerRecruited` ne peut pas servir**, et pour deux raisons indépendantes
qui disent la même chose :

| | |
|---|---|
| `Team::recruit_player` (`team.rs:866`) ouvre par `expect_phase(Recruitment)` | un journalier est aligné en `MatchReporting` — l'appel serait **rejeté systématiquement** |
| `PlayerRecruited` produit `TreasuryMovement::debit(cost_kpo, PlayerRecruitment)` (`team.rs:399`) | à `cost_kpo: 0`, rien n'est prélevé, mais **rien ne filtre les mouvements nuls** : le grand livre gagnerait une ligne « Recrutement de joueur — 0 kPo » par journalier et par match |

`PlayerRecruited` signifie « le coach a acheté un joueur pendant sa phase de
recrutement ». Un journalier est **aligné**, gratuitement, provisoirement.
Réutiliser l'événement ferait mentir l'event store de `teams` sur ce qui s'est
passé — et salirait un écran livré (onglet Trésorerie, cartes 434-436) sans
rien casser, donc sans que personne ne le voie.

`JourneymanFielded` ne porte **pas de `cost_kpo`** : aucun mouvement de
trésorerie n'en découle. Sa garde de phase est `MatchReporting`.

**`teams` reste le seul BC à faire naître un joueur.** Le chemin
`teams → players` demeure unique, et l'event store de `teams` raconte l'histoire
complète de son effectif — en distinguant cette fois « j'ai recruté » de « j'ai
aligné un journalier », qui ne sont pas le même fait.

### 4. La refrappe, et l'événement de retrait qu'elle impose

**`TempPlayersInitialized` peut être émis plusieurs fois pour la même équipe.**
Il est déclenché par l'enregistrement des coups de pouce
(`record_inducements_use_case:395`), et `reset_if_needed` existe précisément
parce que le coach peut y revenir : à chaque passage, les journaliers sont
réinitialisés et **leurs ULID refrappés** (`init_temp_players_use_case:254`).

Sans traitement, le second passage crée une **seconde fournée** de journaliers.
Les premiers restent, occupant leurs maillots et comptant dans la valeur
d'équipe, pour un match où plus personne ne les aligne.

**L'idempotence invoquée plus haut ne couvre pas ce cas** : elle protège du
*même* événement reçu deux fois, pas de deux événements portant des
identifiants différents.

La réponse est symétrique :

```
TempPlayersInitialized  →  JourneymenFielded    →  JourneymanFielded    →  crée
TempPlayersReset        →  JourneymenWithdrawn  →  JourneymanWithdrawn  →  retire
```

Chaque fait de match a sa conséquence d'effectif, et l'event store des deux BCs
raconte les deux. Le retrait vise **les journaliers de cette équipe pour ce
rapport** — pas tout l'effectif.

**Ce n'est pas la carte 456.** Celle-ci retire un journalier *aligné puis
désaligné avant le match* ; la 456 retire celui qui *a joué et n'a pas été
recruté*. Les deux gestes se ressemblent et n'ont ni la même cause ni le même
moment.

### 5. `players` crée en `Journeyman`

`player_creation.rs` fait déjà ce travail pour `TeamCreated` et
`PlayerRecruited`. Il gagne le statut :

```rust
starting_membership: RosterMembership::Journeyman
```

**`PlayerCreated` est un événement persisté**, et le champ doit donc porter
`#[serde(default)]`, avec un `Default` valant `Active` : sans lui, les joueurs
déjà en base cessent de se rejouer. Le défaut est juste par construction — tous
les événements existants décrivent des joueurs embauchés. La convention est
déjà en place ailleurs (`MatchReportCancelled::pairing_id`).

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
| `aligner_un_journalier_ne_touche_pas_la_tresorerie` | le grand livre ne gagne **aucune** ligne |
| `un_joueur_d_avant_la_migration_se_rejoue_en_active` | le `serde(default)` |
| `une_refrappe_ne_laisse_pas_de_journalier_orphelin` | le couple retrait/création |
| `l_effectif_evenementiel_inclut_les_journaliers` | la dette laissée par la 454 |

Les quatre derniers ne figuraient pas au plan d'origine. Le premier couvre un
défaut qui **ne casse rien** — il salit un écran, et c'est ce qui le rend
invisible à la relecture. Le dernier est écrivable pour la première fois : la
carte 454 l'avait laissé en dette faute d'un événement produisant un
journalier.

## Checklist

- [x] `TempPlayerId` frappe un `PlayerId` — il frappait déjà un ULID valide
- [x] `JourneymenFielded` et `FieldedJourneyman`, publiés par `match_report`
- [x] `JourneymenWithdrawn`, son pendant sur `TempPlayersReset`
- [x] `TempPlayersReset` **nomme** les journaliers qu'il retire
- [x] `TeamDomainEvent::JourneymanFielded` et `JourneymanWithdrawn`, **sans
      `cost_kpo`**, donc sans mouvement de trésorerie
- [x] Le listener de `teams`, et son publisher
- [x] `PlayerCreated` gagne `starting_membership`, avec `#[serde(default)]`
- [x] `player_creation` transmet le statut de naissance, en paramètre explicite
- [x] Huit tests sur dix — les deux derniers reportés en `459` (voir ci-dessous)
- [x] `make lint`, `make check-arch`, `make test` — 1677 tests
- [x] `make e2e` — 357 passés, 6 ignorés

## Ce qui a été fait

**Le test du `serde(default)` a été vu échouer.** Attribut retiré, un
`PlayerCreated` d'avant l'épic cesse de se relire — c'est le seul défaut de
cette carte dont l'échec ne se serait vu qu'en production, sur tous les joueurs
existants.

### `TempPlayersReset` a dû apprendre à nommer

Le publisher s'exécute **après** l'append : quand il lit l'événement, l'agrégat
ne porte déjà plus les journaliers. Sans la liste dans l'événement, le retrait
n'aurait eu personne à viser, et le couple symétrique n'aurait rien réglé.

Le champ est persisté, donc `#[serde(default)]` lui aussi. Les resets déjà
écrits rendent une liste vide, ce qui est juste : ils n'avaient rien créé dans
`players`, ils n'ont rien à y retirer.

### Le désalignement passe par le listener de recrutement

Et non par celui du renvoi. Les deux naissances — le joueur acheté et le
journalier aligné — partagent le même corps et la même attribution de maillot ;
seul le statut diffère. Le désalignement est le pendant exact de la naissance,
il vit donc à côté d'elle. Le garde `is_active()` du listener de renvoi reste
strict, comme la carte 454 l'a décidé.

### Le `match` exhaustif, deuxième fois en deux cartes

`Team::treasury_movement` n'a pas de joker : ajouter les deux événements l'a
cassé, et **c'est lui qui a posé la question du mouvement de trésorerie**. La
carte 454 avait connu la même chose avec `guard_active`. Deux fois de suite,
c'est le compilateur qui a tenu le verrou là où les `grep` de contrôle ne
voyaient rien.

### Les deux tests reportés

`un_journalier_aligne_devient_un_joueur` et
`les_actions_du_rapport_pointent_le_joueur_reel` **n'ont de sens qu'en bout de
chaîne** : ils traversent trois BCs et deux bus, et aucun test unitaire ne les
voit d'un bout à l'autre. Ils sont donc écrits par la carte `459`, dont
`test_un_journalier_apparait_apres_un_match` les couvre exactement.

C'est la même mécanique que la dette que la 454 avait laissée à celle-ci, et
qui est réglée ici : une carte de socle ne peut pas prouver ce qu'elle rend
seulement possible.
