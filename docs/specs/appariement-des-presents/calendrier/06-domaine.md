# Calendrier · appariement des présents — domaine

## 1. Les règles métier — récapitulatif validé

| # | Règle | Où elle vit | Test prévu |
|---|---|---|---|
| R1 | Les présences ne sont pas mémorisées | nulle part : aucune table | — |
| R2 | Toutes présentes par défaut | gabarit | e2e |
| R3 | Le tirage se fait poule par poule | use case (un `DrawInput` par poule), `MatchDay::accept_draw` (aucune rencontre entre deux poules) | domaine, use case |
| R4 | Jamais deux équipes d'un même coach | `draw` (filtre préalable), `accept_draw` | domaine ×2 |
| R5 | Revanche seulement si inévitable, la plus ancienne d'abord | `draw` | domaine : 4 équipes, A-B A-C B-C jouées → 2 rencontres dont une revanche ; moins de revanches gagne ; la plus ancienne est préférée |
| R6 | L'aperçu ne se retouche pas | gabarit | e2e |
| R7 | « Générer une journée complète » tire parmi toutes les équipes | use case `generate_pairings` | use case |
| R8 | L'exemptée est celle qui a le plus joué | `draw` | domaine |
| R9 | Tirage seulement sur une journée de match sans rencontres | `MatchDay::can_receive_draw` | domaine |
| R10 | Moins de deux présents dans une poule : aucune rencontre ; aucune poule tirable : refus | `draw::draw_pools` | domaine |
| R11 | Tout est revérifié à l'écriture | `accept_draw`, `save_pairings` sous verrou | domaine, intégration |
| R12 | « Générer toutes les rencontres » passe par le même tirage | use case (inchangé, il appelle R7) | use case |
| R13 | Une rencontre tirée est une rencontre ordinaire | rien de nouveau | e2e (suppression après écriture) |
| R14 | « Générer une journée complète » est tout ou rien | `save_pairings` | intégration |
| — | C'est un vrai tirage : retirer au sort peut changer le résultat | `draw` | domaine (copié) |

## 2. L'agrégat `MatchDay` — forme validée

```rust
pub struct MatchDay {            // champs inchangés
    pub id: MatchId,
    pub season_id: SeasonId,
    pub name: MatchDayName,
    pub day_type: MatchDayType,
    pub date_start: Option<DateString>,
    pub date_end: Option<DateString>,
    pub position: MatchDayPosition,
    pub pairings: Vec<Pairing>,
}

impl MatchDay {
    // existantes : is_rest, engagement_existant, accueillir, liberer

    /// R9 — une journée de match, sans rencontres.
    pub fn can_receive_draw(&self) -> Result<(), DomainError>;

    /// R3, R4, R11 — accepte le tirage et y ajoute ses rencontres, ou refuse
    /// sans rien changer. Appelle d'abord `can_receive_draw`.
    pub fn accept_draw(
        &mut self,
        pairings: Vec<Pairing>,
        exempted: Option<TeamId>,
        rules: &DrawRules,
    ) -> Result<(), DomainError>;

    // privées : check_team(…), check_pair(…), check_exempted(…)
}

/// Ce que la journée ne sait pas d'elle-même, fourni par le use case.
pub struct DrawRules {
    pub enrolled: HashSet<TeamId>,
    pub forbidden: HashSet<(TeamId, TeamId)>, // même coach
    pub pools: Vec<HashSet<TeamId>>,          // appartenance, sans identifiant
}
```

**Les poules sont des ensembles, sans identifiant.** La poule de repli « Toutes
les équipes » porte aujourd'hui l'identifiant `"default"`, qui n'est pas un
`RankingGroupId` valide ; et R3 n'a besoin que de savoir si deux équipes sont
dans le même ensemble.

**La fuite d'état préexistante n'est pas corrigée ici.** Tous les champs de
`MatchDay` sont `pub`, `pairings` compris : onze accès directs dans neuf
fichiers, qui contournent tout invariant posé par `accept_draw`. Son
encapsulation fait l'objet d'une carte à part.

## 3. `domain/draw.rs` — copié de `tirage.rs`, puis renommé

Copie exacte du fichier de la branche `sondage-presence` (règle 5), puis
renommage à l'outil (LSP), sans réécriture.

| Branche | Ici |
|---|---|
| `tirer` | `draw` |
| `DrawInput { equipes, historique, interdites, matchs_joues }` | `DrawInput { teams, history, forbidden, matches_played }` |
| `RencontresJouees`, `Historique`, `NombreDeRencontres`, `NombreDeMatchs` | `PlayedMeetings`, `History`, `MeetingCount`, `MatchesPlayed` |
| `Recherche`, `Cout`, `Solution` (privés) | `Search`, `Cost`, `Solution` |
| — | **nouveau** : `draw_pools(&[DrawInput], rng) -> Result<Vec<DrawProposal>, DomainError>`, qui porte R10 |

Ce qui part : le champ `ecartees` de `DrawProposal`, toujours vide (il ne
servait qu'au sondage), et le commentaire « c'est la DP », devenu faux.

## 4. Les erreurs ajoutées à `DomainError`

| Variant | Règle |
|---|---|
| `RestDayCannotBeDrawn` | R9 |
| `RoundAlreadyPaired` | R9, R11 |
| `NotEnoughPresent` | R10 |
| `TeamNotEnrolled { team }` | R11 |
| `TeamDrawnTwice { team }` | R11 |
| `SameCoachPairing { home, away }` | R4 |
| `PairingAcrossPools { home, away }` | R3 |
| `ExemptedTeamAlsoPaired { team }` | R11 |

## 5. Les tests unitaires prévus

**`draw.rs`** — les tests de la branche, copiés et renommés, dont : R5 sur
quatre équipes, moindre total de revanches, revanche la plus ancienne ; R4 même
au prix d'une rencontre en moins ; R8 ; l'aléa. Plus `draw_pools` : poule à un
présent sans rencontre, aucune poule tirable → `NotEnoughPresent`.

**`MatchDay`** — `can_receive_draw` refuse une journée de repos et une journée
pourvue ; `accept_draw` refuse chacun des cinq cas de R3, R4 et R11, et
**ne modifie pas la journée** quand il refuse ; il ajoute toutes les
rencontres quand il accepte.
