# Renvois — Phase 4 : contrats de données

**Entrée** : `03-back.md` validé.

Les conventions et les types communs sont dans `recrutement/04-dtos.md`. Ce document
consigne les écarts — et une décision de fond que la phase 4 a rendue inévitable.

## 1. Décision — l'appartenance à l'effectif

`players` n'a **aucun moyen d'exprimer qu'un joueur a quitté l'équipe**. Ses statuts
sont `Available`, `MissingNextGame`, `Retired`, `Dead` ; ses quatorze événements
domaine ne couvrent que la création, les compétences, les faits de match et les
blessures.

Sans traitement, un joueur renvoyé resterait `Available` : il continuerait d'être
compté par le port de consultation d'effectif, donc dans la **valeur d'équipe** et
dans le **calcul des journaliers**. Le renvoi serait sans effet réel.

### Un axe distinct, pas un statut de participation

`PlayerParticipationStatus` décrit des **conséquences de match** — le code le dit
littéralement, `player.rs:70` porte le commentaire « Impact des rapports de match »
juste au-dessus. `Available` et `MissingNextGame` y sont posés par l'impact de match,
`Dead` par une blessure. *(`Retired` n'est posé nulle part : variante sans producteur,
jalon de la carte 39.)*

Un renvoi est une **décision de coach**, pas une conséquence de terrain. Il n'a rien à
faire sur cet axe.

```rust
// players/domain/player.rs
pub enum RosterMembership {
    Active,
    Dismissed,
}

pub struct Player {
    …
    pub membership: RosterMembership,          // ← appartenance à l'effectif

    // ── Impact des rapports de match ───────────────────────────────────────
    pub participation_status: PlayerParticipationStatus,
    …
}
```

**Pourquoi pas supprimer le joueur** : `players` est event-sourcé, on n'y supprime
rien. Le joueur reste avec ses SPP et son historique, il cesse simplement d'appartenir
à l'effectif.

### Un seul mot pour un seul fait

Le projet dit déjà **« dismissal »** partout : la phase `Dismissals`, le use case
`validate_dismissals_phase`, l'événement `DismissalsPhaseValidated`. Le seul écart est
`TeamDomainEvent::PlayerFired` — **jamais émis**, donc renommable sans coût.

| Couche | Nom |
|---|---|
| `teams`, domain event | `PlayerDismissed` *(renommage de `PlayerFired`)* |
| app event | `PlayerDismissed` |
| `players`, domain event | `PlayerDismissed` |
| `players`, état | `RosterMembership::Dismissed` |

Un fait, un nom, de bout en bout. Nommer identiquement des deux côtés ne contrevient
pas à la règle du CLAUDE.md : elle interdit de nommer un domain event d'après son
origine externe (`PlayerDismissedReceived`), pas de nommer le même fait pareil.

### Le flux

```
Renvois validés (teams)
   └─► domain event PlayerDismissed         (event store teams)
         └─► publisher teams (couche IO)
               └─► app event PlayerDismissed
                     └─► listener players
                           └─► domain event PlayerDismissed
                                 └─► players_proj.membership = 'Dismissed'
```

C'est une **propagation d'effet**, pas une consultation : `teams` a muté son agrégat,
`players` doit en tirer les conséquences. La grille du CLAUDE.md impose donc l'app
event, et l'émission passe obligatoirement par le publisher.

### Le filtre vit dans le repository, pas chez les appelants

Sept chemins lisent aujourd'hui l'effectif d'une équipe, et **tous les sept excluent
les renvoyés** — y compris l'affichage du roster : le coach n'a pas besoin de voir ses
joueurs renvoyés.

| Chemin | Traitement |
|---|---|
| `players/io/app_events/player_match_impact_listener.rs:178` | actifs seuls |
| `players/io/app_events/team_match_concluded_listener.rs:31` | actifs seuls |
| `infrastructure/match_report/player_data_adapter.rs:27` | actifs seuls — **carte 253 y touche déjà** |
| `infrastructure/match_report/player_data_adapter.rs:64` | actifs seuls |
| `infrastructure/teams/player_count_adapter.rs:19` | actifs seuls |
| `players/io/web/player_table.rs:99` | actifs seuls |
| `IPlayerValuePort` (carte 250) | actifs seuls |

Aucun cas d'usage ne demande les renvoyés. **`find_by_team_id` filtre donc à la
source**, et il n'y a pas de seconde méthode : une variante
`…_including_dismissed` serait du code mort le jour de sa création.

Conséquence directe : aucun appelant n'a de filtre à écrire, donc aucun ne peut
l'oublier. Le jour où un besoin d'historique apparaîtra, la variante se créera à ce
moment-là, avec son consommateur.

L'agrégat reste évidemment lisible par `find_by_id` : un joueur renvoyé conserve son
event store, ses SPP et son historique. Il ne disparaît pas, il cesse d'appartenir.

### Conséquence sur le port

`available_for_next_match` combine désormais **deux axes** : membre actif **et**
participation disponible. L'adapter fait la traduction — `teams` n'importe ni
`RosterMembership` ni `PlayerParticipationStatus`.

### Migration

`players_proj` gagne `membership TEXT NOT NULL DEFAULT 'Active'`. Aucun joueur n'a
jamais été renvoyé : le défaut suffit, il n'y a pas de reprise de données.

### Ce que cette décision ne règle pas

`Retired` **ne se débloque pas** pour autant. La carte 39 parle de retraite
*temporaire*, ce qui par définition n'est pas une fin d'appartenance — c'est
probablement un troisième état, ou une suspension. Le trancher au passage referait
l'erreur d'inventer un concept en marge d'une autre feature.

## 2. DTOs d'entrée

```rust
// io/web/widgets/dismissals_roster_widget.rs
#[derive(Deserialize)]
pub struct MarkPlayerBody {
    pub player_id: String,
    pub version:   i32,
}

#[derive(Deserialize)]
pub struct MarkStaffBody {
    pub staff_uid: String,
    pub version:   i32,
}
```

Le retrait réutilise `RemoveLineBody` du recrutement — même forme, même sémantique :
retirer une ligne du panier par son identifiant.

## 3. Commandes

```rust
pub struct MarkPlayerForDismissalCommand {
    pub team_id:          TeamId,
    pub space_id:         SpaceId,
    pub player_id:        PlayerId,
    pub expected_version: BasketVersion,
}

pub struct MarkStaffForDismissalCommand {
    pub team_id:          TeamId,
    pub space_id:         SpaceId,
    pub staff_type:       StaffType,
    pub expected_version: BasketVersion,
}
```

`UnmarkCommand` réutilise `RemoveBasketLineCommand`.

## 4. DTO de port — troisième élargissement

```rust
pub struct SquadMemberDto {
    pub player_id:                String,
    pub roster_line_id:           String,
    pub personal_name:            String,   // ← renvois
    pub position_name:            String,   // ← renvois
    pub spp:                      u32,      // ← renvois
    pub value_kpo:                u32,
    pub available_for_next_match: bool,
}
```

Carte 250 pour la valeur, recrutement pour la ligne de roster, renvois pour
l'identité : ce n'est plus un port de valeur mais un **port de consultation de
l'effectif**. Il devrait s'appeler `ISquadPort` — à trancher au moment de coder la
carte 250, pour éviter un renommage ultérieur.

## 5. VMs de sortie

### Trois états par ligne, contre deux au recrutement

```rust
pub enum DismissalActionVm {
    Removable { label: String },      // « Renvoyer »
    Marked    { label: String },      // « Annuler » — seule action réversible
    Blocked   { reason: String },     // « Minimum 11 »
}
```

`Marked` n'a pas d'équivalent au recrutement : là-bas une ligne ajoutée ne se retire
que depuis le panier, ici elle se retire aussi depuis la ligne du joueur.

### Effectif et panier

```rust
pub struct DismissalsRosterVm {
    pub context: DismissalsContextVm,
    pub players: Vec<PlayerRowVm>,
    pub staff:   Vec<StaffDismissalRowVm>,
    pub version: u32,
}

pub struct DismissalsContextVm {
    pub roster_name:   String,
    pub treasury_kpo:  u32,
    pub squad_count:   u8,
    pub eligible_count: u8,          // gouverne le plancher — distinct de squad_count
}

pub struct PlayerRowVm {
    pub player_id:  String,
    pub number:     u8,
    pub name:       String,
    pub position:   String,
    pub spp:        u32,
    pub value_kpo:  u32,
    pub is_available: bool,          // « Disponible » / « Absent »
    pub is_marked:  bool,            // ligne barrée
    pub action:     DismissalActionVm,
}

pub struct StaffDismissalRowVm {
    pub staff_uid: String,
    pub name:      String,
    pub owned:     u8,
    pub pending:   u8,               // affiché « −N » en rouge
    pub unit_value_kpo: u32,
    pub action:    DismissalActionVm,
}

pub struct DismissalsCartVm {
    pub lines:            Vec<CartLineVm>,   // réutilisé du recrutement
    pub squad_after:      u8,
    pub eligible_after:   u8,
    pub journeymen_needed: u8,               // 0 si ≥ 11 éligibles
    pub is_alert:         bool,
    pub cta_label:        String,            // « Valider 3 renvois → »
    pub cta_destructive:  bool,              // rouge dès qu'une ligne est en attente
    pub version:          u32,
}
```

| VM | Émis par | Consommé par |
|---|---|---|
| `DismissalsRosterVm` | `from_domain(&basket)` | `dismissals-roster.html` |
| `DismissalsCartVm` | `from_domain(&basket)` | `dismissals-cart.html` |
| `BasketErrorVm` | handlers | `basket-error.html`, **partagé** avec le recrutement |

Aucun `builders.rs` ici non plus : le panier porte ses données après hydratation.

## 6. Règles métier identifiées à cette étape

- **`squad_count` et `eligible_count` sont deux nombres distincts.** C'est le second
  qui gouverne le plancher ; les confondre rendrait le blocage incompréhensible.
- **`cta_destructive` suit l'état, pas la phase.** Le bouton n'est rouge que lorsqu'il
  va réellement détruire quelque chose.
- **`journeymen_needed` est informatif, jamais une conséquence des renvois** : le
  plancher interdit d'y descendre. Il ne peut refléter qu'un déficit déjà causé par
  les blessures.
- **La valeur du joueur est affichée alors qu'elle ne sera pas remboursée** : elle
  mesure ce que le coach s'apprête à perdre, pas ce qu'il va toucher.

## 7. Points ouverts pour la phase 5

- ~~Un rapport dépublié référençant un joueur renvoyé~~ — **cas impossible**,
  vérifié en phase 6 : la correction exige que les deux équipes soient encore en
  phase `PlayerImprovement`, or un renvoi suppose d'avoir atteint `Dismissals`.
- Le renvoi d'un joueur portant un numéro de maillot libère ce numéro. Le recrutement
  attribue « le premier disponible » : à confirmer que la libération est immédiate à
  l'application du lot, et non différée.
