# Écran de recrutement · Phase 3 : architecture back

**Phase 2** : `02-front.md` · **Conception** : `../00-conception.md`

## Le vrai sujet : quatre requêtes filtrent `membership = 'Active'`

Un journalier porte `membership: Journeyman` (décision 4). Or **quatre lectures
excluent aujourd'hui tout ce qui n'est pas `Active`**, et elles ne veulent pas
la même chose :

| Requête | Ce qu'elle sert | Le journalier doit-il y figurer ? |
|---|---|---|
| `projection_repository.rs:29` — l'effectif complet | la fiche d'équipe, la page joueurs | **oui** — il est visible pendant le match (décision 5) |
| `projection_repository.rs:130` — les maillots pris | l'attribution d'un numéro libre | **oui** — il en porte un (décision 2) |
| `projection_repository.rs:148` — le compte des disponibles | `11 − N` pour le rapport suivant | **oui** — sinon on recrée des journaliers pour combler des journaliers |
| `squad_adapter.rs:47` — l'effectif vu par `teams` | la valeur d'équipe, le panier | **oui** — il compte dans la VE (décision 5) |

**Les quatre répondent « oui ».** Le filtre n'est donc pas à assouplir au cas par
cas : il devient `membership <> 'Dismissed'`.

C'est le changement le plus large de la fonctionnalité, et le plus silencieux —
**aucun compilateur ne le signalera**, ce sont des chaînes SQL. Une requête
oubliée donne un journalier invisible là où il devrait compter, sans erreur.

### La forme retenue

```sql
WHERE team_id = $1 AND membership <> 'Dismissed'
```

Et non `IN ('Active','Journeyman')` : la liste devrait être tenue à jour à
chaque nouvelle variante, alors que la question posée est bien « ce joueur
fait-il encore partie de l'effectif ? ».

**Le seul filtre qui reste explicite** est celui du panier, qui doit distinguer
les recrutables des permanents — et il le fait sur `is_temporary`, pas sur une
liste de statuts.

## Ce que `players` gagne

### La troisième variante

```rust
pub enum RosterMembership { Active, Journeyman, Dismissed }
```

Le type existe, une colonne le porte, **49 262 joueurs** sont `Active` et 104
`Dismissed`. Aucune migration de données : la variante s'ajoute, les lignes
existantes ne bougent pas.

**Un seul site filtre dessus en Rust** — `update_roster_use_case:64`, qui garde
`== Active` : réordonner l'effectif ne concerne pas les journaliers, ils partent
ou deviennent permanents.

### Deux événements de domaine

```rust
PlayerBecamePermanent { player_id }   // le recrutement
PlayerJourneymanLost  { player_id }   // la sortie de phase, ou l'annulation
```

Le second **n'est pas** `PlayerDismissed` : un journalier perdu n'a pas été
renvoyé par une décision du coach, il n'a simplement pas été retenu. Confondre
les deux ferait apparaître ces joueurs dans l'historique des renvois.

### Trois listeners, dont deux neufs

| Écoute | Fait |
|---|---|
| `TeamsAppEvent::JourneymenFielded` *(neuf)* | crée les joueurs, `membership: Journeyman` |
| `TeamsAppEvent::JourneymanRecruited` *(neuf)* | bascule en `Active` — **sauf si `Dismissed`** (décision 14) |
| le passage en phase `Dismissals` *(neuf)* | passe les `Journeyman` restants en `Dismissed` |
| `MatchReportCancelled` *(neuf)* | **supprime** les journaliers de ce rapport (décision 15) |

Le dernier est le seul qui supprime plutôt que de marquer : ces joueurs n'ont
jamais joué, et les garder en `Dismissed` polluerait l'effectif d'une trace de
rien.

## Ce que `match_report` gagne

**Un fait, pas une décision** (décision 3) :

```rust
MatchReportAppEvent::JourneymenFielded {
    team_id, space_id,
    players: Vec<FieldedJourneyman>,   // player_id, roster_line_id, jersey
}
```

Émis par `init_temp_players_use_case`, qui frappe déjà les identifiants —
`TempPlayerId` devient un `PlayerId` (décision 1).

**`teams` l'écoute et émet `PlayerRecruited`**, restant le seul BC à faire naître
un joueur. Le chemin `teams → players` demeure unique.

### Le maillot, et qui l'attribue

`premier_libre` vit dans `players` (`player_creation.rs:141`), et lit les
maillots pris par `jerseys_by_team_id` — la requête ligne 130, qui verra donc
les journaliers une fois le filtre élargi.

**`match_report` n'attribue pas le numéro** : il ne connaît pas les maillots
pris. Il émet le fait, et `players` attribue à la création, comme pour tout
joueur.

## Ce que `teams` gagne

### `SquadMemberDto` gagne un champ

```rust
pub struct SquadMemberDto {
    …,
    pub is_temporary: bool,      // membership == Journeyman
}
```

**`is_temporary` et non `is_journeyman`** (décision 6) : ce dernier existe déjà
sur `RosterPositionDto` avec un tout autre sens — « ce poste est la ligne
journalière du roster ». Deux homonymes contradictoires seraient une confusion
assurée.

Une seule lecture continue donc de servir la valeur d'équipe, le panier et la
liste des recrutables.

### L'amélioration prise — la question ouverte en phase 2, tranchée

Le DTO ne la porte pas. Deux voies avaient été envisagées ; la base tranche :

```json
acquired_skills: [{"skill_id": "APPUI_FERME", "skill_name": "Appui Ferme",
                   "spp_cost": 6, "mode": "Chosen", "category_css": "type-general"}]
```

**`players_proj` porte déjà le nom de la compétence**, et les cinq deltas de
caractéristique (`ma_delta` … `av_delta`). Sur 49 000 joueurs, 298 ont une
compétence acquise et 47 une caractéristique améliorée.

Déduire l'amélioration de l'écart de valeur donnerait « +20 » sans dire
« Blocage » — or c'est le nom qui intéresse le coach. **Le DTO gagne donc le
libellé**, lu directement :

```rust
pub improvement_label: Option<String>,   // « Blocage », « +1 ST », None
```

Une amélioration au plus par journalier : il n'a joué qu'un match, et le premier
palier coûte 6 SPP.

### La troisième variante du panier

```rust
pub enum BasketLine {
    Player { … }, Staff { … },
    Journeyman { id: BasketLineId, player_id: PlayerId, price: Kpo },
}
pub enum AppliedLine {
    Player { … }, Staff { … },
    Journeyman { player_id: PlayerId, cost: Kpo },
}
```

`validate_all` **rejoue le panier ligne par ligne sur une copie** — un bras de
plus dans la boucle, et la validation tombe au même endroit que les autres.

**Le garde-fou de la décision 14 vit là** : le panier reçoit la liste des
journaliers recrutables (les `is_temporary` de l'effectif chargé), et une ligne
qui n'y figure plus est une `RejectedLine` — l'écran affiche l'erreur comme il
affiche « effectif complet ».

### La limite de 16

```rust
const MAX_SQUAD: usize = 16;
```

Elle ne compte que les **permanents** (décision 11). `Squad` doit donc distinguer
ce qu'elle compte : les `is_temporary` sont exclus du plafond, mais un journalier
**recruté dans le panier** y entre — il devient permanent.

Sinon un coach à seize dont trois journaliers ne pourrait recruter personne,
alors que les recruter est précisément ce qui le sortirait de l'impasse.

## `journeymen_value` — la collision, et le commentaire qui manque

`team_value.rs:95` calcule la valeur des journaliers **par déduction** :

```rust
let missing = MATCH_SQUAD_SIZE.saturating_sub(available_count(players));
missing * journeyman_price.0
```

Dès que les journaliers sont de vrais joueurs, `available_count` les compte,
`missing` tombe à zéro, la fonction **rend zéro** — et le résultat reste juste
puisque `players_value` les compte.

**La fonction est conservée** : hors match, aucun journalier n'existe, et la
déduction donne la VE théorique de l'équipe si elle jouait maintenant — ce que
le LRB exige.

**Ce qui manque est un commentaire disant pourquoi elle rend zéro pendant un
match.** Sans lui, quelqu'un la croira morte et la supprimera, cassant la VE de
toutes les équipes hors match.

## Ce que le back ne fait pas

- **Aucune migration de données** : la variante s'ajoute, aucune ligne ne bouge.
- **Aucune table neuve.**
- **Aucun changement au déroulé du match** : le rapport garde ses `TempPlayer`
  et ne dépend pas de `players` pour se dérouler.

## Règles métier

**Aucune à préciser.** Cette phase confirme la plus structurante : le filtre
`membership = 'Active'` devient `<> 'Dismissed'` aux quatre endroits, et c'est
le seul changement qu'aucun compilateur ne signalera.
