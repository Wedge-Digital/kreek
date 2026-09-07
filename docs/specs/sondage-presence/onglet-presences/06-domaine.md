# Phase 6 — Domaine : l'agrégat `PresenceSurvey`

**Entrée** : `05-use-cases.md` validé. Les vingt-trois règles et la forme de
l'agrégat ont été présentées et validées avant écriture.

## Récapitulatif des règles métier — validé

| # | Règle | Née en | Portée par |
|---|---|---|---|
| **Le cycle de la campagne** ||||
| R1 | La réponse porte sur l'équipe engagée, jamais sur le coach | 1 | `ouvrir` |
| R2 | Un seul sondage vivant par journée, jamais sur une journée de repos | 1 | `ouvrir` + index unique |
| R3 | Un coach sans adresse connue n'empêche pas le lancement | 1 | `ouvrir` |
| R20 | L'ouverture ne dépend pas de la réussite des envois | 5 | use case |
| R23 | La clôture est calculée, jamais subie | 6 | `statut`, `rouvrir` |
| **La réponse** ||||
| R4 | Le clic enregistre, et la page affiche la réponse inverse | 1 | unité `reponse-coach` |
| R5 | Sans réponse vaut absent pour le tirage, mais se compte à part | 1 | `Presence`, `presents` |
| R6 | L'organisateur peut répondre à la place, et cela se voit | 1 | `Repondant` |
| R7 | Le jeton n'a pas de durée de vie propre | 1 | `statut` |
| R19 | Une réponse ne vaut que pour une équipe de la campagne | 4 | `enregistrer` |
| R21 | La clôture ferme le chemin du coach, pas celui de l'organisateur | 5 | `enregistrer` |
| **Le tirage** ||||
| R8 | Trois objectifs ordonnés : apparier le maximum, rejouer le moins, tirer au sort | 1 | `tirer` |
| R9 | L'exemption ne se répète pas | 1 | `tirer`, `marquer_appariee` |
| R10 | Deux équipes d'un même coach ne se rencontrent jamais | 1 | `tirer`, `valider_proposition` |
| R11 | Refuse une journée qui porte déjà des appariements | 1 | use case |
| R15 | Refuse en dessous de deux présents, et le dit | 2 | `peut_tirer` |
| R17 | C'est un vrai tirage : « retirer au sort » change le résultat | 1 | `tirer` |
| R18 | Revérifie l'engagement au moment d'écrire | 3 | `valider_proposition` |
| R22 | La validation revérifie la proposition | 5 | `valider_proposition` |
| **Après le tirage** ||||
| R12 | Une modification de présence ne refait que les rencontres touchées | 1 | `enregistrer`, `desaccord` |
| R16 | Une arrivée tardive se traite comme une défection, en sens inverse | 2 | idem |
| R24 | Les rencontres appartiennent à la journée, la campagne retient l'exemption | 7 | `Appariement`, `EtatJournee`, `desaccord` |
| R28 | Un coach connecté ne répond que pour ses propres équipes | 2 (unité 3) | `Repondant`, `enregistrer` |
| R13 | La correction se ferme au premier rapport de match publié | 1 | `enregistrer`, `rouvrir` |
| R14 | Une équipe non appariée ne vaut rien au classement | 1 | *rien* — c'est une non-action |

**R14 n'est portée par aucun code**, et c'est volontaire : ne rien faire ne
s'implémente pas. Elle reste écrite parce que la question se reposera, et que la
réponse « on ne touche pas au classement » doit être trouvable.

## R23 — La clôture est calculée, jamais subie

Une campagne est close dès que son échéance est passée, sans qu'aucune tâche
n'ait à l'écrire. `statut(maintenant)` résout trois situations à partir de deux
champs :

| `fermeture` | échéance | `statut` |
|---|---|---|
| `Aucune` | à venir | `Ouverte` |
| `Aucune` | passée | `Close(Echeance)` |
| `Decidee { le }` | quelconque | `Close(Decision)` |

**Écarté : une tâche planifiée.** La sous-commande CLI de la spec
`notifications` aurait pu clore les campagnes échues au passage, mais une
campagne serait restée ouverte jusqu'à vingt-quatre heures après son échéance —
et ses liens auraient répondu pendant ce temps, contre ce que l'e-mail annonce
noir sur blanc. Le calcul n'a pas de retard possible.

**Conséquence : rouvrir suppose une nouvelle échéance.** `rouvrir` prend une
`SurveyDeadline`, faute de quoi la campagne se refermerait dans la seconde. La
phase 4 disait que `ReopenSurveyCommand` ne portait que l'identifiant ; elle est
corrigée.

## L'agrégat

```rust
pub struct PresenceSurvey {
    id:          SurveyId,
    season_id:   SeasonId,
    round_id:    MatchId,
    deadline:    SurveyDeadline,
    auto_remind: AutoRemind,
    opened_at:   OpenedAt,
    fermeture:   Fermeture,
    reponses:    Vec<Reponse>,
    appariement: Appariement,
}

pub enum Fermeture   { Aucune, Decidee { le: FermeeLe } }
pub enum Appariement { Aucun, Fait { exemptee: Option<TeamId> } }   // corrigé en phase 7 — R24

pub struct Reponse {
    team_id:  TeamId,
    coach_id: CoachId,
    token:    SurveyToken,
    presence: Presence,
}
```

`Fermeture` est un enum et non un `Option<FermeeLe>` : croisé à l'échéance, il
produit les trois situations de R23, qu'un `Option` aurait laissé reconstituer à
chaque appelant.

`exemptee` reste un `Option`, et c'est le seul de l'agrégat. Il porte une valeur
seule, sans co-champ qui pourrait devenir orphelin — le défaut que l'enum à
données portées corrige n'existe pas ici, et l'absence d'exemption est l'état
normal d'un effectif pair.

**`Fait` ne porte plus `rencontres` — corrigé en phase 7, R24.** Les
appariements appartiennent à la journée, qui les possède en base ; les recopier
ici créait une seconde vérité qu'aucune transaction ne tenait avec la première,
puisque quatre chemins du Calendrier les suppriment sans rien savoir d'une
campagne. L'exemptée reste, parce qu'elle n'existe nulle part ailleurs et que R9
a besoin de l'historique des exemptions de la saison.

**Aucun champ n'est `pub`.**

### Construction

| Signature | Règles |
|---|---|
| `ouvrir(id, season, round: &MatchDay, destinataires, deadline, auto_remind, maintenant) -> Result<Self, DomainError>` | R1, R2, R3 |
| `rehydrater(…) -> Self` | dépôt seul |

`ouvrir` reçoit **`&MatchDay`**, pas un identifiant : « peut-on sonder cette
journée ? » est une question métier, et la répondre dans le use case l'aurait
sortie du domaine. Elle engendre une `Reponse` par équipe destinataire — R1 —
avec son jeton et `Presence::SansReponse`, y compris pour les coachs sans
adresse : R3 dit que leur équipe entre dans la campagne, seul l'e-mail manque.

### Commandes

| Signature | Règles |
|---|---|
| `enregistrer(&mut self, team, venue, par, journee: &EtatJournee, maintenant) -> Result<EffetReponse, DomainError>` | R19, R13, R21, R12, R16 |
| `clore(&mut self, maintenant) -> Result<(), DomainError>` | R23 |
| `rouvrir(&mut self, nouvelle_deadline, figee, maintenant) -> Result<(), DomainError>` | R13, R23 |
| `valider_proposition(&self, prop, engagees, interdites) -> Result<(), DomainError>` | R5, R10, R18, R22 |
| `marquer_appariee(&mut self, exemptee)` | R9 |
| `defaire_appariement(&mut self)` | — |

**Six commandes, pas sept — corrigé en phase 7, R24.** `remplacer_rencontre`
n'avait plus rien à remplacer : les rencontres ayant quitté l'agrégat, une
réparation ne change chez lui que l'exemptée, et c'est exactement ce que
`marquer_appariee` fait — l'appeler de nouveau avec la nouvelle exemptée
enregistre l'exemption qui a *finalement* eu lieu. Garder les deux noms aurait
été deux portes sur un seul comportement.

```rust
pub enum EffetReponse {
    Enregistree,
    EnregistreeRencontreARefaire { pairing: PairingId },
}
```

C'est ce type qui permet à `record_answer` de **signaler sans réparer** : la
maquette montre une proposition que l'organisateur valide, jamais un fait
accompli. Un `Result<(), _>` aurait obligé le use case à redécouvrir tout seul
qu'une rencontre est touchée.

`enregistrer` reçoit **`&EtatJournee`** — des faits, pas des ports :

```rust
pub struct EtatJournee {
    pub figee:      bool,                    // R13 — un rapport publié
    pub rencontres: Vec<RencontreJournee>,   // (pairing_id, home, away) — R24
}
```

Le use case interroge `IMatchReportStatusPort` et le dépôt de journées une fois
chacun, et passe le résultat ; l'agrégat décide. La question « est-ce
autorisé ? » reste dans le domaine, les faits viennent du dehors.

**`rencontres` a remplacé `figee: JourneeFigee` seul — corrigé en phase 7,
R24.** Depuis que l'agrégat ne possède plus les appariements, c'est ce fait qui
lui permet de répondre à R12 : il y cherche lui-même la rencontre de l'équipe
qui change d'avis, plutôt que de la recevoir toute trouvée. Un paramètre
`rencontre_de: Option<PairingId>` calculé par le use case aurait sorti du
domaine la question « quelle rencontre est touchée ? ».

**`marquer_appariee` enregistre l'exemption qui a eu lieu, pas celle qui était
proposée.** La nuance vient de R9 croisée à R12 : quand l'exemptée reprend du
service après une défection, elle n'a finalement pas été exemptée, et la
compter comme telle la ferait passer devant à la journée suivante pour une
exemption qu'elle n'a pas subie.

### R28 — Trois chemins, trois autorisations

**Corrigé en phase 2 de l'unité `encart-competition`.** `Repondant` valait
`Coach | Organisateur(CoachId)`, et fondait deux chemins qui n'ont pas la même
autorisation :

| Chemin | Ce qui autorise |
|---|---|
| le jeton reçu par e-mail | **le jeton lui-même** — qui le détient répond |
| l'encart du coach connecté | **la session**, et rien ne vérifiait que l'équipe est la sienne |
| l'organisateur | `require_admin_access`, et le badge de R6 |

R19 vérifie que l'équipe est **dans la campagne**, jamais qu'elle appartient au
répondant : depuis l'encart, un `team_id` forgé aurait posé une présence pour
l'équipe d'un autre.

```rust
pub enum Repondant { Jeton, Coach(CoachId), Organisateur(CoachId) }
```

`enregistrer` refuse un `Coach(id)` dont l'identifiant ne correspond pas au
`coach_id` de la réponse — **`Reponse` le porte déjà**, le domaine savait
répondre, personne ne lui posait la question.

`Jeton` n'est pas contrôlé, et ce n'est pas un oubli : le jeton *est*
l'autorisation, comme R7 le dit. Lui faire porter un `CoachId` aurait produit un
contrôle circulaire — comparer la réponse à elle-même.

**Le canal ne se persiste pas, et c'est délibéré.** La table garde deux cas —
`saisi_par_admin` renseigné ou `NULL` — parce que c'est tout ce qu'un lecteur
demande : R6 distingue l'organisateur du coach, jamais le jeton de l'encart. À
la relecture, `NULL` redonne `Coach(coach_id de la réponse)`, qui est
exactement vrai : le coach a répondu. **Le canal est un fait d'autorisation,
vivant le temps de l'écriture** — pas une propriété de la réponse.

Une colonne de plus l'aurait conservé, et personne n'aurait su quoi en faire :
la question qu'on se pose après coup est « qui a dit qu'il venait », et elle a
deux réponses possibles, pas trois.

**Ce que ça dit du pari « l'agrégat se conçoit d'un bloc ».** Il le passe à
moitié : la forme était bonne — `Repondant` existait, les trois unités appellent
bien le même `enregistrer` — mais deux chemins avaient été fondus en une
variante, et seule la troisième unité l'a fait voir. C'est l'argument du workflow
retourné : la troisième méthode révèle que les deux premières avaient la mauvaise
signature, et ici c'est le troisième **appelant**.

### Requêtes

`statut(maintenant)`, `presents()`, `compte_presents()`, `compte_absents()`,
`compte_sans_reponse()`, `engagees()`, `sans_reponse()`, `reponse_par_jeton(token)`,
`desaccord(&self, journee: &EtatJournee)`, `peut_tirer()`.

**`rencontre_a_refaire()` est devenue `desaccord(journee)` — corrigé en phase 7,
R24.** La première lisait un état stocké : elle ne savait qu'un désaccord existe
que parce que la campagne se souvenait des rencontres. La seconde croise les
présences avec les appariements réels de la journée et rend ce qui ne concorde
pas — un présent que rien n'apparie, un apparié devenu absent.

Le bénéfice n'est pas cosmétique : **l'état « défection à traiter » survit
désormais à un rechargement de page**, puisqu'il se recalcule au lieu de se
lire. Avec la forme précédente, il tenait à ce que l'agrégat et la journée
restent d'accord — ce que R24 a précisément montré impossible.

Les quatre compteurs existent parce que `AvancementVm` ne doit rien dériver
(phase 4). `reponse_par_jeton` et `sans_reponse` servent les unités 2 et 3 :
l'agrégat est conçu pour les trois d'un bloc.

### Encapsulation

Les `Vec` internes ne sortent jamais en `&mut`. `presents()` et `sans_reponse()`
rendent des `Vec<&Reponse>`, et `Reponse` n'expose que des getters. Le dépôt lit
par ces getters, comme `save_notifications` reçoit `&CompetitionNotifications`.

**Le seul chemin d'écriture d'une `Presence` est `enregistrer`**, qui porte R19,
R13 et R21 ensemble. Un `pub` sur `reponses` rendrait les trois contournables par
un `survey.reponses[0].presence = …` que rien ne signalerait — ni le
compilateur, ni `check-arch`, ni la revue.

## Le tirage n'est pas dans l'agrégat

`domain/tirage.rs`, fonction pure, en remplacement de `generate_round_pairings` :

```rust
pub fn tirer(input: &DrawInput, rng: &mut impl Rng) -> DrawProposal
```

Il a besoin de l'historique de toute la saison et de la relation équipe → coach ;
la campagne ne possède ni l'un ni l'autre et n'a pas à les posséder. L'agrégat
garde le dernier mot par `valider_proposition`, qui refuse ce qui viole R5, R10
ou R18 quelle qu'en soit la provenance.

**Le générateur est un paramètre.** C'est ce qui rend R8 testable : un `StdRng`
graine fixe donne un tirage reproductible en test, `from_os_rng()` en production.
`random_draw.rs` isole la partie déterministe (`distribute`, testée) du `shuffle`
(non testé) ; ici l'injection fait mieux, en rendant testable l'ensemble.

### L'ordre des objectifs, en clair

1. **R10 refuse** — les paires interdites sont retirées avant tout choix.
2. **R8.1** maximise le nombre de rencontres.
3. **R8.2** minimise le total des rencontres déjà jouées, puis préfère les plus anciennes.
4. **R9** choisit l'exemptée parmi celles qui ne l'ont jamais été, si l'effectif est impair.
5. **R17** départage au sort les combinaisons restées à égalité.

À vingt équipes au plus, l'énumération avec élagage suffit — aucun couplage
maximum pondéré n'est nécessaire, et l'écrire serait payer une complexité pour un
cas qui n'existe pas dans une ligue amateur.

## Les erreurs du domaine

Ajoutées à `DomainError` (`domain/error.rs`), qui n'utilise pas `thiserror` et
implémente `Display` à la main — son message sert de corps de réponse.

```rust
SurveyOnRestDay,                                   // R2
TeamNotInSurvey        { team: String },           // R19
SurveyClosedForCoach,                              // R21
TeamNotOwnedByCoach    { team: String },           // R28
RoundFrozenByReport,                               // R13
NotEnoughPresent       { presents: usize },        // R15
ForbiddenPair          { home: String, away: String },  // R10
TeamNotPresent         { team: String },           // R5, via R22
TeamNoLongerEnrolled   { team: String },           // R18
InconsistentProposal   { motif: &'static str },    // R22
```

`motif` est un `&'static str` et non un `String`, comme `ImmutableTierField` du
même enum : il ne peut venir que du code qui a détecté l'écart, jamais d'une
requête.

## Tests unitaires prévus

Une règle, au moins un test. Ceux du tirage vivent avec `tirer`, les autres avec
l'agrégat.

| Règle | Test |
|---|---|
| R1 | `ouvrir` sur un coach à deux équipes crée **deux** réponses, avec deux jetons distincts |
| R2 | `ouvrir` sur une journée `is_rest()` rend `SurveyOnRestDay` |
| R3 | un destinataire sans adresse entre quand même dans les réponses |
| R5 | `presents()` exclut les `SansReponse` ; `compte_sans_reponse()` les compte |
| R6 | `enregistrer` avec `Organisateur(id)` conserve l'identifiant dans `Presence::Declaree` |
| R8 | **4 équipes, déjà joué A-B A-C B-C → deux rencontres**, dont une revanche |
| R8 | à nombre égal de rencontres, la combinaison au moindre total de revanches gagne |
| R8 | entre deux revanches, la plus ancienne est préférée |
| R9 | l'exemptée est tirée parmi celles qui ne l'ont jamais été |
| R10 | deux équipes d'un même coach ne sont jamais appariées, même au prix d'un match en moins |
| R12 | présent → absent après appariement rend `EnregistreeRencontreARefaire` |
| R16 | absent → présent après appariement le rend aussi |
| R24 | `desaccord` sur une journée vidée au Calendrier rend l'état clos, pas « appariée » |
| R13 | `enregistrer` et `rouvrir` sur journée figée rendent `RoundFrozenByReport` |
| R15 | `peut_tirer` avec un seul présent rend `NotEnoughPresent { presents: 1 }` |
| R17 | deux tirages sur deux graines différentes diffèrent, à effectif suffisant |
| R19 | `enregistrer` sur une équipe hors campagne rend `TeamNotInSurvey` |
| R21 | campagne close : `Coach` et `Jeton` sont refusés, `Organisateur` accepté |
| R28 | `Coach(id)` sur l'équipe d'un autre coach rend `TeamNotOwnedByCoach` ; `Jeton` n'est pas contrôlé |
| R22 | `valider_proposition` refuse une équipe absente, une paire interdite, une équipe en double |
| R23 | `statut` rend `Close(Echeance)` sans `Decidee` ; `rouvrir` avec une échéance passée est refusé |

**Le premier test de R8 est celui qui manquait.** Les six tests actuels de
`generate_round_pairings` portent sur quatre équipes à historique vide ou
presque, et n'ont jamais vu le cas qui casse — mesuré à 54-58 % des tirages en
milieu de saison. C'est lui qui doit exister **avant** la correction, et échouer.

R9, R10 et R17 se testent avec une graine fixe ; R8.1 et R8.2 sont déterministes
et n'en ont pas besoin.

R4, R7, R11, R14, R18 et R20 ne sont pas testées ici : elles vivent dans les use
cases, dans l'unité `reponse-coach`, ou ne s'implémentent pas.
