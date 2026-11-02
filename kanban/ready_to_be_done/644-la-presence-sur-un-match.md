# La présence sur un match

**Épic :** E23 — Le temps réel : connexions, présence et notifications · **Ordre :** 2 · **Dépend de :** 642
**Priorité :** normale
**Conception :** `docs/specs/saisie-mobile/E23-cadrage.md` (R6, R7, § 4 « La présence », § 5, § 11.9)
**Maquette :** `assets/rawpages/html/app-mobile-solo-follow.html`

## Objectif

La page d'un match dit au serveur que cet onglet la regarde. On sait alors qui
suit un match et qui le saisit, et l'écran de suivi l'affiche : « Morgane
saisit ». Un onglet fermé ou parti ailleurs cesse d'y compter.

## Ce que le code fait

- Le flux de la 642 est ouvert une fois par onglet et survit aux navigations
  htmx : le menu remplace `#app-content` par `hx-select`
  (`src/web/templates/app-menu.html:9-10`). Un abonnement ne peut donc pas être
  un paramètre du flux.
- Les droits sur un match : `is_authorized` (admins et coachs des deux
  équipes) et `can_write_report` (`src/app/match_report/use_cases/match_report_access_service.rs:45-55`,
  `:114-128`), lus sur la projection.
- `/live` affiche un bandeau statique « … saisit tout le match » à qui ne peut
  pas écrire (`src/app/match_report/io/web/templates/live-match.html:42-50`).
- Le précédent d'un port de la couche web vers un BC : `IHorsCalendrierPort`
  (`src/web/ports.rs:1-14`), implémenté dans `src/infrastructure/web/hors_calendrier_adapter.rs`.

## Ce qu'il faut faire

- **Le port** dans `src/web/ports.rs` :
  `match_role(user, space_id, match_report_id) -> MatchRole { None, Follow, Write }`,
  implémenté par `src/infrastructure/web/match_role_adapter.rs` sur
  `is_authorized` et `can_write_report`, sans les réécrire. Une erreur de port
  vaut `Aucun`.
- **Le marqueur** dans les pages de match — `live-match.html`,
  `actions-step.html`, `mvp-step.html` :
  `<div class="realtime-presence" data-match="…" data-space="…" hidden></div>`.
- **La déclaration** : le script du layout, au chargement et après chaque
  `htmx:afterSettle` de `#app-content`, envoie
  `POST /app/realtime/presence` (`HX-Request: true`) avec l'identifiant de
  connexion et le match du marqueur — ou sans match s'il n'y en a plus. Le
  serveur demande `match_role` : `Aucun` répond 403, sinon il enregistre le
  rôle **qu'il a calculé** sur la connexion, jamais ce que la page prétend.
- **Le départ** : une présence s'efface quand la connexion tombe (garde de la
  642), ou quand l'onglet en déclare une autre ou aucune.
- **Le widget** `GET /app/{space_id}/realtime/matches/{match_report_id}/presence`,
  gardé par `space_scope` et `match_role` : les autres connectés présents, par
  nom, « saisit » pour le rôle `Saisie`, « suit le match » pour `Suivi`. Il ne
  lit que le registre. Il se recharge sur `presenceChanged`, émis aux présents
  d'un match à chaque arrivée ou départ.
- **Sur `/live`**, le widget remplace la première ligne du bandeau statique ;
  sa phrase d'explication reste.
- **Le journal** : `présence` en `debug` (`conn`, `match_report_id`, `role`).

## Fichiers touchés

`src/web/realtime/` (`presence_controller.rs`, `presence_widget.rs`, registre),
`src/web/ports.rs`, `src/web/routes.rs`, `src/web/router.rs`,
`src/infrastructure/web/match_role_adapter.rs`, `src/main.rs`,
`src/web/templates/app-layout.html`, `src/web/templates/realtime/presence-widget.html`
et sa feuille au bundle (`src/web/css_bundle.rs`), `live-match.html`,
`actions-step.html`, `mvp-step.html`.

## Tests

- **Unitaires** : une présence posée, remplacée, effacée par la déclaration
  vide et par la fermeture ; le rôle enregistré est celui du port ; `Aucun`
  refuse ; le widget ne liste pas l'onglet qui le demande.
- **E2E** (`tests/e2e/test_realtime_presence.py`) : DevCoach saisit sur
  `/step3`, le membre simple (`X-Bypass-Auth-Profile: simple`) ouvre `/live`
  dans un second contexte et lit « DevCoach saisit » ; DevCoach ferme son
  onglet, la mention disparaît chez le membre simple. Un tiers qui déclare sa
  présence reçoit 403.
- `tests/impact-map.toml` et `tests/e2e/visual/urls.py` mis à jour.

## Hors périmètre

- Le rechargement du journal sur signal : 646.
- Une présence hors des pages de match.

## Conséquences sur les données de production

Aucune : la présence vit dans le registre, en mémoire. Le `POST` de présence
n'est gardé contre les requêtes inter-sites que par `SameSite=Lax`, comme
toutes les mutations (carte 648).

## Terminé quand

- [ ] Sur `/live`, l'autre coach lit « Morgane saisit » quand Morgane est sur une page de saisie du match
- [ ] Morgane ferme son onglet : la mention disparaît chez l'autre coach sans qu'il recharge
- [ ] Un tiers ne peut ni déclarer sa présence sur le match ni lire le widget
- [ ] Le rôle affiché est celui que le serveur a calculé
- [ ] `make lint`, `make check-arch`, `make test`, e2e impactés
