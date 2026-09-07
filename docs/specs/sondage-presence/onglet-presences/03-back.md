# Phase 3 — Architecture back : l'onglet Présences

**Entrée** : `02-front.md` validé — deux widgets, neuf actions, un panneau à six
états.

## Ce que l'existant impose

### Le BC n'est pas event-sourcé

`competition_match_days` et `competition_match_day_pairings` sont écrites en
CRUD, et la projection `competition_match_display_proj` est mise à jour dans la
foulée par `save_pairing`. Il n'y a ni event store ni rejeu : la règle
« projection et événement dans la même transaction » de `CLAUDE.md` s'applique
donc ici sous sa forme faible — **table et projection dans la même
transaction**, ce que `save_pairing` fait déjà.

### Deux domain events existent, et suffisent

`PairingCreated` et `PairingDeleted` portent déjà tous les champs d'affichage
dont la projection a besoin. Le tirage confirmé émet le premier, son annulation
et la défection de R12 émettent le second. **Aucun domain event à créer.**

La campagne elle-même n'en émet aucun, et ce n'est pas un oubli : un domain
event de ce BC n'existe que pour être converti en app event par le publisher, et
**aucun autre BC n'a à savoir qu'un sondage est ouvert**. R14 le garantit — le
classement n'est pas concerné. Le jour où un BC en aurait besoin, la règle
d'émission de `CLAUDE.md` s'appliquera : domain event d'abord, publisher
ensuite, jamais d'app event émis directement.

### Deux helpers d'appariement sont déjà écrits

`use_cases/admin/team_enrollment.rs` expose `build_new_pairing_projection`,
`resolve_team_names`, `load_enrolled_teams` et `filter_enrolled_team_ids` —
utilisés par `generate_pairings`. Le tirage confirmé les reprend tous les
quatre. Le dernier fonde R18, ci-dessous.

### La dette SQL n'est pas reprise

`match_day_repository.rs` porte ses `INSERT` en dur dans le code Rust, alors que
`CLAUDE.md` demande des fichiers sous `repositories/sql/`. Le dossier
`sql/match_days/` ne contient d'ailleurs que des lectures. Les requêtes de ce
chantier vont dans `sql/presences/`, sans exception.

## Plan de fichiers

### Domaine

| Fichier | Contenu |
|---|---|
| `domain/presence_survey.rs` | l'agrégat `PresenceSurvey`, ses VOs et ses méthodes de commande |
| `domain/presence_survey_repository_port.rs` | `IPresenceSurveyRepository` + DTOs de lecture |
| `domain/match_day.rs` | **modifié** — `generate_round_pairings` corrigée (R8, R17) |
| `domain/error.rs` | **modifié** — les refus de R2, R11, R13, R15 |

La forme complète de l'agrégat est le livrable de la phase 6, pas de celle-ci :
R12 et R16 imposent qu'il sache refaire *une* rencontre en connaissant les
autres, et cette signature se conçoit d'un bloc avec les trois unités.

### Use cases — `use_cases/presences/`

| Fichier | Mutation |
|---|---|
| `launch_survey_use_case.rs` | ouvre la campagne, appelle l'expédition |
| `record_answer_use_case.rs` | pose ou change une réponse |
| `remind_use_case.rs` | relance les sans-réponse |
| `close_survey_use_case.rs` | clôt |
| `reopen_survey_use_case.rs` | rouvre — R7 réarme les jetons |
| `draw_pairings_use_case.rs` | calcule un tirage, **n'écrit rien** |
| `confirm_draw_use_case.rs` | écrit les appariements |
| `undo_draw_use_case.rs` | les retire |
| `repair_pairing_use_case.rs` | R12/R16 — refait la rencontre touchée |

**`record_answer` est le seul point d'écriture d'une réponse**, appelé par les
trois unités : l'organisateur depuis les boutons de la carte, le coach depuis
son jeton, le coach connecté depuis l'encart. R1 porte sur l'équipe, jamais sur
le coach ; un use case par chemin d'entrée aurait fait trois endroits où tenir
R6 et R13.

**`close` et `reopen` sont deux fichiers**, pas un avec un booléen. Le workflow
interdit le use case fourre-tout, et les deux n'ont ni les mêmes gardes ni les
mêmes conséquences — rouvrir réarme des jetons, clore n'en réarme aucun.

**`draw` est instrumenté comme les autres** bien qu'il n'écrive rien : c'est une
action de l'organisateur, et le journal doit la porter. Pas de marqueur
`arch:no-instrument` ici — il est réservé aux services d'hydratation.

### Domain service

`use_cases/presences/survey_roster_service.rs` croise `find_enrolled_teams`
(équipes et `coach_id`) avec `list_space_members` (`coach_id` et `email`) et les
réponses de la campagne, et rend des objets du domaine local.

C'est le cas d'école de la section « Domain services pour données inter-BCs » de
`CLAUDE.md` : **`TeamInfoDto` et `SpaceMemberDto` n'atteignent jamais un
handler ni un gabarit.** C'est aussi ce service qui produit le compte « sans
adresse connue » de R3, par défaut de correspondance sur `coach_id`.

### IO web

| Fichier | Contenu |
|---|---|
| `io/web/admin/presences_tab.rs` | l'onglet : page entière ou fragment, selon `veut_la_page_entiere` |
| `io/web/admin/presences_widgets.rs` | les deux widgets — barre latérale, panneau |
| `io/web/admin/presences_actions.rs` | les neuf actions |

Découpage calqué sur `schedule_tab.rs` / `schedule_widgets.rs` /
`schedule_actions.rs`, à quoi s'ajoute la garde commune : **chaque handler,
fragment compris, appelle `require_admin_access`** — sans quoi le chemin htmx du
changement d'onglet contourne le contrôle d'accès, ce que le commentaire
d'`admin_page.rs` documente déjà.

### Templates

| Fichier | Rendu par |
|---|---|
| `templates/admin/presences.html` | la page hôte — assemblage pur, deux conteneurs `hx-get` |
| `templates/admin/widgets/presences-rounds.html` | la barre latérale |
| `templates/admin/widgets/presences-panel-empty.html` | aucun sondage |
| `templates/admin/widgets/presences-panel-running.html` | en cours / clos |
| `templates/admin/widgets/presences-panel-draw.html` | tirage proposé |
| `templates/admin/widgets/presences-panel-paired.html` | journée appariée |
| `templates/admin/widgets/presences-panel-defection.html` | défection à traiter |

Cinq gabarits pour six états : « en cours » et « clos » partagent le leur, comme
la maquette qui les rend avec le même markup et deux bandeaux.

**Six structs `Template`, pas un gabarit à branches.** Le `{% include %}`
d'Askama n'accepte qu'un chemin littéral : un fichier unique aurait voulu dire
six `{% if %}` imbriqués. Le handler choisit la struct et rend `Html<String>` —
`admin_page.rs` procède déjà ainsi avec son champ `content`.

### Repository et schéma

`io/repository/presence_survey_repository.rs`, requêtes dans `sql/presences/`.

```
competition_presence_surveys
    id, season_id, round_id, deadline, auto_remind, opened_at
    close_le TIMESTAMPTZ NULL                          ← Fermeture::Decidee
    UNIQUE (round_id)                                  ← R2

competition_presence_answers
    id, survey_id, team_id, coach_id, token
    presence TEXT NOT NULL CHECK (presence IN ('sans_reponse','presente','absente'))
    repondu_le      TIMESTAMPTZ NULL
    saisi_par_admin TEXT NULL                          ← R6
    UNIQUE (survey_id, team_id)                        ← R1
    UNIQUE (token)
    CHECK (
      (presence  = 'sans_reponse' AND repondu_le IS NULL AND saisi_par_admin IS NULL)
      OR (presence <> 'sans_reponse' AND repondu_le IS NOT NULL)
    )
```

Le jeton vit dans la table des réponses et non dans une table à part : R7 lui
refuse toute échéance propre, donc il n'a rien à porter qu'une réponse ne porte
déjà. Une table de jetons aurait dupliqué la clé `(survey_id, team_id)` pour ne
rien ajouter.

### La présence est un enum à données portées, jamais un booléen nullable

Trois états — sans réponse, présente, absente — dont deux seulement portent un
horodatage et un auteur. Le type doit rendre les combinaisons absurdes
inexprimables :

```rust
pub enum Presence {
    SansReponse,
    Declaree {
        venue: Venue,         // Presente | Absente
        le:    ReponduLe,
        par:   Repondant,
    },
}

pub enum Repondant {
    Coach,                    // par son jeton, ou depuis l'encart
    Organisateur(CoachId),    // R6 — le badge « saisi par vous »
}
```

**Un enum plat n'aurait rien réglé.** `presence: Presence` avec `repondu_le:
Option<…>` et `saisi_par: Option<…>` à côté remplace un `Option` par deux, et
laisse construire une réponse déclarée sans horodatage — ou un horodatage sans
réponse. C'est le même défaut, réparti.

Ce que l'enum porté fait gagner concrètement :

| Question | Réponse |
|---|---|
| « compte pour le tirage ? » | `matches!(self, Declaree { venue: Presente, .. })` — R5 tient dans le type |
| « qui a répondu ? » | `Repondant`, jamais absent quand une réponse existe — R6 |
| « depuis quand ? » | `ReponduLe`, jamais absent non plus |

**Les `NULL` restent en base, et n'en sortent pas.** Postgres n'a pas de type
somme : les trois colonnes ci-dessus sont la projection à plat de l'enum, et le
`CHECK` interdit les combinaisons que le type Rust ne sait pas exprimer. Le
dépôt reconstruit l'enum à la lecture ; une ligne incohérente — qu'aucun chemin
d'écriture ne peut produire, et que le `CHECK` refuse — est une erreur de dépôt,
traitée comme les `try_new(...).map_err(db_err)?` du reste du projet.

C'est la même répartition que partout ailleurs ici : **la contrainte de base
garantit, le type exprime.** L'index unique de `notification_deliveries` suit
déjà ce principe — sa spec dit que c'est la contrainte qui protège, pas le code
applicatif.

### Le statut de la campagne n'est pas stocké du tout

**Corrigé en phase 6.** Cette section décrivait une colonne `statut` et un
`close_le` tenus cohérents par un `CHECK` — deux façons de dire la même chose,
donc une de trop. R23 tranche : le statut est **calculé** depuis l'échéance et
la décision de clôture, jamais persisté.

```rust
pub enum Fermeture { Aucune, Decidee { le: FermeeLe } }
// statut(maintenant) croise `fermeture` et `deadline` — cf. 06-domaine.md
```

Une colonne `statut` aurait pu dire « ouverte » sur une campagne échue, et rien
n'aurait signalé la divergence : la seule façon de ne pas avoir à synchroniser
deux vérités est de n'en stocker qu'une.

`opened_at` reste un champ ordinaire — une campagne qui existe a toujours été
ouverte, il n'y a pas d'état où la date manque.

### Contexte et routes

`CompetitionsContext` reçoit `presence_survey_repository: Arc<dyn
IPresenceSurveyRepository>`, instancié dans `main.rs`. **Aucun port nouveau** :
`ITeamInfoPort`, `ICompetitionSpaceMemberPort` et `IMatchReportStatusPort` y
sont déjà.

Douze constantes dans `routes.rs`, sur le patron des dix-huit de `schedule` :

```
/app/{space_id}/competitions/{competition_id}/{season_id}/admin/presences
                                                          …/presences/rounds
                                                          …/presences/panel
                                                          …/presences/launch
                                                          …/presences/answer
                                                          …/presences/remind
                                                          …/presences/close
                                                          …/presences/reopen
                                                          …/presences/draw
                                                          …/presences/confirm-draw
                                                          …/presences/undo-draw
                                                          …/presences/repair
```

Et l'onglet dans `admin-page.html`, entre Calendrier et Paramètres.

## L'écriture du tirage est atomique

Valider un tirage écrit N appariements, N projections et l'état de la campagne.
Écrits un par un, une panne au troisième laisse une journée à moitié appariée —
et deux organisateurs qui valident en même temps passent tous les deux la garde
de R11 avant que l'un ait écrit.

**Forme retenue** : une méthode plurielle et transactionnelle sur
`IMatchDayRepository`.

```rust
async fn save_pairings(
    &self,
    match_day_id: &str,
    pairings: &[(Pairing, NewPairingProjection)],
) -> Result<(), MatchDayRepositoryError>;
```

Elle ouvre la transaction, prend un `SELECT … FOR UPDATE` sur la journée — ce
qui fait tenir R11 face à la course — écrit les paires et leurs projections,
puis commet.

**`save_pairing` n'est pas modifiée.** Elle a trois appelants réels
(`generate_pairings`, `app_events/appariement.rs`,
`match_report_cancelled_listener`) et trois mocks de test ; les deux derniers
appelants écrivent **un seul** appariement et n'ont rien à gagner à une
transaction. Seul `generate_pairings` migre vers la méthode plurielle : il écrit
N appariements et souffre exactement du même défaut que celui qu'on corrige ici.

C'est la mise en œuvre de la décision « transaction unique » sans l'ampleur
annoncée à tort au moment de la prendre — un appelant à migrer, pas six sites à
réécrire.

## Frontière avec `reponse-coach/`

`launch_survey_use_case` et `remind_use_case` **déclenchent** l'expédition ; ils
ne la définissent pas. Le point d'entrée est nommé ici :

```rust
send_survey_emails(survey: &PresenceSurvey, destinataires: &[…]) -> …
```

Son gabarit, la fabrication du jeton, la route publique et le journal d'envoi
sont le contenu de `reponse-coach/03-back.md`. Ce qu'on peut déjà dire : il
réutilisera `IEmailService` (déjà dans le contexte), la table
`notification_deliveries` et un cinquième `NotificationType` — un envoi par
destinataire, jamais groupé, R7 de la spec `notifications` valant ici pour la
même raison.

## Règle métier apparue en phase 3

### R18 — Le tirage revérifie l'engagement au moment d'écrire

Une équipe peut être désinscrite de la saison entre sa réponse au sondage et la
validation du tirage. `confirm_draw` refiltre donc les équipes retenues par
`filter_enrolled_team_ids`, comme `generate_pairings` le fait déjà, et signale
celles qu'il écarte.

Sans cette vérification, une réponse « je serai là » vieille de cinq jours
suffirait à créer un appariement pour une équipe qui n'est plus dans la
compétition. La présence dit qu'un coach vient ; elle ne dit pas qu'il est
toujours inscrit.

**Conséquence sur `draw`** : l'aperçu doit filtrer aussi, sans quoi il
proposerait une rencontre que la validation refuserait — l'organisateur verrait
son tirage changer sans avoir rien fait.
