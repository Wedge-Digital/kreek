# Le canal SSE et le registre des connexions

**Épic :** E23 — Le temps réel : connexions, présence et notifications · **Ordre :** 1 (en parallèle de la 643) · **Dépend de :** rien
**Priorité :** normale
**Conception :** `docs/specs/saisie-mobile/E23-cadrage.md` (R1, R3, R8, § 4, § 5, § 6, § 11.1, 11.2, 11.4, 11.15, 11.16)

## Objectif

L'application gagne un canal du serveur vers le navigateur : chaque onglet
authentifié ouvre un flux SSE, le serveur sait quelles connexions sont
ouvertes et à qui elles appartiennent, et une connexion fermée disparaît du
registre sans geste du client. Rien n'y passe encore que le battement de cœur
et l'événement d'accueil : la 644, la 645 et la 646 s'y branchent.

## Ce que le code fait

- Aucun canal serveur → navigateur en production ; seul `tower-livereload`
  ouvre un flux SSE, en build de développement (`src/main.rs:724-738`).
- axum 0.8.9 fournit `axum::response::sse::{Sse, Event, KeepAlive}` ; `Sse` pose
  `content-type: text/event-stream` et `cache-control: no-cache`, pas
  `X-Accel-Buffering` ; `KeepAlive::default()` bat toutes les 15 s.
- `futures-util` et `dashmap` sont des dépendances directes ; `tokio-stream`
  n'est que transitif (`Cargo.lock:3227`).
- Toute route `/app/…` passe par `require_auth` (`main.rs:678`), qui renvoie un
  `303` vers la connexion sans session (`src/web/middleware/require_auth.rs:22-29`).
- `request_log` journalise la réponse à l'ouverture, et son span ne couvre pas
  le corps (`src/web/middleware/request_log.rs:47-77`).
- Aucun `TimeoutLayer`, `server.request_timeout_ms` lu nulle part
  (`src/config.rs:33`), pas de `with_graceful_shutdown` (`main.rs:774`).
- Le layout charge Alpine en `defer` et le toast en JS nu pour cette raison
  (`src/web/templates/app-layout.html:70-76`).

## Ce qu'il faut faire

- **Le registre**, `src/web/realtime/registry.rs` : `ConnectionRegistry` sur
  `DashMap<ConnectionId, Connection>` plus un index utilisateur → connexions.
  Une connexion porte `user_id`, `coach_name` pris sur `AuthSession` à
  l'ouverture, un `mpsc::Sender<RealtimeMessage>` borné à 32, et une présence
  `Option<…>` vide (la 644 la remplit). Livraison par `try_send` : une file
  pleine perd le message et le compte, elle ne bloque jamais (R8).
  `Arc<ConnectionRegistry>` entre dans `AppState` (`src/state.rs`), construit
  dans `compose`.
- **Le flux**, `src/web/realtime/stream_controller.rs`, `GET /app/realtime/stream`
  sous `require_auth` : enregistrement, premier événement `hello` portant
  l'identifiant de connexion, puis les messages ; `KeepAlive::default()`,
  `retry: 5000`, en-tête `X-Accel-Buffering: no`. Le stream porte une garde dont
  le `Drop` retire la connexion et journalise la fermeture — seul nettoyage.
- **La route** dans `src/web/routes.rs` (`path` et `Routes`) et
  `src/web/router.rs`.
- **Le client**, dans `app-layout.html` à côté du toast, en JS nu : un
  `EventSource` sur `app_routes.web.realtime_stream()` (jamais d'URL en dur),
  qui garde l'identifiant de connexion et traduit chaque type de message en
  événement DOM sur `body`. À la reconnexion (`open` après `error`), il émet
  `realtimeResync`.
- **Le retour du bfcache** : sur `pageshow` avec `persisted` et un flux fermé
  (`readyState === CLOSED`), le client rouvre le flux et émet `realtimeResync`.
- **Le journal**, cible `kreek::web::realtime`, niveau `info` : `flux ouvert`
  (`conn`, `coach`, `ouvertes=N`) et `flux fermé` (`conn`, `duree_s`,
  `livres`, `perdus`, `ouvertes=N`) ; `warn` sur une file pleine.
- **L'axe 12** : aucun `.send(` nu ; `try_send` n'est pas visé, un `.send(`
  éventuel porte `// arch:ok envoi à une connexion SSE`.

## Fichiers touchés

`src/web/realtime/` (nouveau : `mod.rs`, `registry.rs`, `stream_controller.rs`),
`src/web/mod.rs`, `src/web/routes.rs`, `src/web/router.rs`, `src/state.rs`,
`src/main.rs` (`compose`), `src/web/templates/app-layout.html`, `Cargo.toml` si
`tokio-stream` est retenu pour envelopper le récepteur.

## Tests

- **Unitaires** (`src/web/realtime/tests/`) : ouverture et fermeture au `Drop` ;
  deux onglets d'un même utilisateur, deux entrées, la fermeture de l'un laisse
  l'autre ; une file pleine perd sans bloquer et compte la perte.
- **Harnais** (`src/web/test_harness.rs`) : la réponse du flux porte
  `content-type: text/event-stream`, `cache-control: no-cache`,
  `X-Accel-Buffering: no`, et son premier événement est `hello`.
- **E2E** (`tests/e2e/test_realtime_stream.py`) : une page de l'application ouvre
  le flux et reçoit `hello` ; une navigation par le menu (htmx, `hx-select`) ne
  le rouvre pas. Pas de `networkidle` : le flux reste ouvert.
- `tests/impact-map.toml` mis à jour (axe 8).

## Hors périmètre

- La présence (644), le routage des notifications (645), le direct (646).
- Le relais `LISTEN/NOTIFY` pour plusieurs instances : décrit au § 4, non
  construit.
- La fermeture des flux par un arrêt gracieux : il n'y en a pas aujourd'hui.

## Conséquences sur les données de production

**Aucune donnée** n'est lue ni écrite : le registre vit en mémoire.

**La configuration nginx est à appliquer avant la livraison**, sans quoi le flux
n'arrive pas (tampon actif par défaut) ou se coupe. Dans le `server` de
`bloodbowlclub.com`, avant `location /` pour la lisibilité — une `location =`
exacte l'emporte quel que soit l'ordre :

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

Et **`http2`** sur la ligne gérée par Certbot — `listen 443 ssl http2;` avant
nginx 1.25.1, `http2 on;` à partir de cette version : en HTTP/1.1, le
navigateur plafonne à six connexions par origine, et six onglets ouverts
bloqueraient tout autre clic.

L'application envoie aussi **`X-Accel-Buffering: no`** sur la réponse du flux :
c'est la garantie que les tests vérifient, valable même si le bloc manquait.

À vérifier sur le serveur : `nginx -v` (syntaxe `http2`) ;
`nginx -T | grep -E 'gzip|proxy_buffering|proxy_read_timeout|worker_connections|worker_shutdown_timeout'` ;
`worker_connections`, chaque flux en tenant deux ; `worker_shutdown_timeout 30s`
pour borner un rechargement ; l'absence d'un autre mandataire devant nginx.

Effets d'exploitation : une connexion par onglet authentifié ; un redéploiement
coupe tous les flux — tant que la 491 n'est pas livrée, la reconnexion reçoit un
`303` et l'`EventSource` s'arrête.

## Terminé quand

- [ ] Un onglet authentifié ouvre `GET /app/realtime/stream` et reçoit `hello`, puis un battement toutes les 15 s
- [ ] La réponse porte `X-Accel-Buffering: no`, vérifié par le harnais
- [ ] Fermer l'onglet produit la ligne `flux fermé` et fait baisser `ouvertes=N`
- [ ] Une navigation par le menu ne rouvre pas le flux
- [ ] Le bloc nginx et `http2` sont appliqués en production, et un flux y arrive sans délai
- [ ] `make lint`, `make check-arch`, `make test`, e2e impactés
