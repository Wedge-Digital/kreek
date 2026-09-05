# Le panier accueille un journalier

**Épic :** E15 — Recruter un journalier
**Ordre :** 3 · **Dépend de :** 454 (qui a fait remonter `is_temporary` jusqu'au DTO)
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
        return Err(DomainError::MaxPlayersReached);
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

### 5. La limite de 16 change de définition — et c'est le cœur de la carte

**L'impasse à lever.** Un coach a seize membres, dont trois journaliers. Il lit
`16/16`, ses boutons d'achat sont grisés, et il ne peut pas recruter les
journaliers qu'il veut garder — **alors que les recruter est exactement ce qui
le libérerait**, puisqu'un journalier non recruté part de toute façon.

```rust
/// Seuls les PERMANENTS comptent. Un journalier du panier y entre — il devient
/// permanent. Ceux qui restent n'y sont pas : ils vont partir.
fn permanent_count(&self) -> usize {
    self.squad.permanent_size()
        + self.lines.iter().filter(|l| matches!(l,
            BasketLine::Player { .. } | BasketLine::Journeyman { .. })).count()
}
```

#### Le domaine doit d'abord savoir distinguer

C'est une règle métier : elle vit dans l'agrégat panier, et personne d'autre ne
peut la trancher. Or l'agrégat ne sait pas aujourd'hui qui est journalier.
`SquadMemberDto` porte `is_temporary` depuis la carte 454, mais
`to_domain_squad` (`basket_hydration_service.rs:190`) le laisse tomber, et le
`Player` du domaine `teams` n'a pas de champ pour l'accueillir.

**Ce n'est pas un booléen qu'on fait glisser, c'est un second axe** — et le code
a déjà payé pour l'apprendre. `SquadPresence` existe parce qu'un booléen s'était
trompé : *« un booléen `available_for_next_match` les confondait, et c'est ce qui
a laissé les morts occuper une place »*.

Les deux axes sont indépendants, et les quatre combinaisons existent : un
journalier alignable, un journalier blessé, un permanent alignable, un permanent
mort. C'est exactement le modèle que `players` porte de son côté —
*« Appartenance à l'effectif : un axe **distinct** de la participation »*.

```rust
/// Sa place est-elle acquise ? Un journalier tient une place qu'il va rendre :
/// au bout de la phase de recrutement, il est embauché ou il part.
pub enum SquadEngagement { Permanent, Journalier }
```

`Squad` gagne **`permanent_size()`**, à côté de `size()` — qui n'est pas
modifié. `size()` répond « qui occupe une place », question juste pour l'écran
de renvois, qui la garde.

#### Le compteur affiché suit la règle

`projected_squad_size` alimente **deux** choses : `squad_is_full`, qui grise les
boutons d'achat, et `squad_count`, le « x/16 » que le coach lit. Les deux
doivent bouger ensemble — sinon l'écran annonce `16/16` pendant que le bouton
fonctionne.

Le coach à seize dont trois journaliers lira désormais **13/16**, et c'est
précisément ce qui lui explique pourquoi il peut encore recruter.

L'écran de renvois, lui, ne bouge pas : il lit `Squad::size()`, et à la phase
de renvois les journaliers ont déjà disparu (carte 456).

### 6. Deux variantes d'erreur, pas une

*(Le plafond, lui, réutilise `MaxPlayersReached`, l'erreur existante : la carte
écrivait `SquadFull`, qui n'existe pas et ferait doublon pour la même règle.)*

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

- [x] `recruit_journeyman` et `JourneymanRecruited`
- [x] Le débit, motif `PlayerRecruitment`
- [x] Les trois variantes de `BasketLine` et `AppliedLine`
- [x] `SquadEngagement` sur `Player`, et le mapping dans `to_domain_squad`
- [x] `add_journeyman`, `permanent_count`, `Squad::permanent_size`
- [x] Le compteur `x/16` de l'écran suit la même règle que le plafond
- [x] Les deux variantes de `DomainError`, et `MaxPlayersReached` réutilisée
- [x] `basket_mutation::add_journeyman` et le bras de `build_events`
- [x] Treize tests — onze prévus, deux appelés par la mise en œuvre
- [x] `make lint`, `make check-arch`, `make test` — 1691 tests
- [x] `make e2e` — 357 passés, 6 ignorés

## Ce qui a été fait

**Le test central a été vu échouer.** `projected_squad_size` « simplifié » en
`squad.size()` fait tomber `seize_dont_trois_journaliers_autorisent_le_recrutement`
— exactement le scénario annoncé, celui qui compile et paraît juste.

### Le second axe a payé tout de suite

Le compilateur a exigé une décision à **sept** endroits en ouvrant
`SquadEngagement` et la troisième variante de panier : les deux accesseurs de
`BasketLine`, le rejeu de `validate_all`, le lot de `build_events`, l'affichage
du panier, et les mouvements de trésorerie. Un booléen n'en aurait forcé aucun.

C'est la troisième fois de l'épic qu'un `match` exhaustif tient un verrou que
les `grep` de contrôle ne voient pas — après `guard_active` (454) et
`treasury_movement` (455).

### Deux choses que la carte ne prévoyait pas

**Le panier expose ses journaliers en deux listes.** `hireable_journeymen()`
pour ce que l'écran proposera, `journeymen_in_basket()` pour nommer les lignes
déjà posées : le nom d'un journalier n'est pas dans le panier, il vit dans
l'effectif, et la ligne l'y retrouve par son identifiant.

**Le prix est relu sur l'effectif du jour à la validation**, pas repris de la
ligne. C'est ce qui fait tomber le journalier dont la valeur a bougé depuis
l'ajout au même endroit que tous les autres refus — au refus en bloc, avec sa
ligne nommée.
