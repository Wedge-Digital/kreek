# Phase 7 — Effets de bord : persistance, événements, réponses

**Entrée** : `06-domaine.md` validé — l'agrégat, ses neuf méthodes, le tirage
comme fonction pure, les vingt-trois règles.

Cette phase ne code rien. Elle nomme les fichiers, les requêtes, les signatures
et les scénarios, et tranche les quatre points que les phases précédentes
laissaient ouverts ou contradictoires.

## Ce que l'existant impose, et qui a été vérifié

| Fait | Conséquence |
|---|---|
| `PairingCreated` et `PairingDeleted` existent, et le publisher les convertit déjà | aucun événement à créer, aucun listener à câbler |
| Cinq `FakeMatchDayRepo` implémentent `IMatchDayRepository` | `save_pairings` coûte cinq fakes à étendre, pas un |
| `check-arch` axe 14 refuse une feuille CSS absente du bundle | `pages/competition-admin-presences.css` s'inscrit dans `src/web/css_bundle.rs` |
| `notification_delivery_repository.rs` donne la forme du dépôt | erreur maison, `db_err`, `include_str!` sur `sql/…` |
| `schedule_actions.rs` ouvre chaque mutation par `require_admin_access` **puis** `journee_de_la_saison` | les neuf actions font de même — carte 416 |

## 1. Persistance

### La migration

`migrations/20260907000001_competition_presence_surveys.sql` — les deux tables
de `03-back.md` telles quelles. Ce que la base garantit, et que le code n'a donc
pas à arbitrer :

| Contrainte | Règle |
|---|---|
| `UNIQUE (round_id)` sur les campagnes | R2 — un seul sondage vivant par journée |
| `UNIQUE (survey_id, team_id)` sur les réponses | R1 — une réponse par équipe engagée |
| `UNIQUE (token)` | le jeton désigne une réponse et une seule |
| `CHECK` à trois branches sur `(presence, repondu_le, saisi_par_admin)` | la projection à plat de l'enum `Presence` |

Pas de colonne `statut` : R23 la calcule. Pas de table de jetons : R7 leur
refuse toute échéance propre, ils n'ont rien à porter qu'une réponse ne porte
déjà.

**Le `CHECK` n'est pas une ceinture de plus.** Postgres n'a pas de type somme :
il est le seul endroit où l'impossibilité d'une réponse déclarée sans
horodatage s'écrit côté base. C'est la répartition que le projet applique déjà —
*la contrainte garantit, le type exprime*.

### Le port

`domain/presence_survey_repository_port.rs` :

```rust
#[async_trait]
pub trait IPresenceSurveyRepository: Send + Sync {
    async fn find_by_round(&self, round_id: &str)
        -> Result<Option<PresenceSurvey>, PresenceSurveyRepositoryError>;

    async fn save(&self, survey: &PresenceSurvey)
        -> Result<(), PresenceSurveyRepositoryError>;

    async fn list_summaries(&self, season_id: &str)
        -> Result<Vec<SurveySummaryDto>, PresenceSurveyRepositoryError>;
}
```

**Trois méthodes, pas quatre.** `find_by_token` appartient à l'unité
`reponse-coach` et y sera déclarée : c'est elle qui sait ce que la route
publique a besoin de charger, et déclarer ici une méthode qu'aucun appelant
n'utilise ferait porter à cette unité une décision qui n'est pas la sienne. Le
trait se rouvre alors — c'est un ajout, pas une reprise.

`PresenceSurveyRepositoryError` n'a qu'une variante `Database(String)`, comme
`DeliveryError` : le port ne juge rien, les refus sont au domaine.

### `save` réécrit tout, et c'est délibéré

Une seule transaction : `UPSERT` de la campagne, puis un `UPSERT` par réponse.
Changer une présence réécrit les quatorze lignes.

C'est le prix d'un **chemin d'écriture unique**. Un `save_answer` ciblé serait
plus économe de quelques microsecondes et ouvrirait une seconde porte vers la
colonne `presence` — celle-là même que la phase 6 verrouille en faisant
d'`enregistrer` le seul maître d'une `Presence`. R19, R13 et R21 tiennent parce
qu'il n'existe qu'un chemin ; en ouvrir un second les rendrait contournables
par un appelant pressé, et rien ne le signalerait.

Quatorze lignes dans une transaction, pour une action que l'organisateur
déclenche quelques dizaines de fois par saison : la dépense n'existe pas.

### Les requêtes

Toutes sous `sql/presences/`, chargées par `include_str!`. La dette SQL de
`match_day_repository` — des `INSERT` en dur dans le Rust — n'est pas reprise.

| Fichier | Rôle |
|---|---|
| `find_survey_by_round.sql` | la campagne d'une journée |
| `find_answers_by_survey.sql` | ses réponses, pour la réhydratation |
| `upsert_survey.sql` | `ON CONFLICT (id) DO UPDATE` — `deadline`, `auto_remind`, `close_le`, exemptée |
| `upsert_answer.sql` | `ON CONFLICT (survey_id, team_id) DO UPDATE` — les trois colonnes de `Presence` |
| `list_survey_summaries.sql` | **une** requête pour toute la saison |

`list_survey_summaries.sql` joint les journées, leurs campagnes et le compte des
réponses par état, et laisse tomber la ligne de campagne absente — une journée
sans sondage doit apparaître dans la barre latérale. Compter journée par journée
ferait vingt allers-retours pour une colonne.

### Le dépôt reconstruit l'enum, il ne le devine pas

`presence` + `repondu_le` + `saisi_par_admin` redeviennent une `Presence` à la
lecture. Une ligne incohérente — qu'aucun chemin d'écriture ne peut produire et
que le `CHECK` refuse — est traitée comme les `try_new(...).map_err(db_err)?`
du reste du projet : une erreur de dépôt, pas un repli silencieux.

**Pas de `.ok()?` ici.** Le `roster_service` en porte deux, et `CLAUDE.md` les
cite comme le mécanisme qui a fait disparaître un roster sans une ligne de
journal. Une réponse escamotée à la lecture, c'est une équipe qui manque au
tirage sans que personne sache pourquoi.

### `save_pairings` — la méthode plurielle

```rust
async fn save_pairings(
    &self,
    match_day_id: &str,
    pairings: &[(Pairing, NewPairingProjection)],
) -> Result<(), MatchDayRepositoryError>;
```

Transaction, `SELECT … FOR UPDATE` sur la journée, N paires et N projections,
commit. **C'est le verrou qui fait tenir R11 face à la course** : sans lui, deux
organisateurs franchissent tous deux la garde « la journée est-elle vide ? »
avant que l'un ait écrit.

Coût mesuré : **cinq fakes** l'implémentent aujourd'hui —
`match_report_published_listener`, `delete_pairing_use_case`,
`generate_pairings`, `add_match_use_case`, `generate_all_pairings` — soit quatre
lignes chacun. `save_pairing` n'est pas touchée ; seul `generate_pairings`
migre, parce qu'il écrit N appariements et souffre du même défaut.

**Pas de méthode par défaut** qui boucle sur `save_pairing`. Elle compilerait,
les cinq fakes n'auraient rien à changer, et le vrai dépôt resterait non
atomique le jour où quelqu'un oublierait de la redéfinir — un verrou qui se
laisse oublier n'est pas un verrou.

### Tests d'intégration

`io/repository/tests/test_presence_survey_repository.rs`, vraie `PgPool`, sur le
patron de `test_notification_delivery_repository.rs`.

| Test | Ce qu'il prouve |
|---|---|
| une seconde campagne sur la même journée est refusée | R2 tenue **par la base**, pas par le code applicatif |
| une réponse `presente` sans `repondu_le` est refusée | le `CHECK` fait son travail |
| aller-retour des trois états de `Presence`, dont `Organisateur(id)` | R6 survit à la persistance |
| `save` deux fois de suite laisse le même état | l'`UPSERT` est idempotent |
| `list_summaries` rend une ligne pour une journée sans campagne | la barre latérale n'a pas de trou |

## 2. Événements — rien à créer, et c'est le fait notable

Aucun domain event nouveau, aucun app event, aucun listener.

| Use case | Émet |
|---|---|
| `confirm_draw` | un `PairingCreated` par rencontre, **après le commit** |
| `undo_draw` | un `PairingDeleted` par rencontre |
| `repair` | un `PairingDeleted` sur l'ancienne, un `PairingCreated` sur la nouvelle |

Toutes par `emettre()` (`common/services/event_bus/domain_event_publication.rs`).
Jamais de `.send(` direct : `to_enveloppe()` engendre un identifiant, et une
ligne de journal écrite à la main au-dessus reprendrait celui de l'enveloppe
reçue — une trace qui a l'air juste et ne corrèle rien.

**Ce que ça donne, et qui mérite d'être écrit** : le publisher existant convertit
déjà les deux, donc le tirage entré par la porte du sondage produit en aval
exactement les mêmes effets que celui du Calendrier — classement, projections,
rapports de match. Aucun BC n'apprend qu'un sondage existe, et R14 le garantit.

**L'émission vient après le commit.** Un listener qui réagit à un
`PairingCreated` dont la transaction est ensuite annulée aurait travaillé sur un
fait qui n'a pas eu lieu, sans que rien ne le lui dise. L'ordre inverse paraît
plus naturel — « tout dans la même unité » — et c'est le piège.

La campagne, elle, n'émet rien : un domain event de ce BC n'existe que pour être
converti en app event, et personne n'écoute.

## 3. Les handlers — douze signatures

Trois fichiers, calqués sur `schedule_tab.rs` / `schedule_widgets.rs` /
`schedule_actions.rs`.

### La garde, sur les douze

```rust
if let Err(refus) = require_admin_access(&auth_session, &space_id, &competition_id, &season_id, &state).await {
    return refus;
}
if let Err(refus) = journee_de_la_saison(&body.round_id, &season_id, &state).await {
    return refus;
}
```

**Le second contrôle n'est pas redondant** : `space_scope` n'a pas de résolveur
pour `round_id`, qui passe donc librement, dans le chemin comme dans le corps
(carte 416). Et **les fragments l'appliquent aussi** — sans quoi le chemin htmx
du changement d'onglet contournerait le contrôle d'accès.

### Le tableau

| Fichier | Handler | Entrée | Sortie |
|---|---|---|---|
| `presences_tab.rs` | `get_presences` | `Path` | page entière ou fragment selon `veut_la_page_entiere` |
| `presences_widgets.rs` | `get_presence_rounds` | `Path` | `PresenceRoundsTemplate` |
| | `get_presence_panel` | `Path`, `?round_id=` | l'un des six panneaux |
| `presences_actions.rs` | `post_launch` | `LaunchBody` | succès |
| | `post_answer` | `AnswerBody` | succès |
| | `post_remind` | `SurveyIdBody` | succès |
| | `post_close` | `SurveyIdBody` | succès |
| | `post_reopen` | `ReopenBody` | succès |
| | `post_draw` | `SurveyIdBody` | **l'aperçu** |
| | `post_confirm_draw` | `ConfirmDrawBody` | succès |
| | `post_undo_draw` | `SurveyIdBody` | succès |
| | `post_repair` | `RepairBody` | succès |

### Le choix du panneau est une fonction, pas une suite de `if`

```rust
enum Panneau { Aucun, EnCours, Clos, Tirage, Appariee, Defection }

fn etat_du_panneau(survey: Option<&PresenceSurvey>, journee_appariee: bool, maintenant: …) -> Panneau
```

Elle se nourrit de `statut()`, de l'appariement de la journée et de
`rencontre_a_refaire()` — trois questions déjà répondues par le domaine. Le
handler rend la struct correspondante ; il ne rejoue pas la machine à états.

L'écrire en `if` dans le handler l'aurait dupliquée entre le GET du panneau et
les neuf actions qui rendent un refus, et c'est la seconde copie qui aurait
dérivé.

### Ce que rendent les actions — décision A

La phase 2 disait deux choses qui se heurtent : « toutes rendent `HX-Trigger:
presenceChanged` » et « les refus s'affichent dans le panneau ». Rendre le
fragment *et* déclencher le rechargement peint le panneau deux fois ; rendre un
corps vide ne laisse nulle part où poser le refus.

| Cas | Réponse HTTP |
|---|---|
| succès | corps vide + `HX-Trigger: presenceChanged` |
| refus métier (R11, R13, R15, R19, R21, R22) | le fragment de panneau porteur du motif, + `HX-Retarget: #presence-panel` + `HX-Reswap: innerHTML`, **sans** trigger |
| `draw` | le fragment d'aperçu, `HX-Retarget` + `HX-Reswap`, sans trigger |
| corps mal formé, VO invalide | `400` — refus de format, pas refus métier |
| panne | `500` + `tracing::error!` |

Les boutons portent donc `hx-swap="none"` : le succès ne remplace rien, les deux
widgets se rechargent d'eux-mêmes sur `presenceChanged`. C'est le serveur qui
redirige le swap vers le panneau quand il a quelque chose à y dire — mécanisme
htmx standard, pas de JavaScript.

**`draw` ne déclenche pas `presenceChanged`**, et c'est ce qui le rend possible :
l'aperçu ne persiste rien, un rechargement du panneau le perdrait aussitôt.

**Aucune `alert()`, aucun JSON d'erreur** — écart assumé avec le Calendrier, déjà
argumenté en phase 2 : une fois l'explication du tirage dans le panneau, les
refus y vont aussi, faute de quoi il faudrait trancher pour chaque nouveau
message de quel côté il tombe, et cette frontière dérive.

## 4. Templates

| Fichier | Rendu |
|---|---|
| `templates/admin/presences.html` | la page hôte : deux conteneurs `hx-get`, quasi zéro JS |
| `templates/admin/widgets/presences-rounds.html` | la barre latérale |
| `templates/admin/widgets/presences-panel-empty.html` | aucun sondage |
| `templates/admin/widgets/presences-panel-running.html` | en cours **et** clos |
| `templates/admin/widgets/presences-panel-draw.html` | tirage proposé |
| `templates/admin/widgets/presences-panel-paired.html` | journée appariée |
| `templates/admin/widgets/presences-panel-defection.html` | défection à traiter |

Les deux racines de widget portent `hx-disinherit="*"`. La page hôte reprend la
mémoire de `document.body.dataset.activeRoundId` du Calendrier — quatre lignes,
sans lesquelles toute mutation rechargerait un panneau vide.

**L'onglet** dans `admin-page.html`, entre Calendrier et Paramètres, avec
`active_tab == "presences"` et son `hx-push-url`.

**La feuille** `pages/competition-admin-presences.css`, portée
`.competition-admin-presences`, **inscrite dans `src/web/css_bundle.rs`** dans
la section « pages », par ordre alphabétique entre
`competition-admin-groups.css` et `competition-admin-schedule.css`. L'axe 14 de
`check-arch` refuse toute feuille absente du bundle, et aucun gabarit ne porte
de `<link>` : la règle 5 des widgets a été inversée par la carte 342, et c'est
elle qui a supprimé le clignotement.

**Aucun `style="…"`**, aucune valeur d'espacement en dur — les tokens `--p0` à
`--p5` de `common.css`. Le breakpoint est `768px`, comme partout.

## 5. Tests E2E

`tests/e2e/test_competition_presences.py`, pytest + Playwright, sur le patron de
`test_competition_admin_enrollments.py`.

| Scénario | Ce qu'il éprouve |
|---|---|
| lancer une campagne, voir les trois colonnes | R3 — une équipe sans adresse est bien dans « sans réponse » |
| poser une présence à la main, voir le badge | R6 — « saisi par vous » survit au rechargement |
| clore, puis poser encore une réponse | R21 — la clôture ferme le coach, pas l'organisateur |
| tirer avec un seul présent | R15 — le motif s'affiche, le bouton reste inactif |
| tirer, valider, ouvrir le Calendrier | les rencontres y sont, avec leurs projections |
| passer un présent à absent après le tirage | R12 — le panneau de défection propose, il n'agit pas |
| valider la réparation | la rencontre change, les autres ne bougent pas |

**`cliquer_quand_cable` sur tout ce qui vient d'être injecté** (`htmx_helpers.py`).
Le panneau est remplacé à chaque action : c'est exactement la fenêtre où un
bouton est peint, visible, et inerte. Pas de `sleep` — une durée fixe n'a aucune
marge sur une machine chargée, et c'est là que la suite échoue.

**`tests/impact-map.toml` est mis à jour dans la même carte** que le test. La
skill `test-impact` l'exige, et une carte tests↔BC incomplète fait sauter en
silence le test qu'on vient d'écrire.

Côté unitaire, les dix-neuf tests de `06-domaine.md` restent le socle — et le
premier test de R8, celui qui doit échouer avant la correction de
`generate_round_pairings`, se livre **avant** tout ce qui s'appuie dessus.

## 6. Ce que cette phase corrige aux précédentes

### B — `SurveySummaryDto.statut` contredisait R23

La phase 4 le donnait en `String` venu du SQL ; R23 dit que le statut est
calculé, jamais stocké. Le calculer dans `list_survey_summaries.sql` aurait mis
la règle dans une requête — et à deux endroits, puisque `PresenceSurvey::statut`
la porte déjà.

**Forme retenue** : une fonction libre du domaine,

```rust
pub fn statut_de(fermeture: &Fermeture, deadline: &SurveyDeadline, maintenant: …) -> SurveyStatus
```

que `PresenceSurvey::statut()` appelle, et que la barre latérale appelle aussi.
Le DTO porte `deadline` et `close_le` bruts ; `PresenceRoundItemVm::from_domain`
en tire `etat` et `resume`.

Une règle métier calculée à deux endroits finit par l'être de deux façons. Celle
de R23 croise deux champs et une horloge — assez pour diverger.

### R24 — Les rencontres appartiennent à la journée, la campagne ne retient que l'exemption

**Apparue en phase 7**, en instruisant les effets de bord.

`06-domaine.md` donne `Appariement::Fait { rencontres: Vec<PairingId>, exemptee }`.
Or **quatre chemins du Calendrier suppriment des appariements sans rien savoir
d'une campagne** : `delete_pairing_use_case::execute`, `clear_round`,
`clear_season` et `delete_round`. Et R11 fait de l'un d'eux le chemin normal :
*« l'organisateur vide la journée depuis le Calendrier s'il veut retirer »*.

Conséquence, avec la forme de la phase 6 : l'organisateur vide sa journée au
Calendrier, revient en Présences, et l'onglet affiche « journée appariée » avec
quatre rencontres qui n'existent plus — et un bouton « Annuler le tirage » qui
ne défera rien.

Ce n'est pas un défaut d'implémentation à venir : c'est une **donnée dupliquée**.
Les rencontres sont écrites dans `competition_match_day_pairings`, dont la
journée est propriétaire ; les recopier dans la campagne crée une seconde vérité
qu'aucune transaction ne tient ensemble, puisque les deux écritures partent
d'écrans différents.

**Forme retenue** :

```rust
pub enum Appariement { Aucun, Fait { exemptee: Option<TeamId> } }
```

L'exemptée **reste** dans la campagne : elle n'existe nulle part ailleurs, et R9
a besoin de l'historique des exemptions de la saison pour choisir la suivante.
Les rencontres, elles, se lisent sur la journée.

**Conséquence sur `enregistrer`.** R12 doit savoir quelle rencontre est touchée
par une défection ; l'agrégat ne la connaît plus. Le use case la lui fournit —
même patron que `figee: JourneeFigee`, et pour la même raison : *les faits
viennent du dehors, l'agrégat décide*. Les deux faits voyagent ensemble :

```rust
pub struct EtatJournee {
    pub figee:      bool,                    // R13 — un rapport publié
    pub rencontres: Vec<RencontreJournee>,   // (pairing_id, home, away)
}

enregistrer(&mut self, team, venue, par, journee: &EtatJournee, maintenant) -> Result<EffetReponse, DomainError>
```

**Les rencontres entières, pas la seule rencontre touchée.** Un paramètre
`rencontre_de: Option<PairingId>` calculé par le use case aurait sorti du
domaine la question « quelle rencontre est touchée ? », qui est métier. Avec la
liste, l'agrégat la cherche lui-même, et `appariee` n'est plus un champ à part :
c'est `!rencontres.is_empty()`.

`EffetReponse::EnregistreeRencontreARefaire { pairing }` ne change pas.

**Deux conséquences mécaniques, à ne pas manquer :**

`remplacer_rencontre` **disparaît**, absorbée par `marquer_appariee(exemptee)`.
Les rencontres ayant quitté l'agrégat, une réparation ne change chez lui que
l'exemptée — et l'appeler de nouveau enregistre l'exemption qui a *finalement*
eu lieu, ce que R9 demande déjà. Garder les deux noms aurait été deux portes sur
un seul comportement.

`rencontre_a_refaire()` devient **`desaccord(&self, journee: &EtatJournee)`**,
qui croise les présences avec les appariements réels et rend ce qui ne concorde
pas. Le bénéfice dépasse la correction : l'état « défection à traiter » survit
désormais à un rechargement de page, puisqu'il se recalcule au lieu de se lire.
Avec la forme précédente, il tenait à ce que l'agrégat et la journée restent
d'accord — ce que R24 montre impossible.

**Et le panneau devient juste** : « appariée » est un fait de la journée, lu à
chaque affichage. Vider la journée au Calendrier ramène la campagne à l'état
clos sans qu'aucune réconciliation n'ait à tourner — il n'y a plus rien à
réconcilier.

Écarté : **un listener intra-BC sur `PairingDeleted`** qui remettrait la campagne
d'accord. Il marcherait, au prix d'un décalage d'un battement et d'un état à
reconstruire quand il se perd — pour maintenir cohérente une donnée qu'on peut
simplement ne pas dupliquer.

Écarté aussi : **réconcilier à l'affichage**, en croisant `rencontres` et les
appariements réels. C'est faire dériver une valeur métier dans la couche de
présentation, ce que `CLAUDE.md` interdit nommément — et le défaut serait resté,
en plus de coûter une écriture sur un GET.

### C — `find_by_token` est reporté en unité 2

Le port n'en porte pas. C'est l'unité `reponse-coach` qui sait ce que sa route
publique charge, et le trait se rouvrira alors par un ajout.
