# Écran de recrutement · Phase 5 : couche applicative

**Phase 4** : `04-dtos.md`

## Un use case de plus, et deux qui s'étendent

```
teams/use_cases/
├── basket_mutation.rs                    ← add_journeyman, à ajouter
├── basket_hydration_service.rs           ← charge les recrutables
└── validate_recruitment_phase_use_case.rs ← un bras de plus dans build_events
```

**Aucun fichier neuf dans `teams`.** Le recrutement d'un journalier est une
mutation de panier, comme l'ajout d'un poste ou d'un staff — il rejoint ses
voisins plutôt que d'ouvrir un chemin parallèle.

## 1 · `basket_mutation::add_journeyman`

```rust
pub struct AddBasketJourneymanCommand {
    pub team_id: TeamId,
    pub player_id: PlayerId,
    pub expected_version: BasketVersion,
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn add_journeyman(
    cmd: AddBasketJourneymanCommand,
    space_id: &str,
    team_repo: &dyn ITeamRepository,
    basket_repo: &dyn IPhaseBasketRepository,
    catalog: &dyn IRosterCatalogPort,
    squad: &dyn ISquadPort,
) -> Result<(), BasketMutationError>
```

**Signature identique à `add_player`**, à la commande près. Le panier est ouvert
par `ouvrir_panier_recrutement`, muté, persisté avec sa version. Rien de neuf
dans l'orchestration.

`expected_version` porte déjà la concurrence : deux onglets ouverts sur le même
panier ne se marchent pas dessus, et le second reçoit un conflit.

## 2 · L'hydratation charge les recrutables

`basket_hydration_service.rs:59` appelle déjà `squad_port.find_squad(team_id)`.
Le DTO gagnant `is_temporary` (phase 4), les recrutables se déduisent **de la
lecture qui existe** :

```rust
let hireable: Vec<HireableJourneyman> = effectif.iter()
    .filter(|m| m.is_temporary)
    .map(|m| HireableJourneyman { player_id: …, price: Kpo(m.value_kpo) })
    .collect();
```

**Aucune lecture supplémentaire.** Et c'est cette liste qui porte le garde-fou :
un journalier retiré de l'effectif entre l'affichage et la validation n'y figure
plus, donc sa ligne de panier est rejetée (décision 14).

## 3 · `validate_recruitment_phase` — un bras de plus

Le commentaire de `build_events` donne la règle :

> Un événement **par ligne**, jamais un événement de lot : l'event store reste
> lisible — « ce joueur a été recruté tel jour » — et le grand livre de
> trésorerie en découle directement.

```rust
AppliedLine::Journeyman { player_id, cost } =>
    team.recruit_journeyman(player_id, cost),
```

**`recruit_journeyman` et non `recruit_player`.** Le second frappe un
identifiant neuf — `PlayerId::new()` — et porte une `base_value` distincte du
coût. Ici le joueur **existe déjà**, son identifiant vient du panier, et le prix
est sa valeur. Réutiliser la méthode obligerait à lui passer un identifiant
qu'elle n'est pas censée recevoir.

### Ce que le mouvement de trésorerie devient

`team.rs:399` fait déjà correspondre `PlayerRecruited` à un débit. Le nouvel
événement en a besoin d'un aussi :

```rust
TeamDomainEvent::JourneymanRecruited { cost_kpo, .. } =>
    Some(TreasuryMovement::debit(…, *cost_kpo, MovementReason::PlayerRecruitment)),
```

**Le même motif que `PlayerRecruitment`**, et non un neuvième motif : le grand
livre raconte « un joueur a été recruté », ce qui est vrai. Un motif distinct
obligerait la carte 435 — le relevé de trésorerie — à en connaître un de plus
pour dire la même chose.

## 4 · Ce qui se passe dans les autres BCs

Trois listeners neufs dans `players`, aucun use case.

| Écoute | Fait | Pourquoi pas un use case |
|---|---|---|
| `JourneymenFielded` | crée les joueurs en `Journeyman` | `player_creation.rs` existe déjà et fait ce travail pour `TeamCreated` et `PlayerRecruited` |
| `JourneymanRecruited` | bascule en `Active` | une ligne de projection, pas une orchestration |
| passage en `Dismissals` | passe les restants en `Dismissed` | idem |
| `MatchReportCancelled` | supprime les journaliers du rapport | idem |

**Le garde-fou de `JourneymanRecruited`** (décision 14) :

```rust
if player.membership == RosterMembership::Dismissed {
    tracing::warn!(player_id = %id,
        "journalier recruté après avoir été perdu — teams a débité, le joueur reste sorti");
    return;
}
```

Le débit a déjà eu lieu quand ce listener s'exécute. Il ne peut pas l'empêcher —
mais cette ligne de journal est **ce qui permettra de rembourser à la main**, et
c'est pour ça qu'elle porte le `player_id`.

## Les erreurs

`BasketMutationError` ne change pas : `Domain(DomainError)` couvre déjà le cas,
et `DomainError` gagne `JourneymanNoLongerAvailable` (phase 4).

**Aucune variante applicative neuve.** Un journalier disparu est un refus du
domaine, exactement comme un quota atteint — l'écran l'affiche au même endroit,
de la même façon.

## Ce que la couche applicative ne fait pas

- **Aucune lecture supplémentaire** : `find_squad` sert déjà tout l'écran.
- **Aucune transaction neuve** : `append_batch` écrit déjà le lot d'événements.
- **Aucun calcul de prix** : c'est `value_kpo`, lu tel quel (décision 9).
- **Aucun nettoyage de panier** : `validate_recruitment_phase` supprime déjà le
  panier de la phase.

## Une conséquence à ne pas manquer

Un journalier recruté **entre dans le lot d'événements de la validation de
phase**, aux côtés des recrutements ordinaires. Le dernier événement du lot fait
passer l'équipe en `Dismissals` — et c'est **ce passage** qui déclenche la perte
des journaliers non recrutés (décision 13).

L'ordre est donc garanti par la structure : on ne peut pas perdre un journalier
qu'on vient de recruter, puisque son basculement en `Active` précède le
changement de phase dans le même lot.

## Règles métier

**Aucune à préciser.** Les quinze décisions de `00-conception.md` couvrent la
fonctionnalité, et cette phase confirme la plus fine : la perte des journaliers
et leur recrutement ne peuvent pas se contredire, l'ordre du lot les sépare.
