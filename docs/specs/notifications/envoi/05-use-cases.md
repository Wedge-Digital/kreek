# Phase 5 — Use cases : le service d'envoi

**Entrée** : `04-dtos.md`, validée.

## Correction de la phase 4 — un coach peut avoir deux équipes

`team_enrollment_projection` n'a de contrainte que sur `team_id`, et aucun
garde-fou domaine n'interdit à un coach d'inscrire deux équipes dans la même
saison. Or la clé d'idempotence ne porte **pas** d'équipe : un coach reçoit un
email par journée, pas un par équipe.

Le `Option<Fixture>` de la phase 4 aurait donc perdu le second match **en
silence** — le défaut même que cette spec reproche au reste du code.

```rust
pub enum RoundParticipation {
    NotPlaying,
    Playing(Vec<Fixture>),   // non vide par construction (smart constructor)
}
```

L'enum garde ce que l'`Option` apportait — le gabarit doit traiter les deux
branches, donc R4 reste tenue par le type et non par une consigne — et ne perd
plus rien.

Ajouter `team_id` à la clé d'idempotence était l'autre voie : elle aurait donné
deux emails au même coach. Écartée — deux emails pour une même journée
ressemblent à un bug, une liste de deux matchs non.

## Domain service — `notification_recipients.rs`

```rust
pub async fn resolve(
    notification: NotificationType,
    season:       &SeasonContext,
    round:        Option<&RoundRef>,
    teams:        &dyn ITeamInfoPort,
    members:      &dyn ICompetitionSpaceMemberPort,
) -> Result<Vec<Recipient>, RecipientsError>;
```

Vit dans `use_cases/` : il consomme des ports, donc il n'est pas du domaine pur
(cf. CLAUDE.md, « Domain services pour données inter-BCs »).

### R7 est tenue par le chemin de données, pas par une vérification

Les quatre cas commencent par **la même opération** : lister les membres de
l'espace, et n'en sortir personne.

| Notification | Ensemble | Calcul |
|---|---|---|
| Ouverture — mode `invitation` | les invités | invités **∩** membres |
| Ouverture — mode `open` | tous les membres | membres |
| Veille de journée | les inscrits | inscrits **∩** membres |
| Fin de journée | les inscrits | inscrits **∩** membres |
| Date limite | ceux qui peuvent encore s'inscrire | (invités ou membres) **∩** membres **−** inscrits |

**L'intersection n'est pas un contrôle ajouté : c'est le seul chemin vers une
adresse email.** Ni `invited_coaches` ni `find_enrolled_teams` ne portent
d'adresse ; seul `list_space_members` en a. Un invité qui a quitté l'espace, ou
une équipe dont le coach n'y est plus, tombe donc naturellement — sans qu'aucune
ligne ne vérifie quoi que ce soit.

C'est la troisième fois dans cette spec qu'une règle est tenue par une structure
plutôt que par de la vigilance : R9 par la signature de `due_today()`, R2 par la
`target_date` de la clé, R7 ici par le seul chemin menant à l'email. Ces
trois-là ne peuvent pas être oubliées lors d'une modification future.

### La date limite se calcule par différence

C'est le seul cas où l'ensemble n'existe dans aucun port : ni « les invités qui
ne se sont pas inscrits », ni « les membres qui n'ont pas bougé » ne se
demandent. On soustrait les `coach_id` des équipes inscrites de l'ensemble des
candidats.

C'est ce qui justifie l'existence de ce service : sans lui, cette soustraction
finirait dans la CLI, où elle n'a rien à faire.

### Les fixtures

Pour les deux notifications de journée, chaque destinataire reçoit sa
`RoundParticipation` : les appariements de la journée, croisés avec les équipes
du coach. Un coach dont aucune équipe n'apparaît dans un appariement est
`NotPlaying` — ce qui est une information, pas une absence de donnée (R4).

## Le cœur partagé — `notification_dispatch.rs`

Les deux déclencheurs de R11 partagent tout sauf leur déclenchement. Ce qu'ils
partagent vit ici :

```rust
pub async fn dispatch(
    notification: NotificationType,
    season:       &SeasonContext,
    round:        Option<&RoundRef>,
    target_date:  &DateString,
    deps:         &DispatchDeps<'_>,
) -> DispatchOutcome;

pub struct DispatchOutcome {
    pub sent:                  usize,
    pub skipped_already_sent:  usize,
    pub failed:                usize,
}
```

Pour chaque destinataire, dans cet ordre :

1. **Réserver** la ligne du journal (`INSERT … ON CONFLICT DO NOTHING`). Zéro
   ligne insérée → déjà envoyé, on passe. C'est R3, et la base tranche, pas le
   code.
2. **Rendre** le gabarit avec le contexte du destinataire.
3. **Envoyer**, un destinataire à la fois.
4. **Confirmer** (`sent_at = now()`) ou laisser la ligne à `NULL`.

**Aucune transaction n'enveloppe ces quatre étapes, et c'est délibéré.** La règle
de transaction unique du CLAUDE.md vise les projections event-sourcées ; ici, un
appel réseau se produit entre l'étape 1 et l'étape 4, et tenir une transaction
ouverte pendant un aller-retour HTTP est précisément ce que les garde-fous de la
carte 317 cherchaient à empêcher.

**Un échec d'envoi n'interrompt pas la boucle.** Le destinataire est compté en
`failed`, sa ligne reste à `sent_at NULL`, et on continue. Un coach dont
l'adresse est invalide ne doit pas priver les vingt-neuf autres de leur email.

## Use case 1 — le cron

`send_due_notifications_use_case.rs`

```rust
pub struct SendDueNotificationsCommand {
    pub today: DateString,
}

pub struct SendDueNotificationsReport {
    pub seasons_examined:     usize,
    pub notifications_due:    usize,
    pub sent:                 usize,
    pub skipped_already_sent: usize,
    pub failed:               usize,
}

pub enum SendDueNotificationsError {
    Database(String),
}
```

Orchestration :

```
pour chaque saison candidate                    ← SQL borné sur les dates du jour
  charger réglages, journées, invitations
  dues ← notification_schedule::due_today(…)    ← domaine pur, R9
  pour chaque notification due
    dispatch(…)
```

**`today` est une entrée de la commande, pas une lecture de l'horloge.** C'est ce
qui rend le use case testable sans attendre le lendemain, et c'est aussi ce qui
permettra à la CLI d'exposer une date forcée (phase 7). Un use case qui appelle
`now()` lui-même n'est testable qu'en trichant sur l'horloge de la machine.

**Le rapport n'est pas décoratif** : c'est lui que la CLI imprime et dont elle
tire son code de sortie. `failed > 0` → `exit(1)`, ce qui rend R1 observable
dans les logs du cron. Une exécution parfaitement silencieuse et une exécution
qui a perdu douze emails ne doivent pas se ressembler.

## Use case 2 — l'ouverture des inscriptions

`send_registration_open_use_case.rs`

```rust
pub struct SendRegistrationOpenCommand {
    pub season_id: SeasonId,
}
```

Appelé depuis la validation de l'étape 5 (R11), **en tâche détachée** : la
réponse HTTP ne doit pas attendre trente envois, et un échec d'expédition ne doit
pas faire échouer la création de la compétition. L'organisateur a terminé son
magicien ; ce qui se passe ensuite est du ressort du journal, pas de sa page.

`target_date` vaut ici la date du jour — l'ouverture n'ayant pas de date propre.
La clé reste unique : deux validations successives de la même saison le même jour
ne renverraient rien.

> **Limite assumée** : deux validations à un jour d'intervalle produiraient deux
> envois, la `target_date` ayant changé. Le cas suppose un organisateur qui
> repasse l'étape 5 le lendemain ; jugé assez rare pour ne pas compliquer la clé,
> et assez visible pour être corrigé si on se trompe.

## Erreurs

| Erreur | Cause | Effet |
|---|---|---|
| `Database` | lecture des saisons ou du journal impossible | l'exécution s'arrête, `exit(1)` |
| échec d'envoi unitaire | `IEmailService` refuse ou le réseau tombe | compté en `failed`, la boucle continue |
| destinataire sans adresse | ne peut pas arriver — l'email est le seul chemin (cf. R7) | — |

La troisième ligne mérite d'être écrite justement parce qu'elle **ne peut pas
arriver** : c'est une conséquence du chemin de données, et la noter évite qu'on
ajoute un jour une garde inutile pour un cas impossible.

## Règles métier

Aucune n'apparaît à cette phase. La correction du `Option<Fixture>` en enum
relève du contrat de données, pas d'une règle nouvelle — R4 est inchangée, elle
est seulement mieux tenue.

## Ce que cette phase laisse aux suivantes

- **Phase 6** — `due_today()`, et le récapitulatif des onze règles.
- **Phase 7** — la migration et son index sur `COALESCE(round_id, '')`, les
  quatre gabarits, la sous-commande et ses arguments, le point d'accroche exact
  dans la validation de l'étape 5, et les tests.
