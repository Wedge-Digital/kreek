# Le journalier joue vraiment

**Priorité : haute — fonctionnalité livrée qui ne marche pas**
**Épic :** E15 — Recruter un journalier (rouverte)
**Dépend de :** 454 à 459, toutes livrées
**Fichiers :** `src/app/match_report/io/app_events/app_event_publisher.rs`,
`src/app/players/io/app_events/player_recruited_listener.rs`,
`src/app/players/domain/{events,player}.rs`,
`src/app/players/io/repository/player_repository.rs`,
`tests/e2e/test_journeyman_recruitment.py`

## Le défaut, constaté en production

> Une équipe joue un match avec un journalier. Il marque **deux touchdowns**.
> Ses actions ne lui sont pas imputées, son score de SPP reste à zéro, et il n'y
> a donc rien à dépenser à la phase d'amélioration.

Vérifié en base : le journalier existe, maillot 13, statut `Available`, **0 SPP**.
Les deux TD sont bien dans le rapport de match. Ils n'en sortent jamais.

## La cause : une règle antérieure que l'épic n'a pas réconciliée

```rust
// app_event_publisher.rs:427
let ActionPlayer::Regular(player_id) = &a.player else {
    return None; // BR1 — stars/mercenaires/journaliers exclus
};
```

Ce n'est pas un oubli d'implémentation : c'est une règle **délibérée**, écrite
avant l'épic, et **un test la protège** —
`la_haine_d_un_journalier_ne_produit_aucun_app_event`, dont le commentaire dit
« ce test est là pour qu'on ne le défasse pas sans s'en apercevoir ».

Elle était juste tant qu'un journalier n'existait pas dans `players`. La carte
455 a changé ça, et **aucune des six cartes n'est revenue sur ce filtre**.

La conception le demandait pourtant, sans ambiguïté :

> Page 3851 : les journaliers **génèrent des SPP**. C'est ce qui rend l'embauche
> intéressante.
>
> Il participe comme un joueur ordinaire — **il agit, il gagne des SPP**, il peut
> prendre une amélioration à la phase prévue.

**Et le test qui l'aurait attrapé n'a jamais été écrit.**
`les_actions_du_rapport_pointent_le_joueur_reel` a été légué de la 455 à la 459,
puis déclaré couvert par `test_un_journalier_apparait_apres_un_match` — qui ne
vérifie que l'existence du joueur, jamais qu'une action l'atteint. Une dette
annoncée payée sans l'être.

## Conception

### 1. Le filtre s'ouvre au journalier — dans le publisher, pas dans le sélecteur

Le rapport de match a **raison** de traiter le journalier comme un remplaçant :
il en est un de son point de vue, et le sélecteur temporaire est sa place. Ce
qui a changé, c'est qu'il existe désormais dans `players`.

`build_player_ref`, quelques lignes plus bas, sait déjà faire la distinction :

```rust
TempPlayerKind::StarPlayer { .. } => …   // n'existe pas dans players
TempPlayerKind::Mercenary  { .. } => …   // n'existe pas dans players
TempPlayerKind::Journeyman { .. } => …   // son TempPlayerId EST son player_id
```

`build_player_impact_events` reçoit donc les joueurs temporaires et résout un
`ActionPlayer::Temp` en joueur réel **quand, et seulement quand**, il est
journalier.

**Toucher le sélecteur à la place le ferait apparaître deux fois** à l'écran de
saisie — une fois en régulier, une fois en temporaire. Le rapport garde son
modèle ; c'est la sortie du BC qui apprend à distinguer.

`la_haine_d_un_journalier_ne_produit_aucun_app_event` devient faux et doit être
**réécrit, pas supprimé** : il devient le test qui garde la distinction
vedette/journalier, celle qui compte encore.

### 2. Solitaire (4+) — un trait de naissance, perdu à l'embauche

`LONER_4` — « Solitaire (4+) » — existe déjà au corpus, catégorie `TRAITS`.

Le LRB, cité par la conception :

> Un Journalier embauché **perd le Trait Solitaire (X+)** et conserve les PSP
> gagnés pendant le match.

Il entre donc dans les compétences de base à la création, et `JourneymanHired`
l'en retire.

**C'est le seul cas de l'application où une compétence de base disparaît.**
L'agrégat et la projection doivent savoir le faire, et le commentaire doit dire
pourquoi ce cas est unique — sans quoi le prochain lecteur y verra une
incohérence à « simplifier ».

**`LONER_4` et non `LONER_3`** : le 3+ est celui des vedettes. Deux traits de
même nom et de seuils différents ; se tromper rend le journalier meilleur qu'il
ne doit être.

### 3. Le nom — « Journalier #13 »

`personal_name` est vide aujourd'hui, et l'affichage retombe sur le nom de
poste : **deux journaliers Trois-quart sont indiscernables** à l'écran de
recrutement comme dans l'effectif.

Le maillot est connu au moment de la création — `prochain_maillot_libre` le
calcule juste avant `creer_joueur` —, donc le nom se compose là.

**Il survit à l'embauche**, et c'est voulu : le coach reconnaît celui qu'il a
gardé, et peut le renommer comme n'importe quel joueur. Le remettre à vide à
l'embauche ferait disparaître le seul repère qu'il avait.

## Ce que la carte ne fait pas

- **Elle ne touche pas au jet de relance.** Solitaire (4+) est un trait passif
  qui s'applique quand le joueur veut utiliser une relance d'équipe. Le rapport
  de match ne modélise pas les relances : le trait est porté et affiché, son
  effet de jeu reste à la table.
- Elle ne change rien aux vedettes ni aux mercenaires, qui restent hors de
  `players` et hors des impacts.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `une_action_de_journalier_produit_un_impact` | le filtre ouvert |
| `une_action_de_vedette_n_en_produit_aucun` | la distinction gardée |
| `la_haine_d_un_journalier_atteint_players` | l'ancien test, retourné |
| `un_journalier_nait_avec_solitaire` | `LONER_4` dans les compétences de base |
| `l_embauche_retire_solitaire` | et lui seul — les autres restent |
| `un_journalier_est_nomme_avec_son_maillot` | « Journalier #13 » |
| **`test_le_journalier_marque_et_gagne_des_spp`** (e2e) | **la dette de la 455**, enfin |

Le dernier est celui qui compte : il enregistre un touchdown sur le journalier
dans le rapport, publie, et vérifie que ses SPP montent. C'est le test que la
455 devait écrire, que la 459 a déclaré couvert, et qu'aucune n'a produit.

## Checklist

- [x] `build_player_impact_events` résout un journalier en joueur réel
- [x] `la_haine_d_un_journalier…` réécrit, pas supprimé — **unitaire et e2e**
- [x] `LONER_4` aux compétences de base à la naissance
- [x] `JourneymanHired` le retire — agrégat **et** projection
- [x] `personal_name` = « Journalier #{maillot} »
- [x] Neuf tests, dont deux e2e
- [x] `make lint`, `make check-arch`, `make test` — 1709 tests
- [x] `make e2e` — 365 passés, 7 ignorés

## Ce qui a été fait

### Le filtre distingue par la nature, pas par l'identifiant

`joueur_reel` regarde `TempPlayerKind` : le journalier existe dans `players`
depuis la carte 455, la vedette et le mercenaire n'y existeront jamais. Le
rapport garde son modèle — le journalier reste un remplaçant de son point de
vue, et le sélecteur temporaire reste sa place.

### Deux tests affirmaient le contraire, pas un

Le premier était unitaire, et je l'avais repéré. **Le second était un fichier
e2e entier** — `test_haine_journalier.py`, dont le titre même disait « ne
rejoint aucun agrégat joueur ». Il a fallu la suite complète pour le trouver :
la vérification ciblée ne l'avait pas touché.

Les deux sont **retournés, pas supprimés**. Ce qu'ils gardent désormais est la
distinction qui subsiste — celle de la vedette.

### Le barème n'appartient pas au test

`test_le_journalier_marque_et_gagne_des_spp` affirmait d'abord qu'un touchdown
vaut trois SPP. Faux : le barème appartient à la compétition, et va de 2 à 6
selon celui qu'elle a choisi — la compétition du fixture utilise
`brawlin_brutes`, où le touchdown en vaut deux.

Le test affirme maintenant ce qu'il contrôle : le touchdown **atteint le
joueur**, et l'événement porte son nom. Combien il vaut se teste ailleurs.

### Un fixture à part pour les propriétés de naissance

`test_le_journalier_nait_avec_solitaire_et_un_nom` lisait d'abord le journalier
de `journalier_ctx` — que les scénarios de recrutement **embauchent**, et
l'embauche retire justement Solitaire. Il lisait l'état d'après. Un journalier
neuf lui est donc réservé.

### La constante vit dans le domaine

`SOLITAIRE_DU_JOURNALIER` était d'abord posée dans la couche qui crée les
joueurs. Le compilateur a refusé de l'importer dans l'agrégat — le domaine ne
dépend pas de l'IO —, et il avait raison : c'est une règle du jeu, pas un détail
de câblage.
