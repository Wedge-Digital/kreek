# Un joueur mort quitte l'effectif visible et libère sa place

**Priorité : haute**
**Contexte :** `teams` (quotas) + `players` (affichage)
**Prérequis levé :** `SquadAdapter` délègue à `players` (commit `4ba7339`)

## Le constat

Le domaine sait qu'un joueur est mort — `PlayerParticipationStatus::Dead`
(`players/domain/match_impact.rs:33`), posé par `Player::apply` sur
`InjurySustained { injury_type: Mort }` (`player.rs:469`). Statut terminal,
jamais relevé, contrairement à `MissingNextGame` que le listener post-match
remet à `Available` (BR12).

Ce savoir se perd au passage vers `teams`. L'ACL écrase les quatre statuts en
un booléen :

```rust
// infrastructure/teams/squad_adapter.rs
fn is_available(participation_status: &str) -> bool {
    participation_status == "Available"
}
```

Dans `teams`, un mort, un blessé et un retraité sont donc indistinguables — et
trois gardes du panier de recrutement comptent les morts :

| garde | s'appuie sur | conséquence |
|---|---|---|
| `check_squad_max` (`recruitment_basket.rs:349`) | `Squad::size()` = `members.len()` | un mort mange une place sur seize |
| `check_position_quota` (`:356`) | `Squad::count_at()` | un Blitzer mort bloque le quota de Blitzers |
| `check_cross_limits` (`:369`) | idem | idem sur les limites croisées |

Ironie : `Squad::eligible_count()` (`basket.rs:140`) fait exactement le bon
filtre, mais ne sert qu'au compte des journaliers et au plancher des onze.

Côté affichage, trois écrans montrent les morts : le tableau de la fiche
d'équipe (`player_table_widget.rs:132`), le sélecteur de joueurs d'un rapport
de match (`match_player_selector_widget.rs:30`), et les comptes par poste de
`match_report` (`player_data_adapter.rs:79`). Aucun n'affiche de statut : le
mort y est un joueur ordinaire.

## Le point de conception

**Le booléen est le mauvais type.** Trois indisponibilités, deux traitements
opposés :

| statut | occupe une place ? | s'affiche ? | pourquoi |
|---|---|---|---|
| `MissingNextGame` | **oui** | oui | il revient au match suivant (BR12) |
| `Retired` | **oui** | oui | carte 39 : « il compte toujours dans les quotas tout au long de la saison » |
| `Dead` | **non** | **non** | — |

Filtrer sur `available_for_next_match` libérerait donc aussi la place d'un
blessé : c'est faux, et ça casserait BR12.

`Retired` n'est **pas** le renvoi. Le renvoi a son propre axe,
`membership: RosterMembership::{Active, Dismissed}`, écrit par
`player_dismissed_listener` — un renvoyé disparaît de toutes les lectures. Les
deux axes sont explicitement distincts depuis la carte 260.

## Qui filtre quoi

**`players` filtre ce qu'il montre ; `teams` décide ce qui occupe une place.**
Deux questions différentes, aucun des deux BCs ne peut répondre à celle de
l'autre.

Le quota reste dans le domaine de `teams`, conformément à la grille de décision
de `CLAUDE.md` (« ce quota est-il atteint ? » → Domaine). L'ACL **ne filtre
pas** : elle traduit. Le port porte déjà l'avertissement — *« la valeur
d'équipe ne somme que les disponibles. Un port qui filtrerait à la source
servirait l'un et trahirait l'autre »* (`teams/ports.rs:76`).

```rust
// teams/domain/ — le vocabulaire de teams, pas celui de players
pub enum SquadPresence { Alignable, Empeche, Perdu }
impl SquadPresence {
    pub fn occupe_une_place(&self) -> bool { !matches!(self, Self::Perdu) }
    pub fn alignable(&self) -> bool { matches!(self, Self::Alignable) }
}
```

`teams` ne connaît toujours pas le mot « mort » — il connaît ses deux seules
questions.

## Décisions prises

- **Le mort disparaît aussi de la page des renvois.** `dismissals_basket` lit
  le même `Squad` : filtrer dans le domaine l'y retire en même temps que de la
  fiche. Sa place est déjà libre, le renvoyer n'a plus d'objet. Sa fiche joueur
  individuelle reste consultable (`find_by_id` ne filtre pas).
- **`Retired` occupe sa place** — donc `Empeche`, comme `MissingNextGame`.
- **Le mort appelle un journalier.** Comportement actuel conservé :
  `team_value.rs:99` retranche les non-disponibles de `MATCH_SQUAD_SIZE`. Le
  trou est réel jusqu'au recrutement.

## Checklist

- [ ] `SquadPresence` dans `teams/domain/` + ses deux prédicats
- [ ] `Player.available_for_next_match: bool` → `presence: SquadPresence`
      (supprime un primitif nu d'une entité domaine — règle CQRS)
- [ ] `Squad::size()` et `count_at()` filtrent `occupe_une_place()`
- [ ] `Squad::eligible_count()` passe à `alignable()` — comportement inchangé
- [ ] `SquadMemberDto.available_for_next_match` → `presence`
- [ ] `squad_adapter::is_available` → `presence()`, table à trois branches
- [ ] Lecture dédiée dans `IPlayerProjectionRepository` excluant les morts —
      **pas** un filtre dans `find_by_team_id`, qui a six appelants aux besoins
      divergents (deux listeners qui rejouent l'équipe, l'édition d'effectif,
      l'ACL de `match_report`)
- [ ] La fiche d'équipe, le sélecteur de match et `find_player_counts_by_position`
      câblés sur cette lecture
- [ ] Tests unitaires `recruitment_basket` : quota de poste libéré par un mort,
      plafond de seize libéré, blessé toujours comptant, plancher des onze
      inchangé
- [ ] Test unitaire `squad_adapter` : les quatre statuts, trois présences
- [ ] Test e2e : tuer un joueur par un rapport de match, vérifier son absence
      de la fiche et la place rendue au recrutement
- [ ] Entrée du test dans `tests/impact-map.toml` (axe 8 de `check-arch`)

## Ce que la carte ne couvre pas

- **La phase de retraite temporaire** (carte 39). `Retired` reste un statut que
  personne ne pose ; la carte se contente de décider ce qu'il vaudra le jour où
  la 39 sera faite.
- **L'affichage d'un statut sur la fiche.** Le mort disparaît, il n'est pas
  badgé. Un écran d'historique d'effectif — qui montrerait morts et renvoyés —
  est un autre besoin.
- **Le journalier.** Son compte ne change pas.

## Terminé quand

Sur la base de démonstration : un joueur tué par un rapport de match n'apparaît
plus dans l'onglet Joueurs de sa fiche, et le poste qu'il occupait redevient
recrutable — alors qu'un joueur seulement blessé continue d'apparaître et de
bloquer son quota.
