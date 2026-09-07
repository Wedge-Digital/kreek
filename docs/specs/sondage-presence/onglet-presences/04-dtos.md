# Phase 4 — Contrats de données : l'onglet Présences

**Entrée** : `03-back.md` validé — neuf use cases, deux widgets, six états de
panneau, l'enum `Presence` à données portées.

## Ce que l'existant impose

**Le DTO d'entrée est plat, la commande porte les value objects.** `schedule_actions.rs`
désérialise des structs à `String` nues (`RoundIdBody`, `AddMatchBody`), et c'est
conforme : `CLAUDE.md` place la construction des VO dans le handler, par leurs
smart constructors. La frontière est donc **DTO plat → `try_new` → commande
typée**, et le refus de validation est un `400`, jamais un refus métier.

**Les VM vivent avec leur widget.** Ce BC n'a pas de `view_models.rs` : `RoundItemVm`
et `RoundDetailVm` sont déclarés dans `schedule_widgets.rs`. On suit.

**Pas de `builders.rs`.** La règle de `CLAUDE.md` le réserve aux VM qui dépendent
de DTO de port ; ici le domain service `survey_roster_service` (phase 3) rend
déjà des objets domaine, donc tous les VM ont un `from_domain()` co-localisé.

**Le piège `team_options_json`.** `RoundDetailVm` porte les équipes **déjà
sérialisées** parce qu'Askama échappe en entités HTML, que le navigateur ne
décode pas dans un `<script>` : « L'Ost » s'y affichait `L&#x27;Ost`. L'onglet
Présences n'interpole rien dans un `<script>` — aucune liste JSON à passer — le
piège ne se présente donc pas. À ne pas rouvrir en chemin.

## Les value objects du domaine

Aucun identifiant nouveau : `SeasonId`, `MatchId` (la journée), `PairingId`,
`CoachId` et `TeamId` existent au `shared_kernel`.

| VO | Forme | Règle |
|---|---|---|
| `SurveyId` | `EntityId` | — |
| `SurveyToken` | `nutype` sur `SUlid`, distinct de `EntityId` | R7 — il voyage dans une URL publique |
| `SurveyDeadline` | `nutype` sur `DateString` | R4 — au-delà, les liens ne répondent plus |
| `AutoRemind` | `nutype(bool)` | le champ de la case à cocher |
| `Presence` | enum à données portées (03-back) | R5, R6 |
| `Venue` | `Presente` \| `Absente` | — |
| `Repondant` | `Jeton` \| `Coach(CoachId)` \| `Organisateur(CoachId)` | R6, R28 |
| `ReponduLe` | `nutype` sur `OffsetDateTime` | — |
| `SurveyStatus` | `Ouverte` \| `Close { le }` | 03-back |

**`SurveyToken` n'est pas un `EntityId`**, bien qu'il en ait la forme. Un
identifiant technique et un secret d'URL n'ont ni le même cycle de vie ni les
mêmes règles de divulgation ; les confondre autoriserait à écrire
`survey.id` là où le jeton est attendu, et le compilateur ne dirait rien.

## Les commandes

Une par use case mutant. Aucune primitive nue — R1 impose `TeamId` partout où
une réponse est en jeu.

```rust
pub struct LaunchSurveyCommand {
    pub season_id: SeasonId,
    pub round_id:  MatchId,
    pub deadline:  SurveyDeadline,
    pub auto_remind: AutoRemind,
}

pub struct RecordAnswerCommand {
    pub survey_id: SurveyId,
    pub team_id:   TeamId,
    pub venue:     Venue,
    pub par:       Repondant,     // R6 — jamais déduit du contexte d'appel
}

pub struct DrawCommand        { pub survey_id: SurveyId }
pub struct ConfirmDrawCommand { pub survey_id: SurveyId, pub proposal: DrawProposal }
pub struct RepairCommand      { pub survey_id: SurveyId, pub proposal: DrawProposal }
```

`RemindCommand`, `CloseSurveyCommand` et `UndoDrawCommand` ne portent que
`survey_id`.

`ReopenSurveyCommand` porte **en plus une `SurveyDeadline`** — corrigé en phase
6. R23 rend la clôture calculée : rouvrir sans repousser l'échéance rouvrirait
sur une campagne qui se referme dans la seconde. `SurveyIdBody` ne convient donc
pas pour cette action, qui a son `ReopenBody { round_id, deadline }`.

**`Repondant` est un champ de la commande, pas une déduction du handler.** Les
trois unités appellent le même use case (03-back) ; si l'auteur se déduisait de
la route, la page publique et l'encart devraient chacun le reconstituer, et R6
tiendrait à trois endroits au lieu d'un.

**`ConfirmDrawCommand` porte la proposition entière**, pas seulement l'identifiant
de la campagne. L'aperçu ne persiste rien (02-front) : ce que l'organisateur a
sous les yeux doit donc voyager jusqu'à l'écriture, sinon le serveur retirerait
au sort et écrirait autre chose que ce qui était affiché.

## Ce que le tirage rend

```rust
pub struct DrawProposal {
    pub rencontres: Vec<ProposedPairing>,
    pub exemptee:   Option<TeamId>,     // absente si l'effectif est pair
    pub ecartees:   Vec<TeamId>,        // R18 — désengagées depuis leur réponse
}

pub struct ProposedPairing {
    pub home: TeamId,
    pub away: TeamId,
    pub historique: Historique,
}

pub enum Historique {
    Inedite,
    Revanche { fois: NombreDeRencontres, derniere: MatchDayName },
}
```

**`Historique` vient du domaine, et c'est la règle qui compte ici.** La maquette
affiche « 2ᵉ rencontre · J1 » : si le VM devait le déduire en relisant les
journées, on aurait un calcul métier dans la présentation — précisément ce que
« un view model transpose, il ne dérive pas » interdit, et ce qui a produit
quatre défauts en trois cartes ailleurs dans le projet. Le tirage sait *pourquoi*
il a concédé ; il le dit.

`exemptee` est un `Option` et non un troisième variant : l'absence d'exemption
est l'état normal d'un effectif pair, elle ne porte aucune donnée, et R9 ne s'y
applique pas. C'est le seul `Option` de cette phase, et il est là où il a un sens.

## Les DTO d'entrée HTTP

Tous en corps JSON, désérialisés à plat, convertis par le handler.

| DTO | Champs | Devient |
|---|---|---|
| `LaunchBody` | `round_id`, `deadline`, `auto_remind` | `LaunchSurveyCommand` |
| `AnswerBody` | `round_id`, `team_id`, `presence` (`"presente"`/`"absente"`) | `RecordAnswerCommand` |
| `SurveyIdBody` | `round_id` | `Remind`/`Close`/`Draw`/`UndoDraw` |
| `ReopenBody` | `round_id`, `deadline` | `ReopenSurveyCommand` — R23 |
| `ConfirmDrawBody` | `round_id`, `rencontres: Vec<PairBody>`, `exemptee` | `ConfirmDrawCommand` |
| `RepairBody` | `round_id`, `rencontre: PairBody`, `exemptee` | `RepairCommand` |
| `PairBody` | `home_team_id`, `away_team_id` | `ProposedPairing` |

`PairBody` ne porte pas `historique` : le client n'a pas à renvoyer un motif que
le serveur recalcule de toute façon avant d'écrire. Le lui faire porter
autoriserait à mentir sur l'historique d'une rencontre.

**La campagne est désignée par `round_id`, jamais par `survey_id`.** R2 garantit
une seule campagne vivante par journée, donc la journée suffit à la nommer — et
le client n'a pas à connaître un identifiant qu'il ne lit nulle part. Le use case
résout `round_id → SurveyId`, et son échec est le refus de R2 plutôt qu'un `404`
énigmatique.

`space_id`, `competition_id` et `season_id` restent dans le chemin, comme les dix-huit
routes de `schedule`, et `require_admin_access` en vérifie la cohérence.

## Les DTO de lecture du repository

Primitives acceptées — ce sont des types de query, sans invariant à protéger
(`CLAUDE.md`, exception « DTOs de lecture »).

```rust
pub struct SurveySummaryDto {      // une ligne de la barre latérale
    pub round_id: String,
    pub round_name: String,
    pub statut: String,
    pub reponses: i64,
    pub attendues: i64,
    pub presents: i64,
    pub appariee: bool,
}
```

**Une requête pour toute la saison, pas une par journée.** La barre latérale
affiche l'état de chaque journée ; les compter une à une ferait vingt allers-retours
pour une colonne.

## Les structs de template et leurs VM

### Barre latérale

```rust
#[derive(Template)] #[template(path = "admin/widgets/presences-rounds.html")]
pub struct PresenceRoundsTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String, pub competition_id: String, pub season_id: String,
    pub rounds: Vec<PresenceRoundItemVm>,
}

pub struct PresenceRoundItemVm {
    pub round_id: String,
    pub name: String,
    pub etat: String,        // "aucun" | "en_cours" | "clos" | "apparie" | "defection"
    pub resume: String,      // « 10 réponses sur 14 », « 4 matchs créés »
    pub is_rest: bool,
}
```

`etat` et `resume` sont **calculés par le domaine**, pas par le gabarit. Le
gabarit choisit une pastille et imprime une phrase ; il ne décide pas laquelle.

### Les six panneaux

| Struct | Gabarit | VM portés |
|---|---|---|
| `PanelEmptyTemplate` | `presences-panel-empty.html` | `DestinatairesVm` |
| `PanelRunningTemplate` | `presences-panel-running.html` | `CampagneVm`, `AvancementVm`, trois `Vec<AnswerRowVm>` |
| `PanelDrawTemplate` | `presences-panel-draw.html` | `DrawVm` |
| `PanelPairedTemplate` | `presences-panel-paired.html` | `DrawVm` |
| `PanelDefectionTemplate` | `presences-panel-defection.html` | `DefectionVm`, `DrawVm` |

Tous portent en plus `app_routes`, les trois identifiants de chemin et
`RoundHeadVm` — l'en-tête de journée est commun aux six états.

```rust
pub struct DestinatairesVm {   // R3
    pub equipes: usize,
    pub coachs: usize,
    pub sans_adresse: usize,
    pub deadline_defaut: String,
}

pub struct AvancementVm {      // la barre segmentée
    pub presents: usize,
    pub absents: usize,
    pub sans_reponse: usize,
    pub engagees: usize,
}

pub struct AnswerRowVm {
    pub team_id: String,
    pub team_name: String,
    pub coach_label: String,     // « Lepandawan · 2 équipes »
    pub initiales: String,
    pub repondu_le: String,      // vide si sans réponse
    pub saisi_par_admin: bool,   // R6 — le badge « saisi par vous »
}

pub struct DrawVm {
    pub rencontres: Vec<DrawRowVm>,
    pub exemptee: Option<String>,
    pub inedites: usize,
    pub revanches: usize,
    pub ecartees: Vec<String>,   // R18
}

pub struct DrawRowVm {
    pub home: String, pub away: String,
    pub tag: String,             // « 1re rencontre » | « 2e rencontre · J1 »
    pub est_revanche: bool,
}
```

**`AvancementVm` reçoit quatre comptes du domaine ; il n'en dérive aucun.** Le
réflexe serait de faire `rows.len()` sur chaque colonne — c'est exactement le
défaut de la carte 495, où la vue recomptait ce que le domaine savait compter, et
comptait autre chose. `PresenceSurvey` expose `compte_presents()`,
`compte_absents()`, `compte_sans_reponse()` et `engagees()`.

Même raison pour `DrawVm::inedites` et `revanches` : ils viennent du domaine, qui
a produit les `Historique`. Les recompter en parcourant `rencontres` marcherait
aujourd'hui et deviendrait faux le jour où une rencontre porterait un troisième
motif.

**`coach_label` est construit par le domain service**, pas par le gabarit : « 2
équipes » suppose de savoir combien d'équipes ce coach engage dans cette saison,
ce qui est une question sur le roster de la campagne, pas sur la ligne affichée.

## Interfaces d'utilisation

| Type | Émis par | Consommé par |
|---|---|---|
| `LaunchBody`, `AnswerBody`, `SurveyIdBody`, `ConfirmDrawBody`, `RepairBody` | le navigateur (htmx) | `presences_actions.rs` |
| `LaunchSurveyCommand` … `RepairCommand` | `presences_actions.rs` | les use cases de `use_cases/presences/` |
| `Presence`, `Venue`, `Repondant`, `SurveyStatus` | l'agrégat `PresenceSurvey` | l'agrégat, le dépôt, le domain service |
| `TeamInfoDto`, `SpaceMemberDto` | `ITeamInfoPort`, `ICompetitionSpaceMemberPort` | **`survey_roster_service` seul** — jamais un handler ni un gabarit |
| `SurveySummaryDto` | `IPresenceSurveyRepository` | `presences_widgets.rs` → `PresenceRoundItemVm` |
| `DrawProposal`, `ProposedPairing`, `Historique` | `draw_pairings_use_case` | `presences_actions.rs` (→ `DrawVm`), `confirm_draw_use_case` |
| `PresenceRoundItemVm`, `AnswerRowVm`, `DrawVm`, `AvancementVm`, … | `from_domain()` co-localisés | les gabarits Askama, seuls |
| `NewPairingProjection`, `Pairing` | `confirm_draw_use_case` via `build_new_pairing_projection` | `IMatchDayRepository::save_pairings` |

La ligne qui compte : **les DTO de port s'arrêtent au domain service.** C'est la
règle « Domain services pour données inter-BCs », et c'est aussi ce qui rend le
panneau testable sans HTTP.

## Règle métier apparue en phase 4

### R19 — Une réponse ne vaut que pour une équipe de la campagne

`RecordAnswerCommand` porte un `TeamId` venu du navigateur ou d'un jeton.
L'agrégat refuse une équipe qui n'appartient pas à sa liste de destinataires —
désengagée depuis l'ouverture, ou jamais engagée.

Sans cette garde, un organisateur pourrait poser une présence pour une équipe
d'une autre compétition du même espace : `require_admin_access` vérifie que la
saison appartient à la compétition, jamais que l'équipe appartient à la campagne.
C'est la même classe de trou que celui refermé par la carte 416, à un étage plus
bas.

**Distincte de R18**, qui refiltre au moment d'écrire le tirage : R19 refuse
d'enregistrer, R18 refuse d'apparier. Une équipe désengagée après avoir répondu
garde sa réponse — elle est simplement écartée du tirage, et l'écran le dit.
