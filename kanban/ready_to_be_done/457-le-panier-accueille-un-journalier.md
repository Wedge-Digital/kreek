# Le panier accueille un journalier

**Ordre :** 2 · **Dépend de :** 454
**Conception :** `docs/specs/embaucher-un-journalier/ecran-de-recrutement/`
(`05-use-cases.md`, `06-domaine.md`)

## Objectif

Le domaine du recrutement d'un journalier — panier, agrégat, erreurs, limite
d'effectif. **Aucun écran** : ses tests prouvent les règles avant qu'on puisse
cliquer.

## Conception

### 1. La méthode d'agrégat

```rust
pub fn recruit_journeyman(
    &self,
    player_id: PlayerId,
    cost_kpo: Kpo,
) -> Result<TeamDomainEvent, DomainError> {
    self.expect_phase(GamePhase::Recruitment)?;
    if self.treasury.0 < cost_kpo.0 {
        return Err(DomainError::InsufficientTreasury);
    }
    Ok(TeamDomainEvent::JourneymanRecruited { player_id, cost_kpo })
}
```

**Mêmes gardes que `recruit_player`**, mais **ni `roster_line` ni
`base_value`** : le joueur existe déjà, `players` sait tout de lui. `teams` ne
transporte que ce qu'il décide — la cible et le prix.

C'est le principe écrit sur `PlayerDismissed` : *« `players` possède le joueur,
il sait tout de lui ; ce qu'il ignorait, c'est la décision. »*

### 2. Le mouvement de trésorerie garde son motif

```rust
TeamDomainEvent::JourneymanRecruited { cost_kpo, .. } =>
    Some(TreasuryMovement::debit(…, *cost_kpo, MovementReason::PlayerRecruitment)),
```

**`PlayerRecruitment`, et non un neuvième motif.** Le grand livre raconte « un
joueur a été recruté », ce qui est vrai. Un motif distinct obligerait le relevé
de trésorerie (carte 435) à en connaître un de plus pour dire la même chose.

### 3. Trois variantes de panier

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

**`AppliedLine::Journeyman` n'a pas de `base_value`** : ici le prix **est** la
valeur courante. Ajouter un champ qui duplique l'autre inviterait à les faire
diverger.

`validate_all` rejoue le panier ligne par ligne sur une copie — un bras de plus
dans la boucle, et la validation tombe au même endroit que les autres.

### 4. Deux règles propres au journalier

```rust
pub fn add_journeyman(&mut self, player_id: PlayerId) -> Result<(), DomainError> {
    if déjà_dans_les_lignes(player_id) {
        return Err(DomainError::JourneymanAlreadyInBasket);
    }
    let Some(h) = self.hireable.iter().find(|h| h.player_id == player_id) else {
        return Err(DomainError::JourneymanNoLongerAvailable);
    };
    if self.permanent_count() >= MAX_SQUAD {
        return Err(DomainError::SquadFull);
    }
    …
}
```

**Un journalier ne s'ajoute pas deux fois.** C'est une règle que les postes
n'ont pas : un poste est un **type** — deux Trois-quarts sont deux joueurs — un
journalier est **un homme**, et il n'y en a qu'un.

**Le garde-fou vit dans le domaine et reste pur** : le panier compare son
contenu à la liste des recrutables qu'on lui a donnée, il n'interroge rien.

```rust
pub struct HireableJourneyman { pub player_id: PlayerId, pub price: Kpo }
```

Juste ce qu'il faut pour valider. Le nom, les SPP et l'amélioration sont de
l'affichage.

### 5. La limite de 16 change de définition

```rust
/// Seuls les PERMANENTS comptent. Un journalier du panier y entre — il devient
/// permanent. Ceux qui restent n'y sont pas : ils vont partir.
fn permanent_count(&self) -> usize {
    self.squad.permanent_len()
        + self.lines.iter().filter(|l| matches!(l,
            BasketLine::Player { .. } | BasketLine::Journeyman { .. })).count()
}
```

Sans cette règle, un coach à seize dont trois journaliers ne pourrait recruter
personne — **alors que les recruter est précisément ce qui le sortirait de
l'impasse.**

`Squad` gagne donc deux méthodes : `len()` pour la valeur d'équipe qui veut tout
le monde, `permanent_len()` pour le plafond.

### 6. Deux variantes d'erreur, pas une

```rust
JourneymanNoLongerAvailable,   // recharger la page
JourneymanAlreadyInBasket,     // regarder son panier
```

Une seule variante serait plus courte, mais **les deux causes se corrigent
différemment**. Un message unique enverrait chercher.

### 7. La mutation et la validation

`basket_mutation::add_journeyman` — signature identique à `add_player`, à la
commande près. `expected_version` porte déjà la concurrence.

`validate_recruitment_phase::build_events` gagne un bras :

```rust
AppliedLine::Journeyman { player_id, cost } => team.recruit_journeyman(player_id, cost),
```

**Un événement par ligne**, comme le commentaire de `build_events` l'exige.

L'hydratation ne fait **aucune lecture supplémentaire** : les recrutables se
déduisent du `is_temporary` de `find_squad`, déjà appelé.

## Tests

| Test | Règle |
|---|---|
| `recruter_hors_phase_echoue` | la garde de phase |
| `recruter_sans_tresorerie_echoue` | la garde de trésorerie |
| `l_evenement_ne_porte_ni_roster_line_ni_base_value` | la forme de l'événement |
| `le_meme_journalier_ne_s_ajoute_pas_deux_fois` | la règle propre |
| `un_journalier_absent_des_recrutables_est_refuse` | le garde-fou |
| `deux_journaliers_differents_s_ajoutent` | le cas passant |
| `seize_permanents_bloquent_le_recrutement` | le plafond |
| **`seize_dont_trois_journaliers_autorisent_le_recrutement`** | le cas qui donne son sens |
| `un_journalier_du_panier_compte_dans_le_plafond` | il devient permanent |
| `validate_all_rejette_un_journalier_disparu` | le garde-fou à la validation |
| `le_debit_porte_le_motif_player_recruitment` | le grand livre |

`seize_dont_trois_journaliers_autorisent_le_recrutement` échoue si quelqu'un
« simplifie » `permanent_count` en `squad.len()` — ce qui compilerait et
paraîtrait juste.

## Checklist

- [ ] `recruit_journeyman` et `JourneymanRecruited`
- [ ] Le débit, motif `PlayerRecruitment`
- [ ] Les trois variantes de `BasketLine` et `AppliedLine`
- [ ] `add_journeyman`, `permanent_count`, `Squad::permanent_len`
- [ ] Les deux variantes de `DomainError`
- [ ] `basket_mutation::add_journeyman` et le bras de `build_events`
- [ ] Les onze tests
- [ ] `make lint && make test && make check-arch`
