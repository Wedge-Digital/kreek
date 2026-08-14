# Phase 3 — Architecture back : le service d'envoi

**Entrée** : les maquettes de phase 1, et les règles R1 à R10.

**Pas de phase 2** : un email n'a pas d'architecture front — ni composition
HTMX, ni événements DOM, ni swap. Cf. le README.

## Ce que l'investigation a trouvé

### Un troisième réglage mort, et il touche l'ordonnanceur

`ScheduleConfig` porte `schedule_timezone`, saisi à l'étape 3 du magicien avec
son sélecteur et `Europe/Paris` par défaut, stocké — et lu par personne. La phase
1 avait par ailleurs décidé que le cron tournerait dans le fuseau du serveur.

Les deux ne pouvaient pas être vrais ensemble. **R10 tranche : le fuseau du
serveur, et le sélecteur disparaît.** Trois réglages sont donc retirés de
l'étape 3 et de l'étape 4, pas deux.

Conséquence à ne pas laisser derrière nous : le VO `Timezone`
(`shared_kernel/bloodbowl/timezone.rs`) n'a **aucun autre utilisateur**. Il
devient orphelin le jour où le champ disparaît.

### Les journées ont un troisième type : `Rest`

`MatchDayType` vaut `FixedDate`, `TimeFrame` ou **`Rest`**. Une journée de repos
ne déclenche évidemment aucune notification, et les requêtes doivent l'exclure
explicitement.

Cela ne concernait pas `configuration/` — `applicability()` lit la *structure*,
dont le type `ScheduledDate` n'a que deux variantes — mais `envoi/` s'y ferait
prendre : la table persistée, elle, en a trois.

### Les dates sont du texte, et c'est sans danger

`DateString` est validée `^(?:\d{4}-\d{2}-\d{2})?$` : la comparaison
lexicographique en SQL sur `TEXT` est donc exacte, et une égalité sur la date du
jour suffit. **Seule précaution** : la chaîne vide passe cette regex, il faut
l'exclure au même titre que `NULL`.

## Où vit l'envoi

Dans **`competitions`**. Il possède les saisons, les réglages, les journées, les
appariements et la date limite d'inscription — c'est-à-dire tout ce qui *décide*
qu'une notification est due. Ce qui lui manque, il l'obtient par port.

| Besoin | Propriétaire | Accès |
|---|---|---|
| saisons, réglages, journées, appariements, date limite | `competitions` | ses repositories |
| équipes inscrites et leurs coachs | `teams` | `ITeamInfoPort` — **existe** |
| membres de l'espace et leurs adresses | `spaces` | `ICompetitionSpaceMemberPort` — **existe**, à étendre |
| l'expédition elle-même | service partagé | `IEmailService` — **existe** |

Trois des quatre sont déjà en place. Le seul ajout est une méthode de listing sur
le port `spaces`, décrite plus bas.

## R9 est tenue par la signature, pas par la discipline

```rust
// domain/notification_schedule.rs — pur, aucun accès au monde extérieur
pub fn due_today(
    today: &DateString,
    match_days: &[MatchDay],
    invitations: Option<&CompetitionInvitations>,
    settings: &CompetitionNotifications,
) -> Vec<DueNotification>;
```

La fonction reçoit `today` et les dates de la saison, et rend ce qui est dû
**aujourd'hui**. Elle n'a **aucun accès au journal d'envois** : elle ne peut donc
pas poser la question « qu'est-ce qui manque ? », même si quelqu'un le voulait.

C'est le point d'architecture le plus important de cette phase. R9 aurait pu
être une consigne écrite, que le premier développeur pressé aurait contournée en
ajoutant un paramètre. En privant la fonction de la donnée, c'est le compilateur
qui garde la règle.

Le journal n'intervient qu'**après**, pour filtrer les doublons — jamais pour
produire du travail.

## Le journal d'envois : `sent_at` est nullable, et c'est structurant

```sql
competition_notification_deliveries
    notification_type   TEXT        NOT NULL
    season_id           TEXT        NOT NULL
    round_id            TEXT                  -- NULL pour les notifications de saison
    target_date         TEXT        NOT NULL  -- la date visée, cf. R2
    coach_id            TEXT        NOT NULL
    claimed_at          TIMESTAMPTZ NOT NULL DEFAULT now()
    sent_at             TIMESTAMPTZ           -- NULL = réservé, non confirmé
    UNIQUE (notification_type, season_id, round_id, target_date, coach_id)
```

**La ligne est insérée avant l'envoi**, pas après. C'est elle qui réserve le
créneau : deux instances de cron lancées en parallèle se disputent la contrainte
d'unicité, et une seule gagne. Envoyer d'abord et journaliser ensuite laisserait
la fenêtre grande ouverte.

`sent_at` n'est renseigné qu'après confirmation de `IEmailService`. **Une ligne à
`sent_at NULL` est donc un échec constaté**, et c'est ce qui donne à R1 sa
journalisation : la notification perdue n'est pas silencieuse, elle a une ligne.

**Discipline qui en découle, et qui est contre-intuitive** : ces lignes ne sont
**jamais rejouées le lendemain**. Les reprendre serait précisément le « cherche
ce qui n'est pas parti et envoie-le » que R9 interdit. On réessaie **dans la même
exécution** — un échec réseau transitoire mérite une seconde tentative — jamais
d'un jour sur l'autre.

`round_id` est nullable parce que deux des quatre notifications ne concernent
pas une journée. La contrainte d'unicité sur une colonne nullable ne joue pas en
PostgreSQL : il faudra un index unique avec `COALESCE(round_id, '')`, sans quoi
la protection tombe exactement là où on la croit acquise.

## Ports — un seul ajout

```rust
// ports.rs — ICompetitionSpaceMemberPort gagne :

/// Les membres de l'espace, avec leur adresse. Sert deux choses :
/// borner le périmètre en mode d'accès libre (R7), et résoudre les adresses
/// des coachs inscrits — un inscrit étant nécessairement membre de l'espace.
async fn list_space_members(&self, space_id: &SpaceId) -> Vec<SpaceMemberDto>;

pub struct SpaceMemberDto {
    pub coach_id: String,
    pub coach_name: String,
    pub email: String,
}
```

Une seule méthode, et non deux. Les coachs inscrits sont **nécessairement**
membres de l'espace : lister les membres une fois puis indexer par `coach_id`
répond aux deux besoins en un aller-retour.

L'adapter existant (`infrastructure/competitions/space_member_adapter.rs`)
s'appuie sur `list_members_for_space.sql`, déjà écrite dans `spaces` et rendant
exactement `id, coach_name, coach_icon, email`.

**Les trois tables `competitions__*_cache` ne sont pas utilisées** — elles
contiennent pourtant emails et appartenance, et sont vides. Le piège est
consigné dans le README ; il se referme ici en passant par le port.

## Plan de fichiers

```
migrations/
└── <ts>_competition_notification_deliveries.sql

src/app/competitions/
├── domain/
│   ├── notification_schedule.rs      ← due_today(), DueNotification — pur
│   └── notification_delivery.rs      ← la clé d'idempotence et ses VOs
├── ports.rs                          ← += list_space_members, SpaceMemberDto
├── use_cases/
│   ├── send_due_notifications_use_case.rs   ← l'orchestration
│   └── notification_recipients.rs           ← domain service : R7, les 4 cas
└── io/
    └── repository/
        ├── notification_delivery_repository.rs
        └── sql/notifications/
            ├── list_seasons_with_round_starting.sql
            ├── list_seasons_with_round_closing.sql
            ├── list_seasons_with_deadline.sql
            ├── claim_delivery.sql        ← INSERT … ON CONFLICT DO NOTHING
            └── confirm_delivery.sql      ← UPDATE … SET sent_at = now()

src/cli/
└── send_notifications.rs             ← la sous-commande

assets/templates/emails/fr_FR/
├── competition_registration_open.html
├── competition_round_eve.html
├── competition_round_closing.html
└── competition_registration_deadline.html

src/infrastructure/competitions/
└── space_member_adapter.rs           ← += list_space_members
```

Les gabarits vont dans `assets/templates/emails/fr_FR/`, où vit déjà
`lost_login.html` — la maison, et le dossier `en_EN` a été supprimé (français
seul).

## Domain service pour R7

`notification_recipients.rs` traduit les DTOs des ports en destinataires. Les
handlers — ici la CLI — ne voient jamais un `TeamInfoDto` ni un `SpaceMemberDto`.

| Notification | Destinataires | Calcul |
|---|---|---|
| Ouverture des inscriptions | mode `invitation` : les invités ; mode `open` : les membres de l'espace | lecture directe |
| Veille de journée | les coachs inscrits | `find_enrolled_teams` |
| Fin de journée imminente | les coachs inscrits | idem |
| Date limite | invités non inscrits, ou membres non inscrits | **par différence** |

La dernière ligne est la raison pour laquelle ce service existe : deux des quatre
notifications s'adressent à des gens qui ne sont **pas** inscrits, et cet ensemble
se calcule par différence entre deux ports. Ce n'est pas « lire une liste ».

## L'orchestration

```
send_due_notifications(today) :
  pour chaque saison candidate                      ← SQL borné sur les dates du jour
    charger réglages, journées, invitations
    dues ← notification_schedule::due_today(…)      ← domaine pur, R9
    pour chaque notification due
      destinataires ← notification_recipients::resolve(…)   ← R7
      pour chaque destinataire                              ← jamais groupé
        claim ← réserver la ligne du journal        ← R3 ; si conflit, passer
        html  ← rendre le gabarit, corps personnalisé        ← R4
        envoyer via IEmailService
        confirmer (sent_at) ou laisser NULL          ← R1
```

**Un envoi par destinataire, jamais groupé.** `IEmailService::send` prend un
`Vec<String>` de destinataires, ce qui invite à expédier trente coachs en un
appel — et mettrait les trente adresses en clair dans l'en-tête que chacun
reçoit. R4 impose de toute façon un corps personnalisé.

**La requête de saisons candidates est bornée par la date du jour**, pas
parcourue en entier : c'est ce qui rend le coût du cron indépendant du nombre de
saisons historiques.

## La sous-commande CLI

`main.rs` porte déjà `Serve`, `SeedAccounts` et `SeedE2e`. Une quatrième
s'ajoute. Ce qu'elle expose est spécifié en phase 7 ; ce qui est acquis ici :

- `main()` charge configuration, pool **et migrations** avant de dispatcher — la
  commande ne peut pas tourner sur un schéma périmé ;
- `compose(cfg, pool) -> AppState` donne tout le câblage sans dupliquer
  `main.rs` ;
- la sortie en `exit(1)` sur échec rend R1 observable dans les logs du cron.

## Règles métier

### R10 — apparue à cette phase, tranchée

Le fuseau retenu est **celui du serveur**, et le sélecteur de l'étape 3
disparaît. Consignée dans le README.

## Ce que cette phase laisse aux suivantes

- **Phase 4** — les DTOs : `DueNotification`, `SpaceMemberDto`, les contextes de
  rendu des quatre gabarits.
- **Phase 5** — le use case d'envoi et le domain service des destinataires.
- **Phase 6** — `due_today()`, et le récapitulatif des dix règles.
- **Phase 7** — la migration et son index unique sur `COALESCE(round_id, '')`,
  les quatre gabarits, la sous-commande et ses arguments, les tests.
