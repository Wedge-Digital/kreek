# Le direct sur la page d'un match

**Épic :** E23 — Le temps réel : connexions, présence et notifications · **Ordre :** 3 · **Dépend de :** 642, 643, 644, 645
**Priorité :** normale
**Conception :** `docs/specs/saisie-mobile/E23-cadrage.md` (R4, § 4 « Le client », « Le routage », § 5, § 9, § 11.3, 11.8)
**Maquette :** `assets/rawpages/html/app-mobile-solo-follow.html`

## Objectif

Qui regarde `/live` voit le journal et le score bouger au moment où l'autre
saisit, sans recharger et sans interroger le serveur toutes les 15 s. Le canal
n'envoie qu'un signal — « le match X a changé » — et la page recharge ses
widgets par leurs routes, qui gardent leurs droits.

## Ce que le code fait

- **Le suivi interroge déjà** : pour qui ne peut pas écrire, le journal se
  recharge `every 15s` (`src/app/match_report/io/web/templates/live-match.html:56`, carte 636).
- **Le score est rendu dans la page** (`live-match.html:19-40`) : seul le
  journal est un widget (`:54-59`), et un signal ne le rafraîchirait pas.
- Le journal de saisie se recharge sur `actionRecorded`/`actionDeleted` locaux
  (`actions-step.html:119-121`).
- Aucun écran « l'outsider attend » ni « validations des camps » en « Je saisis
  tout » : ce direct-là appartient à E22.

## Ce qu'il faut faire

- **Le signal** : le routeur de la 645 envoie, pour tout `MatchReportAppEvent`,
  un message `match` (`{ id }`) aux connexions présentes sur ce match, auteur
  compris ; le client émet `matchChanged` sur `body`.
- **Le score en widget** : `GET /{id}/live/score` dans `routes_de_suivi()`
  (garde de E19), rendu depuis le même calcul que la page aujourd'hui ; la page
  ne garde que son conteneur.
- **Les déclencheurs** : le journal et le score de `/live` prennent
  `matchChanged[detail.id=='{{ id }}'] from:body, realtimeResync from:body`.
  Le `every 15s` est **retiré** (décision 8).
- **Le journal** : la livraison d'un signal est comptée dans les compteurs de
  la connexion (`livres`), sans ligne propre.

## Décisions du plan (2026-11-02)

1. **La forme du message** : `event: match`, données `{"match_report_id": …}`,
   comme `presence` ; l'événement DOM reste `matchChanged { id }` (règle 2 des
   widgets).
2. **Un rapport qui n'est plus en cours** (brouillon, publié, annulé) : le
   widget du score répond **204**, htmx ne remplace rien. La page redirige ; un
   widget qui suivrait cette redirection injecterait une page entière dans le
   bandeau. Rediriger celui qui suit vers le récapitulatif à la publication
   serait une autre carte.
3. **Le score écoute aussi `actionDeleted`** : la suppression locale de qui
   saisit ne dépend pas du canal.
4. **Le score se charge par `load`**, comme le journal : la page n'en garde que
   le conteneur. Les noms et les logos, qui ne changent pas pendant un match,
   restent dans la page.
5. **Le trou accepté** : un signal émis entre le chargement des widgets (ou
   l'`open` d'une reconnexion) et la réponse du `POST` de présence est perdu —
   l'onglet n'est pas encore inscrit. Quelques dizaines de millisecondes, une
   fois par chargement ; le signal suivant, ou le rechargement, rattrape. Les
   e2e attendent cette réponse avant de saisir.
6. **Pas de feuille propre au widget** : ses règles restent dans
   `pages/match-report-live.css`, il ne vit que sous `.match-report-live`.
   Capturé seul (`tests/e2e/visual/urls.py`), il paraît sans style.
7. **Les pages de saisie reçoivent aussi le signal** (rôle `Write`, auteur
   compris) sans s'y abonner : sans effet.

## Décisions du 2026-11-03 — la reconnexion

Le premier D3 coupait le flux par `context.set_offline(True)` et échouait à
chaque passage. **Mesuré** (Chromium par Playwright 1.60, serveur SSE jetable) :
la mise hors ligne fait échouer les requêtes *nouvelles*, mais un flux déjà
ouvert reçoit encore tous ses événements vingt secondes après. La même mesure a
établi qu'**un flux dont la reconnexion reçoit un 502 passe à `CLOSED` pour de
bon** : le serveur rétabli, il ne revient pas. C'est le cas de nginx pendant un
redéploiement — tout onglet ouvert perdait en silence le direct, les toasts et
la présence jusqu'au prochain chargement complet.

8. **La page rouvre un flux `CLOSED`**, dans `app-layout.html` : délai qui
   double à chaque échec (5 s, 10 s, 20 s… 60 s au plus), aléa de ±50 % pour
   étaler la vague d'après un redéploiement, remis à zéro à chaque `open`, sans
   limite de tentatives. `broken` reste vrai : l'`open` suivant émet
   `realtimeResync`, et le `hello` redéclare la présence. Un onglet dont la
   session est perdue coûte une requête par minute.
9. **La coupure des e2e vient du serveur** : `POST
   /app/realtime/test/close-my-streams` ferme les seuls flux de l'appelant
   (`ConnectionRegistry::close_user_connections`), et répond **404 hors
   `bypass_auth`**, donc en production. Retirer la connexion détruit son
   `Sender` : le flux se termine comme quand le serveur s'arrête.

## Fichiers touchés

- `src/app/shared_kernel/app_events/match_report_app_events.rs` —
  `match_report_id()`, exhaustif, extrait de `to_enveloppe()`
- `src/web/realtime/message.rs` (`MatchChanged`), `registry/presence.rs`
  (`notify_match_changed`), `notification_router.rs` (le signal avant le
  toast), `toast.rs` (commentaire), `src/web/templates/app-layout.html`
- `src/app/match_report/routes.rs`, `router.rs`,
  `io/web/widgets/live_score_widget.rs` (nouveau, `LiveScoreVm` déplacé),
  `io/web/templates/live-score-widget.html` (nouveau),
  `io/web/live_match_controller.rs`, `io/web/templates/live-match.html`
- `assets/static/css/pages/match-report-live.css` (`.live-score-slot`)
- `src/web/realtime/registry.rs` (`close_user_connections`),
  `close_streams_controller.rs` (nouveau), `mod.rs`, `src/web/routes.rs`,
  `src/web/router.rs` — la coupure réservée aux e2e
- `src/web/realtime/tests/test_match_signal.rs`, `test_close_streams.rs`
  (nouveaux), `src/web/tests/test_realtime_stream.rs`
- `tests/e2e/test_realtime_direct.py` (nouveau),
  `test_match_report_solo_entry.py`, `test_match_report_access.py`,
  `tests/e2e/visual/urls.py`, `tests/impact-map.toml`

## Tests

- **Unitaires** : le signal ne va qu'aux présents du match, pas aux présents
  d'un autre ; le VM du score, extrait de la page, rend les mêmes chiffres.
- **E2E** (`tests/e2e/test_realtime_direct.py`) : le membre simple suit `/live`,
  DevCoach saisit un touchdown dans un second contexte ; le journal et le score
  changent chez le membre simple sans rechargement, en moins de 15 s (D1) ; une
  action d'un autre match ne recharge rien (D2). Le serveur coupe le flux :
  reconnexion native et `realtimeResync` (D3a) ; reconnexion refusée par un
  502, flux `CLOSED` rouvert par la page (D3b).
- `test_match_report_solo_entry.py`, qui comptait sur le rechargement, suit le
  signal. `test_match_report_access.py` ne comptait ni sur le `every 15s` ni sur
  le score dans la page : il gagne seulement la garde du widget de score (le
  tiers reçoit 403, l'autre coach lit 1 à 0).
- `tests/impact-map.toml` mis à jour.

## Hors périmètre

- L'attente de l'outsider et l'état des validations : E22.
- Le direct sur les pages de saisie : celui qui saisit a déjà ses
  rechargements locaux.

## Conséquences sur les données de production

Aucune. Le suivi cesse d'interroger le serveur toutes les 15 s : la charge
baisse. Si le canal tombe, `/live` ne se met plus à jour qu'à la reconnexion —
c'est voulu, une panne du canal doit se voir. Après un redéploiement, les
onglets ouverts se reconnectent étalés sur quelques secondes : chacun refait
son `POST` de présence, et `/live` recharge ses deux widgets. La route de
coupure n'existe pas en production (404 hors `bypass_auth`).

## Terminé quand

- [ ] L'autre coach voit le journal et le score changer sur `/live` sans recharger
- [ ] `/live` n'interroge plus le serveur à intervalle fixe
- [ ] Après une coupure du flux, les widgets se rechargent à la reconnexion
- [ ] Un flux refusé par le serveur (502) est rouvert par la page
- [ ] Un signal d'un autre match ne recharge rien
- [ ] `make lint`, `make check-arch`, `make test`, e2e impactés
