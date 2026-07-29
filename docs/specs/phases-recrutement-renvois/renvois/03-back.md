# Renvois — Phase 3 : architecture back

**Entrée** : `02-front.md` validé.

Le panier, la trésorerie, les ports et la persistance sont **communs aux deux
pages** et décrits dans `recrutement/03-back.md`. Ce document ne consigne que les
écarts.

## Le panier de renvois

Même agrégat hydraté, même table `teams__phase_baskets` discriminée par
`phase = 'Dismissals'`, même version optimiste.

### Ce qu'il contient

| Champ | Origine |
|---|---|
| `team_id`, `version` | panier persisté |
| `lines: Vec<DismissalBasketLine>` | panier persisté — joueurs et staff marqués |
| `squad: SquadSnapshot` | port `players`, hydraté |
| `catalog: RosterCatalog` | port `references`, hydraté — pour le staff possédé |

**Pas de trésorerie.** Un renvoi ne rembourse rien : l'agrégat n'a aucune raison de la
connaître.

### Une seule garde

```rust
fn check_eligible_floor(&self, player_id: &PlayerId) -> Result<(), DomainError>
```

On ne descend pas sous **11 joueurs éligibles au prochain match**. Un joueur absent ne
comptant pas parmi les éligibles, le marquer n'entame pas le plancher : la garde ne
porte que sur les joueurs disponibles.

Toutes les gardes de composition du recrutement — plafond de 16, quota par poste,
limites croisées, trésorerie — sont ici **sans objet** : retirer ne peut violer aucune
borne haute.

Pour le staff, une seule vérification : ne pas marquer plus que ce que l'équipe
possède, lignes déjà en attente comprises.

## Domaine `teams` — ce qui manque

`PlayerFired` et `PlayerNotReEngaged` sont définis, appliqués, et **jamais
construits** : aucune méthode de `Team` ne les produit. Il faut les créer, comme
`PlayerRecruited` côté recrutement.

Et leurs bras `apply()` deviennent **vides** une fois la mutation de `team_value`
retirée par la carte 251 — ils ne faisaient que ça. À trancher au moment de coder :
les supprimer par cohérence avec « on supprime le code mort », ou les garder comme
contrat du licenciement à venir. Ce point est déjà noté dans la carte 251.

## Ports

Le besoin est **plus riche qu'au recrutement**, qui ne demandait que des compteurs par
ligne de roster. Ici il faut la liste nominative, donc `SquadMemberDto` s'étend
encore :

```rust
pub struct SquadMemberDto {
    pub player_id:                String,
    pub roster_line_id:           String,
    pub personal_name:            String,   // ← ajout renvois
    pub position_name:            String,   // ← ajout renvois
    pub spp:                      u32,      // ← ajout renvois
    pub value_kpo:                u32,
    pub available_for_next_match: bool,
}
```

C'est le **troisième élargissement** du même port — carte 250 pour la valeur d'équipe,
recrutement pour la ligne de roster, renvois pour l'identité. Il vaut la peine de le
nommer pour ce qu'il devient : un port de **consultation de l'effectif**, pas
seulement de sa valeur. `IPlayerValuePort` mériterait de s'appeler `ISquadPort`.

Aucun port vers `references` en plus : les prix du staff suffisent, et ils sont déjà
dans `RosterCatalogDto`.

## Fichiers

### Domaine

| Fichier | Contenu |
|---|---|
| `domain/dismissals_basket.rs` | agrégat, garde du plancher, `DismissalBasketLine` |
| `domain/team.rs` | méthodes produisant `PlayerDismissed` (renommage de `PlayerFired`, cf. `04-dtos.md`), retrait de `refund_kpo` de `dismiss_staff` |

### Use cases

`mark_basket_player`, `unmark_basket_player`, `mark_basket_staff`, `unmark_basket_staff`,
et `validate_dismissals_phase_use_case` (existant, rôle élargi à l'application du lot).

### IO — web

| Fichier | Rôle |
|---|---|
| `io/web/dismissals.rs` | page hôte |
| `io/web/widgets/dismissals_roster_widget.rs` | widget + les 2 POST `mark` |
| `io/web/widgets/dismissals_cart_widget.rs` | widget + les 2 POST `unmark` |
| `templates/dismissals.html` | page d'assemblage |
| `templates/widgets/dismissals-roster.html` | + fragments de ligne, trois états |
| `templates/widgets/dismissals-cart.html` | |

Le fragment d'erreur `basket-error.html` est **partagé** avec le recrutement.

### Migrations

Aucune en propre — tout est couvert par celles du recrutement.

## Purge des paniers orphelins

Décision D6, commune aux deux phases : un listener intra-BC abonné au bus interne de
`teams` supprime les deux paniers à chaque entrée en `ReadyToPlay`
(`TeamEnrolled`, `DismissalsPhaseValidated`, `MatchReportingCancelled`,
`CostlyMistakesApplied`).

`io/listeners/phase_basket_purge_listener.rs`, signature `init(event_bus: &EventBus, …)`
— c'est cette convention que `check-arch` (axe 5) utilise pour reconnaître un listener
intra-BC.

**Prérequis : la carte 251**, qui crée le bus interne de `teams` et la publication
depuis `TeamRepository::append`. Sans elle, il n'y a rien à écouter.

Un listener distinct de celui du recalcul de TV, bien qu'abonné aux mêmes événements :
une responsabilité, un listener.

## Règles métier identifiées à cette étape

- **Le plancher se recalcule à chaque hydratation.** Un joueur marqué hier peut être
  devenu indisponible entre-temps (dépublication de rapport), ce qui change le compte
  des éligibles et peut rendre la ligne invalide. Le refus en bloc de la décision D5
  s'applique.
- **Un joueur marqué reste dans l'effectif** jusqu'à la validation : il compte encore
  dans les éligibles tant que le lot n'est pas appliqué. C'est ce qui rend
  l'annulation gratuite.
- **Les SPP sont perdus à l'application du lot**, pas au marquage — donc annulable
  jusqu'au bout.

## Points ouverts pour la phase 4

- Le renvoi d'un joueur doit-il émettre un app event vers `players` pour que le joueur
  y soit marqué sorti, ou `players` réagit-il déjà à un événement existant ? À vérifier
  en phase 4 : `players` doit savoir que le joueur ne fait plus partie de l'équipe.
- Renommage de `IPlayerValuePort` en `ISquadPort` : à faire dans cette feature, ou
  laisser la carte 250 poser le nom définitif dès le départ ?
