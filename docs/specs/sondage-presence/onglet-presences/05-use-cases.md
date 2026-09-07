# Phase 5 — Use cases : l'onglet Présences

**Entrée** : `04-dtos.md` validé — neuf commandes, `DrawProposal`, l'enum
`Presence`.

## Conventions reprises telles quelles

`save_competition_notifications.rs` donne la forme, et les neuf la suivent :

```rust
#[derive(Debug)]
pub enum LaunchSurveyError { RoundNotFound, IsRestDay, /* … */ Database(String) }

impl From<PresenceSurveyRepositoryError> for LaunchSurveyError { /* … */ }

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: LaunchSurveyCommand,
    survey_repo: &dyn IPresenceSurveyRepository,
    /* … */
) -> Result<LaunchOutcome, LaunchSurveyError>
```

`skip_all` est obligatoire — sans lui l'attribut tente d'enregistrer les dépôts,
qui n'implémentent pas `Debug`. Aucune commande ne porte de secret : le jeton est
engendré par l'agrégat, il n'entre jamais par une commande, donc `?cmd` ne peut
pas le journaliser.

Toute émission passe par `emettre()`. Un `.send(` direct reprendrait
l'identifiant de l'enveloppe reçue au lieu de celui que `to_enveloppe()` engendre,
et produirait une trace qui a l'air correcte sans rien corréler.

## Les neuf use cases

### `launch_survey_use_case`

```rust
pub async fn execute(
    cmd: LaunchSurveyCommand,
    survey_repo: &dyn IPresenceSurveyRepository,
    match_day_repo: &dyn IMatchDayRepository,
    team_port: &dyn ITeamInfoPort,
    member_port: &dyn ICompetitionSpaceMemberPort,
    expedition: &dyn ISurveyMailer,
) -> Result<LaunchOutcome, LaunchSurveyError>
```

1. charge la journée — `RoundNotFound`, `IsRestDay` (R2)
2. refuse si une campagne vit déjà sur cette journée — `SurveyAlreadyOpen` (R2)
3. appelle `survey_roster_service` : le croisement des deux ports rend les
   destinataires et ceux **sans adresse** (R3)
4. `PresenceSurvey::ouvrir(round, destinataires, deadline, auto_remind)` —
   l'agrégat engendre les réponses `SansReponse` et leurs jetons
5. persiste
6. expédie
7. rend `LaunchOutcome { destinataires, sans_adresse }`

**Aucun domain event.** 03-back l'a établi : personne hors du BC n'écoute.

`ISurveyMailer` est un trait de la couche applicative dont l'implémentation est
spécifiée dans `reponse-coach/` : le use case déclare ce qu'il lui faut, il
n'écrit pas d'e-mail. C'est aussi ce qui le rend testable sans serveur SMTP.

### `record_answer_use_case`

```rust
pub async fn execute(
    cmd: RecordAnswerCommand,
    survey_repo: &dyn IPresenceSurveyRepository,
    match_report_port: &dyn IMatchReportStatusPort,
) -> Result<AnswerOutcome, RecordAnswerError>
```

1. charge l'agrégat par `round_id` — `SurveyNotFound`
2. interroge `find_published_pairings` sur les appariements de la journée — R13
3. `survey.enregistrer(team_id, venue, par, figee)` : l'agrégat refuse une équipe
   hors campagne (R19), une journée figée (R13), et le chemin du coach sur une
   campagne close (R21)
4. persiste
5. rend `AnswerOutcome { rencontre_a_refaire: bool }` — R12/R16

**Le use case n'appelle pas le port trois fois pour trois questions.** Il charge
l'état figé une fois et le passe à l'agrégat, qui décide. La question « est-ce
autorisé ? » reste dans le domaine ; le use case fournit les faits.

**Il ne répare rien.** Quand la journée est déjà appariée, il enregistre et
signale qu'une rencontre est à refaire. C'est `repair` qui répare, sur décision
de l'organisateur — la maquette montre une proposition, pas un fait accompli.

### `remind_use_case`

Charge, liste les `SansReponse`, expédie, rend le compte. Refuse sur campagne
close — `SurveyClosed` : relancer pour un lien qui ne répond plus serait un
e-mail qui se contredit lui-même.

### `close_survey_use_case` · `reopen_survey_use_case`

Deux fichiers, deux gardes différentes. `close` refuse une campagne déjà close ;
`reopen` refuse une journée figée par un rapport publié (R13) — rouvrir réarme
les jetons (R7), donc rouvrir une journée jouée rouvrirait la porte à des
réponses sur un fait accompli.

**`reopen` reçoit une nouvelle échéance** (R23, phase 6) : la clôture étant
calculée, rouvrir sans repousser la date rouvrirait sur une campagne close dans
la seconde. Il refuse aussi une échéance déjà passée, pour la même raison.

Rouvrir une journée **appariée mais non jouée** reste permis : c'est le chemin
normal quand une défection arrive après le tirage.

### `draw_pairings_use_case`

```rust
pub async fn execute(
    cmd: DrawCommand,
    survey_repo: &dyn IPresenceSurveyRepository,
    match_day_repo: &dyn IMatchDayRepository,
    team_port: &dyn ITeamInfoPort,
) -> Result<DrawProposal, DrawError>
```

1. charge la campagne et la journée
2. refuse si la journée porte déjà des appariements — `PairingsAlreadyExist` (R11)
3. retient les présents (R5), écarte les désengagés via `filter_enrolled_team_ids` (R18)
4. refuse en dessous de deux — `NotEnoughPresent` (R15)
5. construit l'entrée du tirage et appelle le domaine
6. rend `DrawProposal` — **rien n'est écrit**

L'entrée du tirage est un type, pas six paramètres :

```rust
pub struct DrawInput {
    pub equipes:          Vec<TeamId>,
    pub historique:       RencontresJouees,          // comptes par paire — R8
    pub interdites:       HashSet<(TeamId, TeamId)>, // R10, deux équipes d'un coach
    pub jamais_exemptees: HashSet<TeamId>,           // R9
}
```

**C'est le use case qui remplit `interdites`**, parce que la relation équipe →
coach vit dans un autre BC et arrive par `ITeamInfoPort`. Le domaine reçoit des
paires interdites ; il n'a pas à savoir qu'un coach existe. La règle R10 est
métier, sa **matière** est inter-BC — c'est exactement le partage que
`CLAUDE.md` décrit pour les domain services.

### `confirm_draw_use_case`

1. recharge campagne et journée, revalide R11 et R18
2. `survey.valider_proposition(&proposal, …)` — l'agrégat **revérifie** (R22)
3. construit les `Pairing` et leurs projections (`build_new_pairing_projection`)
4. `match_day_repo.save_pairings(...)` — transaction unique, verrou sur la journée
5. **après le commit**, émet un `PairingCreated` par rencontre via `emettre()`
6. marque la campagne appariée

**L'émission vient après le commit, jamais dedans.** Un listener qui réagit à un
`PairingCreated` dont la transaction est ensuite annulée aurait travaillé sur un
fait qui n'a pas eu lieu — et rien ne le lui dirait. L'ordre inverse paraît plus
naturel (« tout dans la même unité ») et c'est le piège.

### `undo_draw_use_case`

Supprime les appariements de la journée, émet un `PairingDeleted` par rencontre,
rend la campagne à l'état clos. Refuse sur journée figée (R13) : on ne défait pas
un tirage dont un match est déjà rapporté.

### `repair_pairing_use_case`

R12 et R16. Supprime la rencontre touchée, écrit celle que l'organisateur a
validée, met à jour l'exemptée. Mêmes gardes que `confirm_draw` — la proposition
de réparation est revérifiée comme la proposition initiale (R22), et l'écriture
passe par la même méthode transactionnelle.

## Ce que ces use cases ne font pas

| Question | Où elle est traitée |
|---|---|
| « cette équipe peut-elle répondre ? » | l'agrégat (R19) |
| « cette réponse est-elle encore permise ? » | l'agrégat, sur les faits fournis (R13, R21) |
| « quelles paires former ? » | le domaine, depuis `DrawInput` (R8, R9, R10) |
| « cette proposition est-elle recevable ? » | l'agrégat (R22) |
| « quel fragment rendre ? » | le handler |

Les use cases chargent, fournissent des faits, persistent et émettent. Aucun ne
contient de `if` qui réponde à « est-ce autorisé ».

## Règles métier apparues en phase 5

### R20 — L'ouverture de la campagne ne dépend pas de la réussite des envois

La campagne est ouverte et persistée **avant** l'expédition ; un échec d'envoi
est journalisé, pas propagé. L'organisateur voit sa campagne ouverte, avec le
compte de ce qui est parti.

Un serveur de messagerie indisponible bloquerait sinon une fonction qui reste
utilisable sans lui : le coach connecté peut répondre depuis l'encart (unité 3),
et l'organisateur peut saisir à la main (R6). Le journal
`notification_deliveries` étant clé par destinataire, une relance rattrape
exactement ceux qui n'ont rien reçu — c'est le bénéfice de R3 de la spec
`notifications`, réutilisé ici sans une ligne de plus.

### R21 — La clôture ferme le chemin du coach, pas celui de l'organisateur

Sur une campagne close, un jeton répond « le sondage est clos » (R4), tandis que
l'organisateur continue de poser des réponses (R6) — la maquette le montre, avec
le badge « saisi par vous » sur l'état clos.

C'est l'agrégat qui tranche, en lisant `Repondant`. La règle n'était nulle part :
R4, R6 et R13 la supposaient chacune de leur côté, et rien ne disait ce que fait
la clôture au chemin de l'organisateur. Sans elle, la lecture naturelle — « close
veut dire close » — retirerait à l'organisateur la seule action que la maquette
lui offre à cet écran.

`Repondant` étant un champ de la commande et non une déduction du handler (phase
4), la règle tient à un seul endroit.

### R22 — La validation revérifie la proposition, elle ne l'écrit pas sur parole

L'aperçu ne persiste rien (phase 2) : la proposition voyage par le client, donc
elle revient modifiable. `confirm_draw` et `repair` la revalident intégralement —
équipes toutes présentes (R5), toutes engagées (R18), aucune paire interdite
(R10), aucune équipe en double, exemptée cohérente avec la parité.

Ce n'est pas de la défiance envers l'organisateur : c'est que **l'état a pu
changer entre l'aperçu et la validation**. Une équipe désengagée dans
l'intervalle, une réponse modifiée dans un autre onglet — la proposition affichée
était juste quand elle a été calculée, et ne l'est plus.

`PairBody` ne portant pas `historique` (phase 4), le motif de revanche est
recalculé et ne peut pas être falsifié.
