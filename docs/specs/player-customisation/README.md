# Player customisation — Spec index

Mode d'administration sur la fiche joueur : modifier un joueur **hors des
règles du jeu** — compétences, caractéristiques, prix, SPP. Maquetté dans
`assets/rawpages/html/app-player-detail-readonly.html` (Phase 1 validée,
commit `8d060e8`).

Contexte : le bouton « ✎ Customiser » existe déjà dans `player-detail.html`,
`disabled`, avec l'infobulle « Fonctionnalité à venir ». `can_customise` est
déjà calculé par `player_detail_controller.rs`, **et sur le bon périmètre** —
`check_admin_rights` vérifie admin d'espace puis admin de compétition, sans le
coach. Il n'y a donc rien à resserrer.

## Pages

| Page | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| player-detail | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (8 cartes : 302-309) |

## Règles métier (identifiées phase 1)

**Un événement par customisation**, propre au BC `players`, pour les quatre
familles : SPP, prix, caractéristiques, compétences. Les événements existants
ne sont pas touchés — l'historique du joueur distingue ainsi l'origine de
chaque modification, et le journal peut afficher sa pastille `🛠️` sans
deviner.

**Chemin d'application dédié et indépendant.** Les customisations s'appliquent
séparément de toutes les autres évolutions, afin d'être **toujours**
appliquées — quels que soient l'état du joueur, la phase de l'équipe, et les
règles qui gouvernent les évolutions normales.

**Une amplitude par modification.** Pas de liste de crans : une customisation,
une amplitude.

**Identifiant unique par customisation**, visant un joueur précis.

**Une customisation ne peut pas être supprimée.**

### Projection

**`players_proj` porte les deltas cumulés** par caractéristique, pas les
valeurs absolues. La base reste lue depuis `references` : aucune donnée de
référence dupliquée dans `players`, et pas de port applicatif injecté dans la
transaction d'append.

La projection doit être revue en ce sens — les bonus/malus de customisation
s'y cumulant avec ceux issus des résultats de match, aujourd'hui résolus
uniquement à la lecture par `player_stats_service::resolve_stats`.

**Le profil du joueur change** dès qu'une customisation est appliquée. Les
consommateurs qui l'interrogent par port sont donc à jour automatiquement —
ce qui **impose** que la customisation écrive `players_proj`, seule table que
lisent `teams::ISquadPort` et les adapters de `match_report`.

Si le profil venait un jour à être partagé par événement, il faudrait publier
un événement de mise à jour. Ce cas n'existe pas aujourd'hui :
`PlayersAppEvent` ne porte que `InitialRosterCompleted` et `PlayerDismissed`,
aucune donnée de profil.

### Validité d'une modification

**Bornes des caractéristiques** — Mouvement `0..9`, Force `0..9`,
Agilité `1+..6+`, Passe `1+..6+`, Armure `2+..12+`.

Une modification qui ferait sortir des bornes est **refusée visiblement**,
jamais ignorée en silence : le panier permettant d'empiler plusieurs
modifications, un écart muet à l'enregistrement ferait croire au commissaire
qu'il a appliqué ce qu'il n'a pas appliqué.

**Pas de compétence en double** — qu'elle soit de base, gagnée par dépense de
SPP, ou déjà ajoutée par customisation.

**Le prix ne descend pas sous 0.** La règle porte sur le **résultat**, pas sur
le delta : `player.value` étant un accumulateur, le domaine lit la valeur
courante avant de trancher.

**L'ajout de SPP est plafonné à 100 par opération** — c'est le montant ajouté
en une fois qui est borné, pas le total du joueur.

### Effet sur la valeur d'équipe

**Le prix déplace la TV**, et c'est voulu : `compute_team_value` somme les
`value_kpo` des joueurs disponibles, et la TV pilote les coups de pouce et
l'équilibrage des matchs.

**Ni les compétences ni les caractéristiques obtenues par customisation ne la
déplacent** — contrairement aux mêmes, acquises par dépense de SPP, qui
portent un `value_delta`.

Asymétrie assumée : deux joueurs aux compétences et caractéristiques
identiques n'auront pas la même valeur selon l'origine de leurs acquis. Le
prix est le seul levier de valeur du mode, et il est explicite. Écrit ici
parce que quelqu'un le prendra pour un bug s'il ne l'est pas.

### Autorisation

**Commissaire de ligue et admin d'espace uniquement.** Le coach de l'équipe ne
peut ni soumettre, ni **voir** le mode.

`can_customise` le fait déjà : `check_admin_rights` vérifie admin d'espace puis
admin de compétition, sans le coach. Une première rédaction de cette spec
affirmait le contraire — confusion avec `can_spend_spp`, qui lui est
explicitement « étendu au coach ». Rien n'est donc à resserrer, mais la règle
mérite un test : elle ne tient aujourd'hui qu'à la composition d'une fonction
que rien n'empêche d'élargir.

**Traçabilité nominative** — chaque action est journalisée avec le nom du
commissaire, comme l'annonce le bandeau du mode.

## Points ouverts

- **États non maquettés** : l'échec d'enregistrement (borne dépassée,
  compétence en double) et l'écran vu par un utilisateur sans droit. Le
  workflow demande que chaque état soit maquetté ; ces deux-là ne le sont pas.
- **Asymétrie de visibilité** : une customisation de prix ou de SPP est
  visible immédiatement par `teams` et `match_report`, qui lisent
  `players_proj`. Une customisation de caractéristique ne l'est aujourd'hui
  que sur la fiche joueur — c'est précisément ce que la refonte de la
  projection doit corriger, et il faudra vérifier qui d'autre en dépend.
