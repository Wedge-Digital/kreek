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
  vaut `None`, et un match qui n'est pas de `space_id` aussi : le `POST` de
  présence n'a pas d'espace dans son chemin, `space_scope` ne le garde pas
  (arbitrage du 2026-11-02).
- **Le marqueur** dans les pages de match — `live-match.html`,
  `actions-step.html`, `mvp-step.html` :
  `<div class="realtime-presence" data-match="…" data-space="…" data-activity="…" hidden></div>`.
  L'**activité** est celle de la page : `follow` sur `/live`, `write` sur
  `/step3`, `/step4` et `/mvp/{side}`.
- **La déclaration** : le script du layout, à chaque `hello` (une connexion
  neuve, reconnexion comprise, n'a aucune présence) et après chaque
  `htmx:afterSettle` quand le match ou l'activité a changé, envoie
  `POST /app/realtime/presence` (`HX-Request: true`) avec l'identifiant de
  connexion, le match et l'activité du marqueur — ou rien s'il n'y en a plus.
- **Le rôle inscrit est `min(droit, activité)`** (arbitrage du 2026-11-02) :
  le serveur calcule le **droit** par `match_role`, et inscrit le minimum du
  droit et de l'activité déclarée (`None < Follow < Write`). La page peut se
  déclarer en dessous de son droit, jamais au-dessus : un admin qui lit
  `/live` « suit le match ». Un droit `None` répond 403 quelle que soit
  l'activité ; une activité illisible répond 400.
- **L'onglet est celui du demandeur** : une connexion inconnue ou appartenant
  à un autre utilisateur reçoit le même 404 — personne ne pose de présence à
  la place d'un autre, et rien n'est révélé des onglets d'autrui.
- **Le départ** : une présence s'efface quand la connexion tombe (garde de la
  642), ou quand l'onglet en déclare une autre ou aucune.
- **Le widget** `GET /app/{space_id}/realtime/matches/{match_report_id}/presence`,
  gardé par `space_scope` et `match_role` : les autres connectés présents, par
  nom, « saisit » pour le rôle `Write`, « suit le match » pour `Follow`. Le
  demandeur n'y figure sur aucun de ses onglets ; une personne à plusieurs
  onglets n'y figure qu'une fois, avec son rôle le plus fort. Il ne
  lit que le registre. Il se recharge sur `presenceChanged`, émis aux présents
  d'un match à chaque arrivée ou départ.
- **Sur `/live`**, le widget se pose **au-dessus** de la ligne statique du
  bandeau, qui reste (arbitrage du 2026-11-02) : elle dit le mode, et reste
  vraie quand l'onglet du saisisseur est fermé.
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
  vide et par la fermeture ; le rôle enregistré est celui du port ; `None`
  refuse ; une page `follow` ne donne jamais `Write`, une page `write` sans
  droit d'écrire donne `Follow` ; le widget ne liste pas l'onglet qui le
  demande.
- **E2E** (`tests/e2e/test_realtime_presence.py`) : DevCoach saisit sur
  `/step3`, le membre simple (`X-Bypass-Auth-Profile: simple`) ouvre `/live`
  dans un second contexte et lit « DevCoach saisit » ; DevCoach ferme son
  onglet, la mention disparaît chez le membre simple (P1, P2). DevCoach quitte
  le match par le menu — navigation htmx, plus de marqueur — et la mention
  disparaît aussi (P3, arbitrage du 2026-11-02). Un tiers, « E2E Coach 02 »
  vraiment connecté, qui déclare sa présence ou lit le widget reçoit 403 (P4).
- `tests/impact-map.toml` et `tests/e2e/visual/urls.py` mis à jour.

## Hors périmètre

- Le rechargement du journal sur signal : 646.
- Montrer au saisisseur qui le suit : le bandeau n'est rendu qu'à qui suit.
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
