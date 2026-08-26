# L'écran du jet · Phase 3 : architecture back

**Entrée** : `02-front.md` validé.

## Aucun widget, donc aucun fichier `_widget.rs`

La page est servie d'un bloc par un contrôleur, et le jet répond par un fragment.
Deux fichiers de vue, pas dix.

## Plan de fichiers

### `teams` — le domaine

| Fichier | Change |
|---|---|
| `domain/team.rs` | `GamePhase::CostlyMistakes` ; `TeamDomainEvent::CostlyMistakesPhaseStarted` ; `validate_dismissals_phase()` choisit sa sortie ; `apply_costly_mistakes()` |
| `domain/costly_mistakes.rs` | **nouveau** — la table, les effets, les dés nécessaires. Pur, testable sans rien |

`CostlyMistakesApplied { roll, incident, gp_lost }` **existe déjà**, ainsi que
`IncidentType`. Son `apply()` repose `ReadyToPlay` et son mouvement de trésorerie
est câblé. Rien à y toucher — sauf à décider si l'événement doit porter aussi les
dés secondaires (voir plus bas).

### `teams` — l'application

| Fichier | Change |
|---|---|
| `use_cases/validate_dismissals_phase_use_case.rs` | rend une **issue** : prête à jouer, ou erreurs coûteuses |
| `use_cases/apply_costly_mistakes_use_case.rs` | **nouveau** — tire, appelle le domaine, persiste |
| `ports.rs` | `IDiceRoller` ; `ITeamAccessPort` (mutualisé avec la carte 389) |

### `teams` — la couche web

| Fichier | Change |
|---|---|
| `io/web/costly_mistakes.rs` | **nouveau** — la page et le POST du jet |
| `io/web/templates/teams-costly-mistakes.html` | **nouveau** — la page |
| `io/web/templates/teams-costly-mistakes-result.html` | **nouveau** — le fragment de résultat |
| `io/web/team_detail.rs` | une branche de bandeau pour la nouvelle phase |
| `io/web/validate_phase_actions.rs` | redirige selon l'issue du use case |
| `routes.rs` | `COSTLY_MISTAKES_PAGE`, `COSTLY_MISTAKES_ROLL` |
| `assets/static/css/pages/costly-mistakes.css` | **nouveau**, inscrit au bundle |

### `infrastructure`

| Fichier | Change |
|---|---|
| `infrastructure/teams/dice_adapter.rs` | **nouveau** — `IDiceRoller` sur `rand` |
| `infrastructure/teams/access_adapter.rs` | celui de la carte 389, ou créé ici |

## Le hasard passe par un port

Le précédent du projet tire en dur dans le use case
(`random_draw.rs:44`, `StdRng::from_os_rng()`), et ses tests ne portent que sur
la **répartition**, jamais sur le tirage. Ça suffisait là ; pas ici.

Cette fonctionnalité doit prouver qu'à 345 kPo, **un 1 donne un incident majeur
et retire exactement 170**. Sans jet forçable, ce test n'existe pas.

```rust
pub trait IDiceRoller: Send + Sync {
    fn d6(&self) -> u8;
    fn d3(&self) -> u8;
    fn two_d6(&self) -> (u8, u8);
}
```

`two_d6` et non deux appels à `d6` : les deux dés d'une catastrophe sont **un
seul geste**, et l'événement doit pouvoir les afficher tels quels. Un test qui
enchaîne deux `d6` truqués deviendrait vite illisible.

## Le domaine décrit ce dont il a besoin

Le use case ne doit pas décider quel dé tirer — c'est la table qui le dit.

```rust
// domain/costly_mistakes.rs
pub fn incident_for(treasury: Kpo, roll: u8) -> IncidentType;

pub enum DiceNeeded { None, OneD3, TwoD6 }
impl IncidentType { pub fn dice_needed(&self) -> DiceNeeded; }

pub fn loss_for(incident: IncidentType, treasury: Kpo, dice: &[u8]) -> Kpo;
```

L'orchestration devient une suite d'échanges où le use case ne tranche rien :

```
1. d6                        → roll
2. incident_for(treasury, roll)
3. incident.dice_needed()    → rien / 1D3 / 2D6
4. tirer ce qui est demandé
5. loss_for(incident, treasury, dice)
6. team.apply_costly_mistakes(roll, incident, gp_lost)
7. repo.append(…)
```

**On ne tire que ce qu'on utilise.** L'alternative — tirer tous les dés d'avance
et laisser le domaine choisir — mettrait dans l'événement des jets qui n'ont
jamais eu lieu.

## Deux sorties pour la validation des renvois

```rust
pub enum ValidateDismissalsOutcome {
    ReadyToPlay,
    CostlyMistakes,
}
```

C'est **le domaine** qui tranche, dans `validate_dismissals_phase()`, en lisant
la trésorerie de l'agrégat : il émet `DismissalsPhaseValidated` ou
`CostlyMistakesPhaseStarted`. `apply()` reste bête — il applique un fait, il n'en
décide pas.

Le précédent de forme est `RecordInducementsOutcome`, dans `match_report`, dont
le use case rend déjà une issue que le contrôleur traduit en redirection.

**Aucune migration** : les équipes dont l'historique ne porte que
`DismissalsPhaseValidated` se rejouent à l'identique.

## Ports et droits

`ITeamAccessPort` — propriétaire, admin d'espace, admin de compétition — est
**celui de la carte 389**. Les deux fonctionnalités le partagent ; la première
livrée le crée.

Il garde ici **le POST du jet**, pas seulement l'affichage : la page a beau ne
s'ouvrir qu'en phase `CostlyMistakes`, l'URL du jet reste devinable, et un jet a
un effet financier.

## Aucun domain service

Rien ne traverse depuis un autre BC : la trésorerie est dans l'agrégat, la table
est du domaine pur, le dé vient d'un port technique. Il n'y a aucun DTO de port à
transformer, donc pas de service au sens du `CLAUDE.md`.

## L'événement porte tout — décision

`CostlyMistakesApplied` ne portait qu'un `roll`. Il portera **tous les jets** :

```rust
CostlyMistakesApplied {
    roll: u8,                 // le D6 de la table
    #[serde(default)]
    damage_dice: Vec<u8>,     // 1D3 pour un mineur, 2D6 pour une catastrophe, vide sinon
    incident: IncidentType,
    gp_lost: Kpo,
}
```

**Le champ se ferme aujourd'hui ou jamais.** Ajouté maintenant, il coûte une
ligne ; plus tard, les jets déjà écrits n'auront jamais eu de dés secondaires, et
aucune migration ne les inventera. `#[serde(default)]` pour que les événements
existants — il y en a peut-être en production — se relisent en liste vide.

Un `Vec<u8>` et non deux champs optionnels : l'incident dit déjà combien de dés
attendre, et deux `Option` inviteraient à les remplir tous les deux.

## L'historique de trésorerie est acquis

Il n'y a **rien à écrire pour lui**. Le grand livre existe
(`teams__treasury_ledger`) et porte, pour chaque mouvement :

| Colonne | |
|---|---|
| `direction` | `Credit` / `Debit` |
| `amount_kpo` | le montant **effectif**, écrêté au solde |
| `reason` | dont **`CostlyMistake`, déjà défini** |
| `balance_after_kpo` | le solde après le mouvement |
| `event_version` | la version de l'événement source, unique par équipe |
| `occurred_at` | l'horodatage |

La ligne est écrite **dans la même transaction que l'événement** — c'est la règle
des projections du `CLAUDE.md`, et `append` l'applique déjà. Une erreur coûteuse
produira donc sa ligne sans qu'on ajoute une seule instruction.

Deux niveaux de lecture en découlent, et il faut les distinguer :

- **le grand livre** répond à « où est passé l'argent ? » — montant, motif, solde
  après. C'est ce qu'affichera l'onglet Trésorerie (carte 48) ;
- **l'event store** répond à « pourquoi ce montant ? » — le jet, les dés,
  l'incident. C'est le détail, et il n'a pas sa place dans une ligne de compte.

Le livre garde le montant **effectif** : une bourde de 50 kPo sur une caisse de
30 y figure pour 30, comme le documente `treasury.rs`. C'est voulu — un compte
enregistre ce qui s'est passé, pas ce qui avait été décidé.

## Règles métier à préciser en phase 4


- **Que fait-on d'un `gp_lost` supérieur au solde ?** `TreasuryMovement::debit`
  écrête déjà et le teste. La question est de savoir si le domaine doit refuser
  de produire un tel montant, ou laisser l'écrêtage faire — il ne peut de toute
  façon pas survenir, chaque effet étant borné par la trésorerie.
- **Le jet doit-il être journalisé ?** Un `info!` avec le jet, l'incident et le
  montant rendrait toute contestation vérifiable. Le use case étant instrumenté
  par `#[tracing::instrument]`, la commande y figure déjà — mais pas le résultat.
