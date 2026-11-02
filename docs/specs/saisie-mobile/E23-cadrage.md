# E23 — Le temps réel : connexions, présence et notifications — cadrage

Fiche technique de l'épic `kanban/epics/ready_to_be_done/E23-temps-reel-connexions-et-notifications.md`,
**brouillon du 2026-11-02, décisions tranchées le 2026-11-02 au § 11**. L'épic demande une
fiche avant les cartes : ce n'est pas un écran, c'est un canal. Elle part de ce
que le code fait, puis pose les règles, les événements, l'architecture du canal,
les routes, nginx, les conséquences, les tests, le découpage et les risques. Les
décisions déjà prises dans l'épic ne sont pas rediscutées : notifications
éphémères, destinataires déduits du domaine et présence qui ne décide que de la
livraison, registre en mémoire avec relais `LISTEN/NOTIFY` seulement à
plusieurs instances, pas de Redis, app events uniquement par le publisher de
`match_report`, aucun accès du canal à une table de BC.

## 1. Ce que le code fait aujourd'hui

Les références sont prises sur `22a91a4d`.

**Le bus**

- `EventBus` est un `tokio::sync::broadcast::Sender<EventEnvelope>` de
  **capacité 256** (`src/common/services/event_bus/event_bus.rs:4-8`). Deux
  instances existent, créées dans `compose` (`src/main.rs:289-290`) :
  `event_bus`, **un seul bus « interne » partagé par tous les BCs**, et
  `app_event_bus`.
- Un abonné en retard de plus de 256 messages reçoit `RecvError::Lagged(n)` :
  les messages sont **perdus** pour lui, sans rejeu. Les boucles
  d'écoute (35 fichiers) le traitent toutes par un `warn!` et continuent (par exemple
  `src/app/match_report/io/app_events/app_event_publisher.rs:53-55`).
- `emettre()` (`domain_event_publication.rs:46-55`) et `publier()`
  (`app_event_publication.rs:52-61`) journalisent `event` et `event_id` en
  `info`, puis font un `send` dont l'`Err` (« aucun abonné ») est ignoré.
- `spawn_listener` (`supervision.rs:35-51`) rend bruyante la mort d'un
  listener ; il ne le relance pas.
- `EventEnvelope` (`src/common/event_envelope.rs:4-12`) : `event_id`,
  `emitter`, `event_type`, `tags`, `payload`, `occurred_at`. Pour un app event
  de `match_report`, `emitter` vaut l'identifiant du rapport
  (`match_report_app_events.rs:212-231`).
- **`event_log_feeder` persiste chaque domain event émis sur `event_bus`** dans
  la table `event_log` (`src/common/event_listener/event_log_feeder.rs:7-24`,
  `src/common/persistance/event_log_repository.rs:53`, branché `main.rs:292`).
  La table n'a aucun lecteur en production (`find_by_tag` n'est appelé nulle
  part).

**Ce qui sort de `match_report` aujourd'hui**

`MatchReportAppEvent` (`src/app/shared_kernel/app_events/match_report_app_events.rs:7-91`)
compte sept variants : `MatchReportConfirmed`, `MatchReportCancelled`,
`JourneymenFielded`, `JourneymenWithdrawn`, `MatchReportPublished`,
`MatchReportUnpublished`, `MatchReportRoundReassigned`. Aucun ne porte d'auteur.
`MatchReportConfirmed` porte déjà `space_id`, `home_team_id`, `away_team_id`.

Le publisher (`io/app_events/app_event_publisher.rs:94-169`) aiguille sans
joker ; le bras groupé `=> {}` (`:150-167`) nomme `MatchReportCreated`,
`SelectionUpdated`, `EntryModeChosen`, `FanFactorRecorded`,
`TeamValuesRecorded`, `InducementsRecorded`, `InducementsCancelled`,
`StarPlayerEngaged`, `ActionRecorded`, `ActionDeleted`, `MvpDrawn`,
`PostMatchRecorded`. Pour `MatchReportConfirmed`, il **relit l'agrégat entier**
(`handle_confirmed`, `:375-387`) ; ses dépendances sont `repo`,
`competition_data`, `team_data` — pas `coach_data` (`context.rs:32-49`).

**Les faits de l'épic, face au code** (chemins relatifs à `src/app/match_report/`)

| Fait de l'épic | Domain event | Auteur | Émis sur le bus aujourd'hui ? |
|---|---|---|---|
| démarrage | `SelectionConfirmed` + `EntryModeChosen` (`domain/events.rs:36-48`) | `confirmed_by`, `started_by` | **oui** (`create_match_report_use_case.rs:158-167`, `update_match_selection_use_case.rs:138-143`) |
| action | `ActionRecorded` (`events.rs:143-153`) | `recorded_by` | **non** : `record_action_use_case.rs:132` appende sans émettre |
| action retirée | `ActionDeleted` (`events.rs:154-158`) | `deleted_by` | **non** (`delete_action_use_case.rs:56`) |
| MVP saisi | `ActionRecorded` d'action `Mvp` | `recorded_by` | **non** (`record_mvp_use_case.rs:85`) |
| MVP tiré | `ActionRecorded` + `MvpDrawn` (`events.rs:165-169`) | sur l'`ActionRecorded` seul ; `MvpDrawn` n'en porte pas | **non** (`draw_mvp_use_case.rs:52`) |
| sélection des coups de pouce validée | `InducementsRecorded` (`events.rs:106-110`), un par camp | `recorded_by` | **non** : `record_inducements_use_case.rs:452` appende ; le bus ne sert qu'à `init_temp_players` (`:455-465`) |
| coups de pouce annulés | `InducementsCancelled` (`events.rs:114-116`) | `cancelled_by` | **oui** (`cancel_inducements_use_case.rs:96-100`) |
| camp validé | **aucun** | — | n'existe pas : c'est E22 (« Valider son camp ») |

Les noms se résolvent sans port nouveau : `ICoachDataPort::find_coach_name`
(`ports.rs:211-212`) pour l'auteur, `ITeamDataPort::find_team_info`
(`ports.rs:143`) pour le nom d'équipe ; `ActionRecorded` porte
`player_display_name`.

**Les listeners des autres BCs**

Treize fichiers de production hors du BC lisent `MatchReportAppEvent` (`ranking`, `competitions`,
`teams`, `players`). **Aucun ne fait de `match` exhaustif** : tous filtrent par
`let … else { continue }` ou par un joker — `teams/io/app_events/journeymen_fielded_listener.rs:77`
(`_ => continue`), `players/io/app_events/journeyman_departure_listener.rs:85-87`,
`teams/io/app_events/match_report_confirmed_listener.rs:20-27`. Ajouter des
variants ne casse donc **aucun** listener ; seuls `event_type()` et
`to_enveloppe()` (`match_report_app_events.rs:200-240`) et l'aiguillage du
publisher sont exhaustifs.

**La résolution des destinataires de E02** (carte 337)

Elle vit dans `competitions` : `use_cases/notification_recipients.rs:88`
(`resolve`), sur `ICompetitionSpaceMemberPort::list_space_members`
(`competitions/ports.rs:92`, adapter `infrastructure/competitions/space_member_adapter.rs:37`).
Elle répond à une autre question — invités, inscrits, membres d'une saison,
avec adresse électronique — et rend des `Recipient` porteurs d'e-mail. **Elle
n'est pas réutilisable** pour un match : rien n'y dit « les coachs de ces deux
équipes ». Ce qui en est réutilisable, c'est le principe (la borne par
l'espace tenue par le chemin de données) et le patron port + adapter.

Côté match, la seule question existante est `ITeamDataPort::is_coach_of_team`
(`ports.rs:141`, adapter `infrastructure/match_report/ref_team_data_adapter.rs:59-69`),
qui compare `team.coach_id`. **Aucun port ne rend l'identifiant du coach d'une
équipe** : `TeamInfoDto` porte `coach_name`, pas `coach_id` (`ports.rs:121-128`).

**Le toast**

- `window.kreekToast(message, type)` (`src/web/templates/app-layout.html:96-115`),
  en JS nu dans le layout, zone `.toast-zone` (`:76`). Succès retiré après
  2,5 s (`:113`), erreur jusqu'au clic. Texte posé par `textContent`.
- **Le type `warning` n'existe pas en JS** : tout ce qui n'est pas `error`
  devient `success` (`:98-100`), alors que `toast.css:43` définit
  `.toast--warning`. `competitions/io/web/templates/admin/schedule.html:93`
  appelle `kreekToast(m, 'warning')` et obtient un toast vert.
- L'écouteur `showToast` (`app-layout.html:120-123`) sert les trois
  `HX-Trigger` de `team_creation` (`finalize_team.rs:221`, `:381`,
  `build_team/submit_team.rs:128`) et celui de `competitions`
  (`move_pairing_widget.rs:273`). `actions-step.html:40` a son propre
  `showToast` Alpine, sans rapport.

**L'authentification d'une requête longue**

- Toute route `/app/…` passe par `require_auth` (`main.rs:678`), qui renvoie un
  `303` vers la connexion à une requête sans session
  (`src/web/middleware/require_auth.rs:22-29`). Un `EventSource` qui reçoit une
  page HTML échoue **définitivement** : il ne se reconnecte pas.
- `bypass_auth` ne connecte que sur une session vide, et ne connaît que deux
  identités, `DevCoach` et `E2E Coach 01` (`bypass_auth.rs:36-51`).
- **Il n'existe pas de middleware CSRF** : aucun fichier ne le définit, malgré
  `CLAUDE.md` et le commentaire de `main.rs:656`. La garde contre les requêtes
  inter-sites est le `SameSite=Lax` du cookie (`main.rs:658-660`). Une route
  `GET` de flux n'est concernée par aucun des deux.
- Sessions en mémoire, `DashMapStore` (`src/common/session_store.rs`) ; la
  carte 491, qui les persiste, est **en `ready_to_be_done/`**, pas faite.
- `request_log` (`src/web/middleware/request_log.rs:47-77`) mesure la durée
  jusqu'au retour de `next.run` : pour un flux, la ligne « réponse envoyée »
  part **à l'ouverture**, et le span `req` ne couvre pas la vie du corps.

**Le client et le serveur SSE**

- htmx **2.0.4** (`assets/static/js/htmx.min.js`), sans extension SSE : seul
  `htmx-ext-json-enc.js` est livré. `EventSource` natif suffit.
- axum 0.8.9 (`Cargo.lock:197-198`) expose `axum::response::sse::{Sse, Event, KeepAlive}`
  sans feature supplémentaire ; `Sse` pose `content-type: text/event-stream` et
  `cache-control: no-cache`, **pas** `X-Accel-Buffering` ; `KeepAlive::default()`
  envoie un commentaire toutes les **15 s**.
- `futures-util` est une dépendance directe (`Cargo.toml`) ; `tokio-stream`
  n'est présent que transitivement (par `sqlx-core`). `dashmap 6` est direct.
- **En build de développement, chaque onglet a déjà un `EventSource`** :
  `tower-livereload` (`main.rs:724-738`) recharge par SSE. `make e2e-server`
  tourne sous `cargo watch -x run`, donc en debug (`Makefile:112-114`).

**Les pages de match**

- `/live` (`routes.rs:13`, `garde_suivi`, `router.rs:95-103`) rend le score
  **dans la page** (`live-match.html:19-40`) et le journal par un widget
  (`:54-59`).
- **Le suivi interroge déjà le serveur** : pour qui ne peut pas écrire, le
  journal se recharge `every 15s` (`live-match.html:56`, carte 636).
- Le journal de saisie (`actions-step.html:119-121`) se recharge sur
  `actionRecorded`/`actionDeleted` locaux.
- Aucun écran « l'outsider attend le favori » ni « validations des camps »
  n'existe : en « Je saisis tout », un seul coach saisit tout ; l'attente est
  refusée par la règle de la carte 627, pas affichée.

**Le routeur et les délais**

Aucun `TimeoutLayer`, aucun `TraceLayer`, aucune compression
(`main.rs:632-741`). `server.request_timeout_ms` est déclaré
(`src/config.rs:33`, `config/default.toml:4`) et **lu nulle part**. Pas de
`with_graceful_shutdown` (`main.rs:774`). Rien ne couperait donc une réponse
longue côté application.

## 2. Les règles

| # | Règle |
|---|---|
| R1 | Un **flux par onglet** ; un utilisateur a zéro, une ou plusieurs connexions. |
| R2 | Les **destinataires** d'une notification de match sont les coachs des deux équipes, moins l'auteur du fait. La présence ne fait que décider qui, parmi eux, la reçoit maintenant. |
| R3 | Une notification **non livrée est perdue** : rien n'est gardé, rien n'est rejoué à la reconnexion. |
| R4 | Le **direct** est un signal (« le match X a changé ») ; la page recharge son widget par son `hx-get`. Le canal ne rend aucun HTML de BC et ne contourne aucune garde. |
| R5 | Un **toast** porte son texte : titre, sous-titre, lien. |
| R6 | S'abonner au direct d'un match exige le **droit de le suivre** (`is_authorized`, admins et coachs des deux équipes). |
| R7 | Le canal **ne lit aucune table de BC** : les faits arrivent par l'`app_event_bus`, le reste par des ports. |
| R8 | Le canal ne **bloque jamais** le bus : un client lent perd ses messages, il ne ralentit pas les autres. |

## 3. Les événements

Aucun domain event n'est ajouté. **Les domain events existants doivent être
émis** (§ 1) : la carte 3 ajoute `emettre()` après l'append dans
`record_action`, `delete_action`, `record_mvp`, `draw_mvp` et
`record_inducements`, comme `cancel_inducements` le fait déjà. Les contrôleurs
leur passent `state.event_bus`.

Les app events proposés, non persistés (pas de `serde(default)`), tous porteurs
de `space_id`, `home_team_id`, `away_team_id` pour le routage :

| App event | Depuis | Contenu propre | Toast | Signal |
|---|---|---|---|---|
| `MatchReportConfirmed` **enrichi** | `SelectionConfirmed` | `confirmed_by`, `confirmed_by_name` | « Saisie du match démarrée par Morgane » | oui |
| `MatchActionRecorded` | `ActionRecorded` | `team_side`, `team_name`, `turn`, `action: ActionTypePayload` (déjà partagé, `:162-171`), `player_display_name`, `recorded_by`, `recorded_by_name` | « Touchdown : Griff #9 (London Rats) — par Morgane » ; MVP compris | oui |
| `MatchActionDeleted` | `ActionDeleted` | `team_side`, `deleted_by` | non (§ 11.7) | oui |
| `MatchInducementsRecorded` | `InducementsRecorded` | `team_id`, `team_name`, `recorded_by`, `recorded_by_name`, `items: Vec<{ label, qty }>` | « London Rats : Relance d'équipe ×2, Pot-de-vin — validé par Morgane » ; sélection vide : « London Rats : aucun coup de pouce — validé par Morgane » | oui |
| `MatchInducementsCancelled` | `InducementsCancelled` | `cancelled_by`, `cancelled_by_name` | « Coups de pouce annulés par Morgane » | oui |

La liste du toast des coups de pouce (décision du 2026-11-02) : le publisher
résout les libellés de `purchases` par `IReferenceCatalogPort`, comme
`inducements_recap_service.rs` le fait déjà pour le récapitulatif, et les
quantités viennent de l'événement. Aucun événement nouveau : le panier n'est
pas enregistré avant validation, et un toast par clic serait du bruit pour
l'adversaire.

**La forme des phrases** (carte 645, arbitrée le 2026-11-02). Le joueur arrive
tel que `match_report` l'affiche, `player_display_name` (« Griff #9 ») : en
tirer le seul numéro pour écrire « le 9 des London Rats » serait recomposer une
valeur, et « du », « des », « saisi(e) » demanderaient des élisions et des
accords selon l'action. D'où la forme à deux-points, et un libellé de coup de
pouce suivi de `×N` au-delà d'un, sans pluriel fabriqué. Un nom absent (les
`*_name` sont des `Option`) retire sa clause — « Saisie du match démarrée » —,
il n'est jamais remplacé. Le sous-titre de tout toast de match est « Voir le
match », qui porte le lien vers `/live`.

`MvpDrawn` reste dans le bras groupé : l'`ActionRecorded` appendé avec lui
porte l'auteur et produit déjà la notification (§ 11.6). Les variants existants
(`MatchReportPublished`, `MatchReportCancelled`…) déclenchent le signal sans
changer. Le routage relit la portée du rapport par la projection
(`find_access_scope`, `find_space_id`, `domain/match_report_repository_port.rs:71-85`)
plutôt que par la relecture de l'agrégat de `handle_confirmed`, qui rejoue le
flux à chaque action.

## 4. Le canal

**Où il vit.** `src/web/realtime/` : registre, contrôleur de flux, contrôleur
de présence, routeur de notifications, mise en forme des toasts. Ses besoins de
BC passent par `src/web/ports.rs`, implémentés dans `src/infrastructure/web/` —
le précédent exact d'`IHorsCalendrierPort` (`src/web/ports.rs:1-14`,
`infrastructure/web/hors_calendrier_adapter.rs`). Deux ports :

```rust
/// Les coachs que ce match concerne : celui de chaque équipe, si l'équipe est bien de l'espace.
async fn coaches_of_match(&self, space_id: &str, home_team_id: &str, away_team_id: &str) -> Vec<String>;
/// Ce que cet utilisateur peut faire sur ce match : rien, le suivre, le saisir.
async fn match_role(&self, user: &User, space_id: &str, match_report_id: &str) -> MatchRole;
```

Le premier s'adosse à `teams` ; le second à `is_authorized` et
`can_write_report` (`match_report_access_service.rs:45-55`, `:114-128`), sans
les réécrire.

**Ce que check-arch en dit.** L'axe 3 ne parcourt que `src/app/` : `src/web/`
et `src/infrastructure/` n'y sont pas soumis, et l'adapter est le seul à
importer les BCs. L'axe 9 ne concerne pas le canal tant qu'il n'est ni dans
`auth` ni dans `spaces`. **L'axe 12** refuse tout `.send(` hors
`emettre`/`publier` : les envois vers les connexions passent par `try_send`
(non visé) ou portent `// arch:ok envoi à une connexion SSE`. **L'axe 13** : pas
de `target:` explicite, la cible par défaut est le chemin du module, donc
`kreek::web::realtime::…`. **L'axe 4** ne voit pas `new EventSource(` ; la règle
vaut quand même, l'URL vient de `app_routes.web.realtime_stream()`. **L'axe 8**
exige les nouveaux tests e2e dans `tests/impact-map.toml`. **L'axe 14**, la
feuille du toast enrichi dans `css_bundle.rs` si elle est nouvelle.

**Le registre.** Une structure `Arc<ConnectionRegistry>` dans `AppState`,
sur `DashMap<ConnectionId, Connection>` plus un index utilisateur → connexions.
Une connexion : `user_id`, `coach_name` (pris à l'ouverture sur `AuthSession`),
un `mpsc::Sender<RealtimeMessage>` borné (32), et sa présence
`Option<(match_report_id, MatchRole)>`. Aucune donnée n'y est écrite ailleurs
qu'en mémoire.

**Le flux.** `GET` authentifié. À l'ouverture : enregistrement, premier
événement `hello` portant l'identifiant de connexion, puis les messages du
canal, `KeepAlive` à 15 s et `retry: 5000`. Le stream est construit sur le
récepteur `mpsc` et porte une garde dont le `Drop` retire la connexion et
journalise la fermeture : c'est le seul nettoyage, et il ne dépend d'aucun
geste du client. L'en-tête `X-Accel-Buffering: no` est posé par l'application
(§ 6).

**Le client.** Dans `app-layout.html`, à côté du toast et en JS nu pour la
même raison (Alpine est différé) : un `EventSource` par onglet, qui traduit
chaque message en événement DOM sur `body` (règle 2 des widgets) —
`toast` → `kreekToast`, `match` → `htmx.trigger(document.body, 'matchChanged', {id})`,
`presence` → `presenceChanged`. Le flux survit aux navigations HTMX (le menu
remplace `#app-content` par `hx-select`, `app-menu.html:9-10`) et se rouvre à
chaque chargement complet. À la reconnexion (`open` après `error`), le client
émet `realtimeResync` : les widgets abonnés se rechargent, ce qui couvre les
signaux perdus pendant la coupure.

**Le routage.** Un listener `spawn_listener` sur l'`app_event_bus`
(`init(app_event_bus: &EventBus, …)`, la convention cross-BC de l'axe 5).
Pour un `MatchReportAppEvent` :

1. **signal** à toute connexion présente sur ce match, auteur compris ;
2. **toast**, si le variant en porte un : `coaches_of_match` moins l'auteur,
   croisés avec le registre, livrés à toutes leurs connexions ;
3. sur `Lagged`, `warn!` et `realtimeResync` envoyé à toutes les connexions
   présentes sur un match.

Le texte du toast et son lien (`AppRoutes::default().match_report.live(…)`,
`match_report/routes.rs:123`) sont composés dans `src/web/realtime/`, depuis les
faits de l'app event : le BC dit ce qui s'est passé, l'hôte dit comment
l'annoncer.

**La présence.** Le flux étant ouvert une fois par onglet, l'abonnement à un
match ne peut pas être un paramètre du flux. Proposé : la page de match pose un
marqueur (`<div class="realtime-presence" data-match="…">`), et le script du
layout, après chaque `htmx:afterSettle` de `#app-content` et au chargement,
envoie `POST presence` avec l'identifiant de connexion et le match — ou sans
match si le marqueur a disparu. Le serveur demande `match_role` au port :
`Aucun` refuse (403), sinon il enregistre le rôle **qu'il a calculé**, jamais
celui que la page prétend. « Morgane saisit » est alors : les connexions
présentes sur ce match dont le rôle est `Saisie`, autres que soi. Un widget du
canal (`GET` présence, gabarit dans `src/web/templates/`) l'affiche et se
recharge sur `presenceChanged` ; il ne lit que le registre.

**Plusieurs instances, plus tard.** Le registre est local au processus. Le jour
où deux binaires tournent, le routeur publie chaque message routé sur un canal
Postgres (`pg_notify`, charge utile < 8 000 octets) et chaque instance livre à
ses propres connexions ; le registre ne change pas d'interface. Rien de cela
n'est construit dans E23.

**Observabilité.** Cible `kreek::web::realtime`, niveau `info` pour ce qui
doit exister en production :

| Moment | Ligne | Champs |
|---|---|---|
| ouverture | `flux ouvert` | `conn`, `coach` (identifiant), `ouvertes=N` |
| fermeture | `flux fermé` | `conn`, `duree_s`, `livres`, `perdus`, `ouvertes=N` |
| routage d'un app event | `notification routée` | `event_id`, `match_report_id`, `destinataires`, `connectes`, `livres` |
| présence | `présence` (`debug`) | `conn`, `match_report_id`, `role` |
| file pleine, `Lagged` | `warn` | `conn` ou `n` |

`event_id` relie la ligne au `publier()` du publisher : un `grep` suffit à
suivre un fait jusqu'aux onglets qui l'ont reçu. Le texte du toast et les noms
de joueurs ne sont pas journalisés. `ouvertes=N` sur chaque ligne d'ouverture et
de fermeture rend une fuite visible sans métrique.

## 5. Les routes

| Route | Garde | Rôle |
|---|---|---|
| `GET /app/realtime/stream` | `require_auth` | le flux de l'onglet |
| `POST /app/realtime/presence` | `require_auth` + `match_role` | déclarer ou retirer sa présence |
| `GET /app/{space_id}/realtime/matches/{match_report_id}/presence` | `space_scope` + `match_role` | le widget « qui suit, qui saisit » |

Les constantes vivent dans `src/web/routes.rs` (`path`), exposées par
`Routes` du web. Les widgets de `match_report` ajoutent
`matchChanged[detail.id=='{{ id }}'] from:body, realtimeResync from:body` à leur
`hx-trigger`.

## 6. nginx en production

**Ce que la configuration actuelle produit pour un flux** (bloc `server` de
`bloodbowlclub.com`, `location /` vers `127.0.0.1:3210`) :

- **`proxy_buffering` est actif par défaut** : nginx retient la réponse
  jusqu'à remplir son tampon (4 ou 8 Kio). Les messages, de quelques dizaines
  d'octets, n'arriveraient qu'en paquets, ou jamais.
- **`proxy_read_timeout` vaut 60 s par défaut** : sans octet reçu de l'amont
  pendant 60 s, nginx coupe. Le battement de 15 s le tient ; un battement plus
  lent que 60 s produirait une coupure et une reconnexion en boucle.
- **`Connection "upgrade"` est forcé sur toutes les requêtes**, websocket ou
  non (l'application n'en a aucun) : l'amont reçoit `Connection: upgrade` sans
  `Upgrade`. Hyper le traite comme une requête ordinaire ; c'est sans effet sur
  le flux, mais interdit toute connexion persistante vers l'amont.
- **`gzip`** : la valeur par défaut de `gzip_types` ne couvre que `text/html`.
  Si la configuration globale l'étend à `text/event-stream` ou à `*`, la
  compression tamponne et le flux se tait.
- **`listen 443 ssl;` sans `http2`** : le navigateur parle HTTP/1.1, limité à
  **six connexions par origine**, tous onglets confondus. Six onglets ouverts
  tiennent six flux, et la septième requête — n'importe quel clic — attend.

**L'en-tête `X-Accel-Buffering: no`**, envoyé par l'application, désactive le
tampon de nginx pour cette seule réponse, sans toucher à la configuration. Il ne
règle ni le délai d'inactivité, ni `gzip`, ni `http2`.

**Le bloc proposé**, dans le même `server`. Une `location =` exacte l'emporte
sur `location /` quel que soit l'ordre ; la placer avant n'est qu'une question
de lisibilité.

```nginx
# Flux SSE (épic E23) : connexion longue, sans tampon ni compression.
location = /app/realtime/stream {
    include proxy_params;
    proxy_pass http://127.0.0.1:3210;
    proxy_http_version 1.1;
    proxy_set_header Connection "";
    proxy_buffering off;
    proxy_cache off;
    gzip off;
    proxy_read_timeout 300s;
    proxy_send_timeout 300s;
}
```

Et, sur la ligne `listen` gérée par Certbot : `listen 443 ssl http2;` avant
nginx 1.25.1, ou la directive `http2 on;` à partir de cette version.

**À vérifier sur le serveur avant la livraison** :

- `nginx -v` — décide de la syntaxe `http2` ;
- `nginx -T | grep -E 'gzip|proxy_buffering|proxy_read_timeout|worker_connections|worker_shutdown_timeout'`
  — ce que la configuration globale impose déjà ;
- `worker_connections` (768 dans le `nginx.conf` de Debian) : chaque flux tient
  **deux** connexions, client et amont ;
- `nginx -t && systemctl reload nginx` : au rechargement, les anciens
  processus gardent leurs flux ouverts jusqu'à leur fermeture ;
  `worker_shutdown_timeout 30s` borne cette attente ;
- l'absence d'un autre mandataire devant nginx (CDN, répartiteur) qui aurait
  son propre tampon.

## 7. Conséquences sur les données de production

| Carte | Données | Effet |
|---|---|---|
| 1 | aucune | une connexion longue par onglet authentifié ; nginx à configurer avant |
| 2 | aucune | présence en mémoire |
| 3 | **la table `event_log` reçoit davantage de lignes** : chaque action, retrait, MVP et achat de coups de pouce, désormais émis, y est écrit par `event_log_feeder` | quelques dizaines de lignes par match. L'event store et les projections ne changent pas. Les app events ne sont pas persistés |
| 4 | aucune | toasts éphémères ; les toasts existants inchangés |
| 5 | aucune | le `every 15s` de `/live` disparaît (§ 11.8) |
| 6 | aucune | — |

**Ce que l'épic disait « aucune donnée persistante »** reste vrai du canal ;
c'est l'émission des faits de saisie (carte 3) qui écrit dans `event_log`. Rien
n'est perdu ni réécrit.

**Effets d'exploitation** : un redéploiement coupe tous les flux. Tant que la
carte 491 n'est pas livrée, la session est perdue, la reconnexion reçoit un
`303` vers la connexion et l'`EventSource` s'arrête — pas de tempête de
reconnexions. Après 491, il se reconnecte en 5 s avec son cookie, et le
`realtimeResync` recharge les widgets. Si `with_graceful_shutdown` est un jour
ajouté, les flux devront être fermés par le signal d'arrêt, sans quoi
l'extinction attendrait leur fin.

## 8. Les tests

**Unitaires**

- Registre : ouverture, fermeture au `Drop`, plusieurs onglets d'un même
  utilisateur, présence posée puis remplacée, livraison aux seuls connectés,
  file pleine qui perd sans bloquer.
- Routage, sur des doublures des deux ports : l'auteur ne reçoit pas son
  toast ; le coach de l'autre équipe le reçoit sur tous ses onglets ; un coach
  d'un autre match ne reçoit rien ; trois matchs simultanés restent étanches ;
  le signal ne va qu'aux présents ; `Lagged` produit un `realtimeResync`.
- Mise en forme : un toast par variant, lien vers `/live`.
- Publisher : chaque nouveau bras produit son app event avec auteur et nom.
- Use cases de la carte 3 : l'événement est émis après l'append, et pas sur un
  conflit (le patron de `cancel_inducements_use_case.rs:155-215`).
- Flux par le harnais (`src/web/test_harness.rs`) : `content-type`,
  `cache-control`, `X-Accel-Buffering`, et le premier événement `hello`.

**E2E**

- **Plusieurs coachs sans nouvelle identité de contournement.** Le serveur e2e
  ne connaît que `DevCoach` et `E2E Coach 01` par en-tête
  (`test_match_report_solo_entry.py:307-308`, `test_match_report_access.py:43`).
  Mais `bypass_auth` ne s'applique qu'aux routes protégées et qu'à une session
  vide : un contexte qui passe d'abord par `/auth/login` avec `E2E Coach 02`
  et `SEED_PASSWORD` (`src/cli/seed_e2e.rs:37`, onze coachs supplémentaires
  `:47`) garde cette identité. `build_full_competition` distribue les équipes
  par ordre de nom de coach (`competition_lifecycle.py:583-592`) : six équipes
  donnent DevCoach et E2E Coach 01 à 05.
- **Trois matchs en parallèle** : quatre contextes au moins, pour qu'un coach
  qui ne joue pas un match prouve qu'il ne reçoit rien. Le test négatif ne vaut
  qu'après un témoin positif : attendre le toast chez le destinataire, puis
  constater son absence ailleurs.
- **Ce qui rend les e2e SSE délicats** : `networkidle` n'arrive jamais avec un
  flux ouvert (la suite l'évite déjà, `test_match_report_step5.py:316`) ; le
  toast de succès disparaît en 2,5 s ; et en debug, chaque onglet tient déjà le
  flux de rechargement à chaud, ce qui rapproche de la limite de six connexions
  HTTP/1.1 — à vérifier par contexte Playwright.
- **La sortie du registre** ne se voit pas directement : elle se constate par
  le widget de présence de l'autre coach, qui perd « suit le match » quand
  l'onglet se ferme.

## 9. Le découpage revu

Numéros attribués le 2026-11-02 : 1 → **642**, 3 → **643**, 2 → **644**,
4 → **645**, 5 → **646**, 6 → **647**.

| # | Carte | Périmètre | Fichiers | Dépend de |
|---|---|---|---|---|
| 1 | Le canal SSE et le registre | flux, registre, client du layout, nginx, journal | `src/web/realtime/` (nouveau), `src/web/routes.rs`, `src/web/router.rs`, `src/state.rs`, `src/main.rs`, `app-layout.html`, `Cargo.toml` si `tokio-stream` | — |
| 2 | La présence sur un match | `POST` présence, port `match_role`, widget, marqueur dans les pages de match | `src/web/realtime/`, `src/web/ports.rs`, `src/infrastructure/web/`, `live-match.html`, `actions-step.html`, `mvp-step.html` | 1 |
| 3 | Les faits de saisie sortent du BC | `emettre()` dans cinq use cases, variants d'app events, bras du publisher, `coach_data` et `catalog` injectés | `use_cases/record_action…`, `delete_action…`, `record_mvp…`, `draw_mvp…`, `record_inducements…`, leurs contrôleurs, `match_report_app_events.rs`, `app_event_publisher.rs`, `context.rs`, `main.rs` | — |
| 4 | Le routage des notifications | listener, port `coaches_of_match`, mise en forme, toast à sous-titre et lien | `src/web/realtime/`, `src/web/ports.rs`, `src/infrastructure/web/`, `app-layout.html`, `components/toast.css` | 1, 3 |
| 5 | Le direct sur la page d'un match | signal, `hx-trigger` des widgets, score en widget, fin du `every 15s`, resynchronisation | `live-match.html`, `action-log-widget`, contrôleur de `/live`, `src/web/realtime/` | 1, 2, 3 |
| 6 | Les tests du temps réel | le scénario du « Terminé quand » | `tests/e2e/`, `tests/impact-map.toml` | 4, 5 |

**Ce qui devrait bouger, sans que cette fiche le change** :

- **La carte 3 ne dépend pas du canal** : elle peut passer en premier, en
  parallèle de la 1.
- **« camp validé » n'existe pas** : il appartient à E22, qui vient après E23
  et branchera son propre app event sur le canal. Idem pour « l'attente de
  l'outsider » et « l'état des validations » de la carte 5 : en « Je saisis
  tout », il n'y a ni attente affichée ni validation par camp. Le direct de E23
  se réduit au journal et au score de `/live`.
- **Le score de `/live` est rendu dans la page** : le direct oblige à en faire
  un widget, ou à le joindre au journal.
- **La carte 6 seule pour les tests** heurte « Couverture obligatoire » de
  `CLAUDE.md` : chaque carte porte ses tests unitaires et e2e ; la 6 garde le
  scénario des trois matchs, comme la 639 pour E21.

## 10. Les risques

- **`Lagged`** : le routeur partage la capacité 256 de l'`app_event_bus`.
  S'il attend ses ports à chaque événement pendant une rafale (une publication
  émet `MatchReportPublished` plus un événement par joueur), il peut décrocher.
  Effet : toasts perdus (accepté), signaux perdus (rattrapés par
  `realtimeResync`). Les ports doivent rester légers ; le routeur ne fait aucun
  appel par connexion.
- **Fuite de connexions** : le nettoyage repose sur le `Drop` du stream. Un
  client disparu sans fermer TCP n'est détecté qu'à l'échec d'une écriture — le
  battement de 15 s en donne l'occasion, la pile TCP peut mettre plus
  longtemps. `ouvertes=N` au journal le montre.
- **Redéploiement et sessions en mémoire** : § 7. La carte 491 change le
  comportement de reconnexion ; les deux cartes ne doivent pas l'ignorer.
- **Onglets multiples** : un toast par onglet ouvert, et la limite de six
  connexions HTTP/1.1 tant que `http2` n'est pas activé. En développement, le
  flux de `tower-livereload` s'ajoute au nôtre : trois onglets sur le même port
  en saturent la moitié.
- **Déconnexion** : un flux ouvert garde l'identité de son ouverture. La
  déconnexion menant à une page complète, l'onglet referme son flux ; un autre
  onglet du même navigateur garde le sien jusqu'à sa prochaine navigation.
- **Le toast global** : l'enrichir (sous-titre, lien) ne doit changer ni la
  signature `kreekToast(message, type)` ni l'écouteur `showToast`. Corriger
  `warning` au passage rendrait orange un toast aujourd'hui vert : c'est un
  changement visible, laissé hors épic (§ 12).

## 11. Décisions du 2026-11-02

Les dix-huit décisions sont prises telles que la fiche les recommandait.

1. **SSE, pas de websocket** (642) : le flux va du serveur au navigateur, les
   gestes du coach restent des requêtes HTMX, et axum le sert nativement.
2. **`EventSource` natif** (642) : l'extension SSE de htmx n'est pas livrée et
   lie un flux à un élément ; un seul flux dans le layout traduit les messages
   en événements DOM sur `body`.
3. **Un signal pour le direct, du texte pour les toasts** (646, 645) : la route
   du BC garde ses droits et son gabarit, le canal ne connaît aucun écran.
4. **Un flux par onglet** (642), avec `http2` en production : élire un onglet
   relais est une complexité que la ligue ne justifie pas.
5. **Le canal résout les coachs d'un match** (645), par un port
   `coaches_of_match` adossé à `teams` : l'app event reste un fait, sans liste
   de destinataires.
6. **La borne par l'espace est celle de l'équipe** (645) : le coach d'une
   équipe du match est concerné même s'il a quitté l'espace, et l'e-mail, qui
   justifiait l'intersection de E02, n'est pas en jeu.
7. **Font un toast** (643, 645) : le démarrage, une action (MVP compris), la
   sélection des coups de pouce validée — avec la liste de ce qui a été pris —,
   l'annulation des coups de pouce. Le retrait d'une action et les autres faits
   ne font qu'un signal.
8. **Le `every 15s` de `/live` est retiré** (646), remplacé par le signal et
   `realtimeResync` à la reconnexion : garder les deux masquerait une panne du
   canal.
9. **Une page s'abonne par un `POST` de présence** (644), déclenché par le
   layout d'après un marqueur de la page ; le rôle est calculé par le serveur.
   C'est la seule forme compatible avec un flux qui survit aux navigations HTMX.
10. **Un MVP tiré au sort fait une seule notification** (643), sur
    l'`ActionRecorded` ; `MvpDrawn` reste dans le BC.
11. **`MatchReportConfirmed` est enrichi** (643) de `confirmed_by` et de son
    nom : aucun listener ne casse, et un second événement dédoublerait le fait.
12. **Le publisher nomme l'auteur** (643), par `ICoachDataPort` qu'on lui
    injecte : le canal n'a pas de port vers l'identité.
13. **Le toast est livré aussi sur la page même du match** (645) : c'est le
    seul signe visible de ce qui vient de changer.
14. **Un toast à lien dure 6 s** (645), cliquable, le lien sur le sous-titre ;
    les toasts existants gardent leurs 2,5 s.
15. **nginx : le bloc dédié et l'en-tête** (642) : `X-Accel-Buffering: no`,
    posé par l'application, est ce que les tests vérifient ; le bloc fixe ce
    que l'en-tête ne règle pas — délai, `gzip`, `Connection` — et `http2`
    s'active sur `listen`.
16. **Battement de cœur à 15 s** (642), `KeepAlive::default()`, sous les 60 s
    du `proxy_read_timeout` par défaut ; `retry: 5000`.
17. **Les coachs e2e au-delà de deux se connectent pour de vrai** (645, 647),
    par `/auth/login` et `SEED_PASSWORD` : aucun code de production nouveau.
18. **L'ordre** : 642 et 643 en parallèle, puis 644 et 645, puis 646, puis 647.

## 12. Ce que le code contredit dans l'épic

- **« Ni SSE, ni websocket, ni polling »** : `/live` interroge le serveur toutes
  les 15 s pour qui suit (`live-match.html:56`), et le développement a déjà un
  flux SSE (`tower-livereload`).
- **« Les listeners devront nommer les nouveaux variants — le compilateur
  l'exige »** : aucun listener ne fait de `match` exhaustif (§ 1). Rien ne
  casse, rien ne vérifie non plus qu'un listener ignore un variant à bon
  escient.
- **« Les faits sortent par le publisher »** suppose qu'ils soient émis :
  quatre sur six ne le sont pas aujourd'hui (§ 1). La carte 3 commence par là.
- **« Aucune donnée persistante »** : vrai du canal, pas de la carte 3, qui
  remplit `event_log`.
- **« camp validé »**, « l'attente de l'outsider », « l'état des validations »
  n'existent pas avant E22.
- **« Un flux par session »** et « plusieurs onglets » se contredisent ; la
  fiche retient un flux par onglet.
- **La résolution de E02** ne sert pas telle quelle : elle parle d'invités et
  d'inscrits d'une saison, et aucun port ne rend le coach d'une équipe.
- **Le middleware CSRF** décrit par `CLAUDE.md` n'existe pas ; le `POST` de
  présence n'est protégé que par `SameSite=Lax`, comme toutes les mutations.
- **`kreekToast` ignore `warning`** (`app-layout.html:98-100`) : les
  avertissements du calendrier s'affichent en vert. Hors épic, mais la carte 4
  touche cette fonction.

## 13. Correction de l'épic

Les constats du § 12 que l'épic doit intégrer, reportés dans son fichier le
2026-11-02 :

- **« Aucune donnée persistante » est faux pour la 643** : émettre les faits de
  saisie écrit une ligne par fait dans `event_log` (`event_log_feeder`). Le
  canal, lui, ne persiste rien.
- **Les faits ne sortent qu'une fois émis** : la 643 commence par ajouter
  `emettre()` dans cinq use cases, avant toute variante d'app event.
- **Les listeners ne nomment rien** : aucun n'est exhaustif, le compilateur
  n'exigera rien d'eux. La phrase de l'épic est retirée.
- **« camp validé » est renvoyé à E22**, qui branchera son app event sur le
  canal ; « l'attente de l'outsider » et « l'état des validations » aussi. Le
  direct de E23 se réduit au **journal et au score de `/live`**.
- **Un flux par onglet**, et non par session.
- **Le polling existe** (`live-match.html:56`) : la 646 le retire.
- **La résolution de E02 n'est pas réutilisée** ; seul son patron l'est.
- **Chaque carte porte ses tests unitaires et e2e** ; la 647 garde le scénario
  des trois matchs.
- **L'ordre** : 642 et 643 en parallèle (décision 18).
- **Le middleware CSRF absent** est sorti de l'épic : carte 648, sans épic.
