# Phase 6 — Domaine : l'écran de réglage des notifications

**Entrée** : `05-use-cases.md`, validée.

## Récapitulatif exhaustif des règles métier

Les neuf règles de la fonctionnalité, et à quelle unité chacune appartient.

| | Règle | Unité |
|---|---|---|
| R1 | Une notification manquée est perdue, et journalisée | envoi |
| R2 | Un décalage de date réarme la notification, sans règle en plus | envoi |
| R3 | Idempotence **par destinataire**, garantie par la base | envoi |
| R4 | Tous les inscrits reçoivent l'email de journée, avec deux corps | envoi |
| R5 | Une notification inapplicable est grisée, avec son motif | **configuration** |
| R6 | Cochée puis rendue inapplicable, elle reste cochée | **configuration** |
| R7 | Le périmètre des destinataires est borné par l'espace | envoi |
| R8 | Les saisons existantes démarrent éteintes, les neuves allumées | **configuration** |
| R9 | L'activation n'est jamais rétroactive | envoi |

Trois contraignent cet écran. Le domaine s'y réduit à **une fonction**, et c'est
le seul endroit de `configuration/` où une décision se prend.

## `applicability()` — la seule logique métier de l'écran

```rust
/// Pourquoi une notification ne peut pas se déclencher sur cette saison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inapplicable {
    /// Pas de calendrier — l'interrupteur est éteint, **ou** aucune journée
    /// n'a été saisie. Le motif affiché est le même dans les deux cas : un
    /// calendrier à zéro journée n'est pas un calendrier.
    NoSchedule,
    /// Aucune journée n'a de fenêtre à clore — que des dates fixes.
    NoTimeFrameRound,
    /// Aucune date limite d'inscription n'est fixée.
    NoDeadline,
}

/// `None` = applicable. `registration_open` n'y figure pas : elle l'est
/// toujours, une compétition ayant par construction une ouverture.
pub struct NotificationApplicability {
    pub round_eve: Option<Inapplicable>,
    pub round_closing: Option<Inapplicable>,
    pub registration_deadline: Option<Inapplicable>,
}

pub fn applicability(
    structure: &CompetitionStructure,
    invitations: Option<&CompetitionInvitations>,
) -> NotificationApplicability
```

### Ce qu'elle décide

| Notification | Inapplicable quand |
|---|---|
| Veille de journée | `use_schedule` éteint, ou `scheduled_dates` vide → `NoSchedule` |
| Fin de journée imminente | idem ; sinon, aucune `ScheduledDate::TimeFrame` → `NoTimeFrameRound` |
| Date limite | invitations absentes, ou `registration_deadline` absente **ou vide** → `NoDeadline` |

Une journée `FixedDate` ne porte qu'une `multiplexe_date` : elle n'a pas de
fenêtre à clore, d'où la distinction entre les deux notifications de journée.
Une compétition dont toutes les journées sont à date fixe peut prévenir de la
veille, jamais de la clôture.

### Deux cas limites que R5 ne tranchait pas

**Un calendrier activé mais sans aucune journée.** C'est l'état normal d'une
saison arrivée à l'étape 4 sans avoir rempli l'étape 3. Traité comme « pas de
calendrier » : le motif à afficher est le même, et distinguer les deux ne
donnerait à l'organisateur aucune action différente.

**Une date limite vide plutôt qu'absente.** Le champ date renvoie `""` quand on
l'efface. Le JS de l'étape 4 le convertit aujourd'hui en `null`, mais la fonction
de domaine traite les deux identiquement plutôt que de dépendre d'une conversion
faite ailleurs — une règle métier qui repose sur un `|| null` dans un template
est une règle qui tombera au premier remaniement de ce template.

### Ni erreur, ni état

`applicability()` est une **fonction pure et totale** : toute structure lui donne
une réponse, aucune entrée ne la met en échec. Elle ne retourne donc pas de
`Result` et n'ajoute rien à `DomainError`.

Elle ne lit pas non plus `CompetitionNotifications` : ce qui est *coché* et ce
qui est *applicable* sont deux choses indépendantes, et c'est très exactement ce
que R6 exige. Les mêler dans une seule fonction rendrait impossible d'afficher
une case cochée et grisée.

## Aucune méthode d'agrégat

`CompetitionNotifications` est une structure de réglages, pas un agrégat gardien
d'invariants : les quatre booléens sont indépendants, aucune combinaison n'est
interdite, et R6 interdit explicitement de filtrer à l'écriture. Lui donner des
méthodes de commande serait inventer des invariants qui n'existent pas.

C'est la même nature que `CompetitionInvitations` et `CompetitionStructure`, qui
n'en ont pas davantage.

## Tests unitaires — un par règle et par cas limite

Sur `applicability()` :

| Test | Attendu |
|---|---|
| calendrier éteint | `round_eve` et `round_closing` = `NoSchedule` |
| calendrier allumé, zéro journée | idem — le cas limite ci-dessus |
| journées à date fixe seulement | `round_eve` applicable, `round_closing` = `NoTimeFrameRound` |
| au moins une journée `TimeFrame` | les deux applicables |
| invitations absentes | `registration_deadline` = `NoDeadline` |
| date limite `Some("")` | `NoDeadline` — le second cas limite |
| date limite renseignée | applicable |

Sur la construction du VM, pour R6 :

| Test | Attendu |
|---|---|
| réglage coché, notification inapplicable | `checked = true` **et** `inapplicable_reason = Some(…)` |

Ce dernier est le test qui garde R6. Sans lui, une future « simplification »
décochant les lignes grisées passerait sans que rien ne proteste.

Sur la sérialisation, pour R8 :

| Test | Attendu |
|---|---|
| JSON vide `{}` | les quatre à `true` — le défaut « saison neuve » |
| JSON à quatre `false` | round-trip fidèle |

Le premier vaut avertissement autant que vérification : il n'a de sens que
parce que la migration remplit les lignes existantes (cf. phase 4). Si ce
remplissage disparaissait, ce test continuerait de passer pendant que ~399
saisons se mettraient à envoyer.

## Ce que cette phase laisse aux suivantes

- **Phase 7** — la migration et son remplissage, le retrait des deux
  interrupteurs morts, le branchement du widget dans ses deux hôtes.
- **Phase 8** — le découpage en cartes.
