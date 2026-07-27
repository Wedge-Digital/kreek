# Phase 6 — Domaine · page Recap

## 1. Récapitulatif des règles métier (validé)

### Éligibilité

| # | Règle |
|---|---|
| 1 | Les 2 équipes doivent être en `GamePhase::PlayerImprovement` |
| 2 | Aucun SPP dépensé sur l'un des 2 effectifs |
| 3 | Le message de blocage nomme l'équipe qui bloque |
| 3a | Si les 2 camps bloquent, un seul message — côté home |
| 3b | Ordre d'évaluation : home SPP → home phase → away SPP → away phase |
| 9 | Garde-fou revérifié côté serveur au POST ; l'affichage n'est qu'un indice |
| 9a | La raison réaffichée est celle recalculée au POST |
| 12 | Échec d'un port du garde-fou → échouer fermé (`EligibilityUnknown`) |
| 16 | Équipe retirée / dissoute : imprécision du libellé acceptée — le blocage reste correct sur le fond, et une équipe dissoute rend la correction sans objet |

### Portée et cycle de vie

| # | Règle |
|---|---|
| 4 | Droits de correction = droits de publication |
| 5 | Pas de motif de correction obligatoire |
| 6 | Les équipes du rapport ne sont pas modifiables |
| 7 | Un rapport dépublié jamais republié reste en l'état |
| 8 | Nombre de corrections successives non limité |
| 10 | Notifications hors scope |
| 13 | `rehydrate()` supporte N alternances publier / dépublier |

### Compensation

| # | Règle |
|---|---|
| 11 | Échec partiel accepté, adossé à l'idempotence de chaque compensation |
| 14 | Les fans se restaurent **par instantané**, jamais par soustraction |
| 15 | Le statut de participation du joueur se restaure aussi |

---

## 2. BC `match_report`

### Value objects (`domain/value_objects.rs`)

`CorrectionEligibility`, `CorrectionBlocker` — définis en phase 4, amendés en
phase 5 (`EligibilityUnknown`).

### Erreur (`domain/error.rs`)

```rust
CorrectionNotAllowed(CorrectionBlocker),
```

Message `Display` construit sans nom d'équipe — le domaine ignore les noms
(phase 4). Le libellé destiné au coach est composé par `build_correction_zone`.

### Méthode domaine

```rust
// domain/match_report_published.rs
impl MatchReportPublished {
    pub fn unpublish(
        &self,
        unpublished_by: CoachId,
        eligibility:    CorrectionEligibility,
    ) -> Result<(MatchReportReadyToPublish, MatchReportDomainEvent), DomainError>
}
```

- `CorrectionEligibility::Blocked(b)` → `Err(DomainError::CorrectionNotAllowed(b))`
- sinon → l'état `ReadyToPublish` reconstruit depuis `self`, avec
  `was_published_before: true`, et l'événement `MatchReportUnpublished`

Symétrique de `MatchReportReadyToPublish::publish()`, à ceci près que `publish()`
est infaillible alors que `unpublish()` porte une garde.

### État — `was_published_before`

Champ ajouté sur `MatchReportReadyToPublish` **seul**, positionné par
`unpublish()` et mis à `false` par `from_pre_match()`.

> **Correction de la phase 4**, qui l'annonçait sur les deux états.
> `into_pre_match()` est une conversion transitoire interne aux use cases,
> jamais persistée : dans `rehydrate()`, un rapport en `ReadyToPublish` reste en
> `ReadyToPublish` et est muté en place pour tous les événements d'édition.
> Aucun événement ne le ramène vers `PreMatch`, donc le drapeau n'a rien à y
> faire.

### Machine à états — règle 13

Arête ajoutée dans `rehydrate()` :

```
(Some(Published(p)), MatchReportUnpublished { .. }) → ReadyToPublish
```

`rehydrate()` étant un `fold` sur le flux, l'alternance publier / dépublier se
traite **sans cas particulier** dès lors que les deux arêtes existent. La
règle 13 ne demande donc aucun compteur ni garde supplémentaire : elle est
satisfaite par construction. C'est un point à couvrir par un test, pas par du
code.

---

## 3. BC `teams`

### État dérivé — aucune migration d'event

```rust
struct LastPostMatch {
    match_report_id:       MatchReportId,
    dedicated_fans_before: DedicatedFans,
    treasury_income:       Kpo,
}

pub struct Team {
    // …existant…
    last_post_match: Option<LastPostMatch>,
}
```

Renseigné dans `apply(PostMatchSequenceStarted)`, **avant** que l'état ne soit
écrasé :

```rust
self.last_post_match = Some(LastPostMatch {
    match_report_id:       self.current_match_report_id,  // avant le clear
    dedicated_fans_before: self.dedicated_fans,           // avant l'écrasement
    treasury_income:       *treasury_income,
});
```

Deux informations qui semblaient perdues sont en réalité disponibles à cet
instant précis de l'hydratation :

- **`dedicated_fans_before`** — l'événement ne stocke que la valeur post-clamp,
  mais `apply()` voit encore la valeur d'avant. C'est ce qui satisfait la
  règle 14 sans toucher au schéma d'événement.
- **`match_report_id`** — absent de `PostMatchSequenceStarted`, mais
  `current_match_report_id` est encore renseigné (il est mis à `None` par ce
  même `apply`).

L'état dérivé est entièrement **rebuildable** depuis les événements existants :
aucune migration, aucune évolution de schéma.

### Événement

```rust
PostMatchSequenceReverted {
    match_report_id:  MatchReportId,
    dedicated_fans:   DedicatedFans,  // valeur absolue restaurée (règle 14)
    treasury_refund:  Kpo,
},
```

`dedicated_fans` est une **valeur absolue**, pas un delta — c'est ce qui rend la
restauration exacte après écrêtage.

### Méthode domaine

```rust
pub fn revert_post_match_sequence(
    &self,
    match_report_id: MatchReportId,
) -> Result<TeamDomainEvent, DomainError>
```

Gardes :
1. `expect_phase(GamePhase::PlayerImprovement)` — règle 1
2. `last_post_match` renseigné **et** portant ce `match_report_id` — règle 11

### `apply(PostMatchSequenceReverted)`

```rust
self.dedicated_fans          = *dedicated_fans;                       // règle 14
self.treasury.0              = self.treasury.0.saturating_sub(treasury_refund.0);
self.game_phase              = Some(GamePhase::MatchReporting);
self.current_match_report_id = Some(*match_report_id);
self.last_post_match         = None;                                  // règle 11
```

Restaurer `current_match_report_id` n'est pas cosmétique :
`start_post_match_sequence` exige la phase `MatchReporting`, et la
re-publication en dépend.

Mettre `last_post_match` à `None` **est** le mécanisme d'idempotence : une
seconde compensation ne trouve plus de dernier post-match et refuse. Règle
domaine, testable unitairement.

---

## 4. BC `players`

### Ce qui n'a pas besoin d'instantané

`injuries: Vec<PlayerInjuryRecord>` porte déjà `context.match_report_id`. Sont
donc **dérivables par filtrage**, sans rien stocker :

| À défaire | Dérivation |
|---|---|
| entrées de `injuries` | filtrer sur `context.match_report_id` |
| `stat_adjustments` ajoutés | un par blessure `Sequel` de ce match |
| `career_persistent_injuries` | une par blessure `BlessureSerieuse` de ce match |

### Ce qui a besoin d'un instantané

Les compteurs d'action et les SPP sont des scalaires cumulés, non tagués :

```rust
struct LastMatchContribution {
    match_report_id:             MatchReportId,
    spp_earned:                  Spp,
    touchdowns:                  TouchdownCount,
    passes:                      PassCount,
    interceptions:               InterceptionCount,
    casualties:                  CasualtyCount,
    mvps:                        MvpCount,
    fouls:                       FoulCount,
    matches_played:              MatchesPlayedCount,
    participation_status_before: PlayerParticipationStatus,  // règle 15
    availability_restored:       bool,
}
```

Accumulé dans `apply()` sur les événements dont `context.match_report_id`
correspond ; réinitialisé dès qu'un nouveau `match_report_id` apparaît. Comme
seul le dernier match est corrigible (garde-fou « à chaud »), un seul
accumulateur suffit.

### Méthode domaine

```rust
pub fn revert_match_impact(&self, match_report_id: &MatchReportId) -> Option<PlayerDomainEvent>
```

Retourne `None` si `last_match` est absent ou porte un autre `match_report_id` —
c'est l'idempotence de la règle 11, et c'est aussi ce qui permet au listener
d'itérer sur tout l'effectif sans se soucier de qui a joué.

Aucun risque de SPP négatif : le garde-fou garantit qu'aucun SPP n'a été
dépensé.

---

## 5. Point ouvert — antériorité à trancher

En typant `participation_status_before` (règle 15), une interaction de l'existant
demande vérification.

`handle_team_match_concluded` charge les joueurs via `find_by_team_id`, puis
restaure en `Available` ceux qui sont `MissingNextGame`. Or les événements de
blessure du **même match** ont déjà été appendés à ce stade : ils sont émis avant
`TeamMatchConcluded` par le publisher, et traités dans la même tâche
séquentielle.

Un joueur blessé pendant le match N (`Amoche`, `BlessureSerieuse`, `Sequel` →
`MissingNextGame`) serait donc remis en `Available` à la fin du traitement du
match N — l'effet « absent au prochain match » serait annulé aussitôt qu'appliqué.

Le commentaire de `team_match_concluded_listener` dit « pour ceux qui
l'étaient », ce qui suggère l'intention inverse : restaurer ceux qui étaient
`MissingNextGame` **avant** ce match, pas ceux que ce match vient de blesser.

**Conséquence pour la correction** : tant que ce point n'est pas tranché,
`participation_status_before` n'a pas de définition stable — « avant » ne
désigne pas le même instant selon la lecture retenue.

**Proposition** : une carte d'investigation dédiée, en amont, avec un test E2E
reproduisant le scénario « joueur blessé au match N → est-il absent au match
N+1 ? ». Le résultat fixe la définition de l'instantané. C'est un problème
préexistant, indépendant de cette feature, mais bloquant pour la règle 15.

---

## 6. Tests unitaires prévus

### `match_report`

| Test | Règle |
|---|---|
| `unpublish_refuse_si_spp_deja_depenses` | 2 |
| `unpublish_refuse_si_phase_avancee` | 1 |
| `unpublish_refuse_si_eligibilite_inconnue` | 12 |
| `unpublish_produit_ready_to_publish_avec_le_drapeau` | — |
| `rehydrate_traite_trois_cycles_publier_depublier` | 13 |
| `le_drapeau_survit_a_l_edition_apres_depublication` | phase 4 |
| `un_rapport_jamais_publie_ne_porte_pas_le_drapeau` | — |
| `depublier_un_rapport_non_publie_est_une_sequence_invalide` | — |
| `verdict_from_retient_home_avant_away` | 3a |
| `verdict_from_retient_spp_avant_phase` | 3b |

### `teams`

| Test | Règle |
|---|---|
| `revert_restaure_les_fans_ecretes_a_vingt` | 14 |
| `revert_restaure_les_fans_apres_plancher_a_zero` | 14 |
| `revert_soustrait_le_gain_de_tresorerie` | — |
| `revert_repasse_en_match_reporting` | — |
| `revert_refuse_si_phase_deja_avancee` | 1 |
| `revert_refuse_un_autre_match_report_id` | 11 |
| `un_second_revert_ne_produit_rien` | 11 |
| `publier_depublier_republier_converge_vers_le_meme_etat` | 8 |

Les deux premiers sont les tests **décisifs** de la feature : ce sont eux qui
démontrent qu'une approche par soustraction serait fausse.

### `players`

| Test | Règle |
|---|---|
| `revert_retire_les_spp_du_match` | — |
| `revert_retire_les_compteurs_de_carriere_du_match` | — |
| `revert_retire_les_blessures_du_match_seulement` | — |
| `revert_retire_le_malus_de_sequelle` | — |
| `revert_restaure_le_statut_de_participation` | 15 |
| `revert_ignore_un_autre_match_report_id` | 11 |
| `un_second_revert_ne_produit_rien` | 11 |
| `revert_ne_touche_pas_les_joueurs_temporaires` | BR1 existante |

### `ranking`

| Test | Règle |
|---|---|
| `revert_supprime_les_deux_lignes_du_match` | — |
| `un_second_revert_supprime_zero_ligne` | 11 |
| `revert_ne_touche_pas_les_lignes_d_un_autre_match` | — |
