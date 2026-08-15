# Phase 4 — Contrats de données : le service d'envoi

**Entrée** : `03-back.md`, validée.

## Correction de la phase 3 — deux déclencheurs, pas un

La phase 3 décrivait une orchestration unique : « pour chaque saison candidate,
requête bornée sur les dates du jour ». **Cela ne marche pas pour l'ouverture des
inscriptions**, qui se déclenche sur un fait — la saison s'ouvre — et non sur une
date à comparer à aujourd'hui.

R11 tranche : elle part **à la validation de l'étape 5**, dans une tâche
détachée. Les trois autres restent pilotées par le cron.

Ce que les deux chemins partagent — et c'est ce qui rend la scission acceptable :
le même use case d'expédition, le même journal, le même service de destinataires,
les mêmes gabarits. **Seul le déclencheur diffère.**

## Domaine — ce qui est dû aujourd'hui

```rust
/// Une notification à envoyer, et le contexte minimal qui la qualifie.
/// `RegistrationOpen` n'y figure pas : elle ne vient jamais de `due_today()`.
pub enum DueNotification {
    RoundEve             { round: RoundRef },
    RoundClosing         { round: RoundRef },
    RegistrationDeadline { deadline: DateString },
}

pub struct RoundRef {
    pub round_id:   MatchId,
    pub round_name: MatchDayName,
    pub date_start: DateString,
    pub date_end:   Option<DateString>,
    pub day_type:   MatchDayType,
}
```

**Émis par** : `notification_schedule::due_today()`. **Consommé par** : le use
case d'expédition, et le service de destinataires.

Aucune primitive nue : c'est du domaine, la règle du CLAUDE.md s'applique
pleinement.

`day_type` voyage parce que le gabarit en dépend — une journée à date fixe n'a
pas de ligne « clôture ». `MatchDayType::Rest` ne peut pas apparaître ici : les
requêtes l'excluent (phase 3), et `due_today()` l'ignore.

## Domaine — la clé d'idempotence

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    RegistrationOpen,
    RoundEve,
    RoundClosing,
    RegistrationDeadline,
}

impl NotificationType {
    pub fn as_str(&self) -> &'static str { … }   // la valeur stockée
}

/// Ce qui identifie un envoi, et rien de plus. C'est **exactement** la clé de
/// l'index unique (R3) : si les deux divergeaient, la protection ne porterait
/// plus sur ce que le code croit protéger.
pub struct DeliveryKey {
    pub notification_type: NotificationType,
    pub season_id:         SeasonId,
    pub round_id:          Option<MatchId>,
    pub target_date:       DateString,
    pub coach_id:          CoachId,
}
```

**Émis par** : le use case, pour chaque destinataire. **Consommé par** : le
repository du journal.

`target_date` est **la date visée**, pas la date d'envoi. C'est ce qui fait tenir
R2 : une journée décalée change la clé, donc réarme la notification, sans qu'une
seule ligne de code lui soit consacrée.

## Port — les membres de l'espace

```rust
pub struct SpaceMemberDto {
    pub coach_id:   String,
    pub coach_name: String,
    pub email:      String,
}
```

**Émis par** : `ICompetitionSpaceMemberPort::list_space_members`, implémenté par
l'adapter qui interroge `spaces`. **Consommé par** : `notification_recipients`,
jamais par la CLI ni par un gabarit.

Primitives nues assumées : c'est un DTO de port en lecture, l'exception que le
CLAUDE.md prévoit explicitement.

## Domaine — le destinataire résolu

```rust
pub struct Recipient {
    pub coach_id:   CoachId,
    pub coach_name: CoachName,
    pub email:      Email,
    /// Ce que ce coach joue cette journée. Cf. la correction ci-dessous.
    pub participation: RoundParticipation,
}

pub struct Fixture {
    pub team_name:  TeamName,
    pub home_team:  TeamName,
    pub away_team:  TeamName,
    pub match_url:  String,
}
```

**Émis par** : `notification_recipients::resolve()`. **Consommé par** : le use
case, qui en tire le contexte de rendu.

> **Corrigé en phase 5** : ce champ était un `Option<Fixture>`. Or rien
> n'empêche un coach d'inscrire **deux équipes** dans la même saison, et la clé
> d'idempotence ne portant pas d'équipe, il reçoit un seul email — le second
> match aurait donc disparu en silence. Le type devient un enum
> `RoundParticipation { NotPlaying, Playing(Vec<Fixture>) }`.

Le type — `Option` hier, enum aujourd'hui — est le point où R4 cesse d'être une
règle écrite pour devenir une forme : le gabarit ne peut pas oublier le cas
« inscrit sans match », le compilateur l'oblige à traiter les deux branches.

## Sortie — les contextes de rendu des quatre gabarits

Un contexte par gabarit, tous en primitives : ce sont des view models.

```rust
#[derive(Template)]
#[template(path = "emails/fr_FR/competition_round_eve.html")]
pub struct RoundEveEmail {
    pub app_url:          String,
    pub coach_name:       String,
    pub competition_name: String,
    pub competition_url:  String,
    pub round_name:       String,
    pub date_start:       String,
    /// `None` pour une journée à date fixe — la ligne « clôture » disparaît.
    pub date_end:         Option<String>,
    /// Corrigé en phase 5 : un enum et non un `Option`, pour que le gabarit
    /// doive traiter les deux branches jusqu'au dernier mètre. Un `Vec` vide
    /// se rendrait silencieusement, et la ligne « tu ne joues pas » de R4
    /// disparaîtrait sans que rien ne proteste.
    pub participation:    ParticipationVm,
}

pub enum ParticipationVm {
    NotPlaying,
    Playing(Vec<FixtureVm>),
}

pub struct FixtureVm {
    pub team_name: String,
    pub home_team: String,
    pub away_team: String,
    pub match_url: String,
}
```

Les trois autres suivent la même forme, avec les variables relevées dans les
maquettes :

| Gabarit | Variables |
|---|---|
| `competition_registration_open` | `app_url`, `coach_name`, `admin_name`, `space_name`, `competition_name`, `season_name`, `competition_url`, `registration_deadline` |
| `competition_round_eve` | ci-dessus |
| `competition_round_closing` | `app_url`, `coach_name`, `competition_name`, `round_name`, `date_end`, `participation` |
| `competition_registration_deadline` | `app_url`, `coach_name`, `admin_name`, `space_name`, `competition_name`, `season_name`, `competition_url`, `registration_deadline`, `remaining_slots` |

### Deux axes de variation dans la veille de journée, pas un

La maquette porte **deux** conditions indépendantes, qu'il serait facile de
confondre :

| Axe | Ce qui change | Piloté par |
|---|---|---|
| type de journée | la ligne « clôture » apparaît ou non | `date_end: Option<String>` |
| coach avec ou sans match | le bloc des matchs, ou « tu ne joues pas » | `participation: ParticipationVm` |

Quatre combinaisons, donc, et les quatre sont atteignables : une journée à date
fixe pour un coach qui ne joue pas est un cas parfaitement ordinaire.

### `app_url` porte son schéma, et ne recopie pas le défaut existant

`send_reset_password_email` construit `format!("http://{}{}", host_domain, …)` —
schéma **en dur**. Des liens en `http://` dans quatre emails partant en
production seraient un défaut visible, et le recopier par symétrie serait le
propager.

`app_url` est donc construit depuis la configuration, schéma compris. Corriger
`lost_login` relève de la **carte 325**, pas de celle-ci — mais il ne faut pas
que la nouvelle fonctionnalité hérite du problème en attendant.

`host_domain` venant de la configuration et non de l'en-tête `Host`, la CLI le
lit sans requête entrante. C'était le risque de ce chemin ; il n'existe pas.

## Interfaces — qui émet, qui consomme

| DTO | Émetteur | Consommateur |
|---|---|---|
| `DueNotification`, `RoundRef` | `due_today()` | use case, service de destinataires |
| `DeliveryKey` | use case | repository du journal |
| `SpaceMemberDto` | port `spaces` | `notification_recipients` **seul** |
| `TeamInfoDto` | port `teams` | `notification_recipients` **seul** |
| `Recipient`, `Fixture` | `notification_recipients` | use case |
| les quatre contextes | use case | gabarits Askama |

**Aucun DTO de port n'atteint un gabarit ni la CLI.** C'est la règle des domain
services du CLAUDE.md, et elle a ici une raison concrète : les destinataires se
calculent par différence entre deux ports (R7), et cette soustraction n'a aucune
raison d'être visible d'un gabarit.

## Règles métier

### R11 — apparue à cette phase, tranchée

L'ouverture des inscriptions part **à la validation de l'étape 5**, en tâche
détachée ; les trois autres sont pilotées par le cron. Consignée dans le README.

## Ce que cette phase laisse aux suivantes

- **Phase 5** — le use case d'expédition, le domain service des destinataires,
  et le point d'accroche dans la validation de l'étape 5.
- **Phase 6** — `due_today()`, et le récapitulatif des onze règles.
- **Phase 7** — la migration et son index sur `COALESCE(round_id, '')`, les
  quatre gabarits, la sous-commande et ses arguments, les tests.
