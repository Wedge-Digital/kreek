# Le widget du logo charge Cloudinary sur chaque fiche d'équipe

**Priorité : moyenne** — un test instable, et trois traceurs tiers sur chaque
fiche d'équipe
**Épic :** aucune
**Dépend de :** rien
**Fichiers :** `src/web/templates/components/upload_widget.html`,
`tests/e2e/test_logo_equipe.py`, `tests/e2e/test_ressources_locales.py`
**Absorbe :** la carte 625, son doublon (annulée le 2026-11-11)

## Le constat (2026-10-04)

**Un test instable.** `test_dismissals_phase.py::test_01_banniere_ouvre_l_effectif`
échoue une fois sur cinq, à sa clôture, sur une erreur de console :

```
pageerror: Cannot read properties of null (reading 'addEventListener')
```

Le test ouvre la fiche d'équipe et clique aussitôt sur le bandeau, qui remplace
`#app-content` par la page des renvois.

**La cause.** Le widget du logo (PR #11, cartes 508 à 512) se charge en différé
sur la fiche et embarque la macro `cloudinary_upload`. Son script télécharge
d'abord `all.js` depuis `widget.cloudinary.com`, **puis** appelle
`document.getElementById('zone-logo_url').addEventListener(…)`. Si l'on a quitté
la fiche pendant le téléchargement, la zone n'existe plus : `null`, et l'erreur.
L'instabilité suit le temps de réponse de Cloudinary.

**Le problème plus grave que le test.** Le commit `38c02228` (2026-08-22) avait
sorti ce chargeur du layout, pour une raison écrite dans la macro : il tire avec
lui **Google Tag Manager, GA4 et Rollbar**, et ne devait suivre un coach que sur
les deux pages qui ont un champ d'envoi. Depuis la PR #11, il se charge sur
**chaque fiche d'équipe** ouverte par qui peut modifier le logo — même s'il n'y
touche jamais.

## La cause, prouvée (2026-11-11)

L'erreur est devenue **systématique** (deux fois sur deux) sur
`test_01_banniere_ouvre_l_effectif` et `test_01_banniere_ouvre_le_catalogue`,
et intermittente sur `test_team_detail_state_banner` et `test_team_print_page`
— le parcours de la carte 625, qui en était le doublon. Le client SSE d'E23,
arrivé entre-temps dans le layout, n'y est pour rien.

La pile, une fois chaque `<script>` de la réponse du widget du logo nommé par
un `//# sourceURL` injecté en test :

```
TypeError: Cannot read properties of null (reading 'addEventListener')
    at HTMLScriptElement.<anonymous> (team-logo-widget-script1.js:54:53)
```

`script1` est le script de la macro `cloudinary_upload`, et sa ligne 54 le
`document.getElementById('zone-{{ field_id }}').addEventListener('click', …)`.
Le cadre `HTMLScriptElement` est l'écouteur `load` du chargeur `all.js`. Au
même instant, le DOM est déjà sur `…/dismissals` (ou `…/recruitment`), sans
`#zone-logo_url` ni `.team-logo`.

Reproduite **six fois sur six** sans réseau : les requêtes
`widget.cloudinary.com` retenues jusqu'au départ de la fiche, puis relâchées
avec un faux `all.js`.

## La décision (2026-11-11)

**(a) puis (b), dans la macro** — la correction vaut pour ses six
consommateurs : `team-logo`, `draft-team`, `new-competition-phase-1`,
`settings-general`, `new-article`, `upload-widget-fragment` (le champ d'envoi
de `spaces`).

1. **(a) Le défaut** : la racine `.upload-widget` est capturée pendant
   l'exécution synchrone du script (`document.currentScript.closest`), tout se
   cherche dessous, et un rappel dont la racine est détachée
   (`!root.isConnected`) sort sans rien toucher. Plus aucun
   `document.getElementById` dans les rappels.
2. **(b) La cause** : le clic est branché tout de suite ; `all.js` n'est
   téléchargé qu'**au premier clic**, le widget Cloudinary créé à ce moment-là
   puis réutilisé. Aucun rappel n'attend plus pendant qu'on navigue : la course
   n'existe plus. C'est la décision de `38c02228` rétablie, et étendue — même
   les pages qui portent le champ ne chargent plus Cloudinary à l'ouverture.

## Tests

- E2E `test_logo_equipe.py::test_quitter_la_fiche_ne_reveille_pas_cloudinary` :
  requêtes Cloudinary retenues, fiche quittée par « ← Mes équipes », requêtes
  relâchées avec `FAUX_CLOUDINARY` ; aucune erreur de console, et aucune
  requête Cloudinary avant un clic. Rouge sur l'ancien gabarit, trois passages
  sur trois.
- E2E `test_ressources_locales.py` : le chargeur ne part qu'au clic sur le champ
  et n'amène que les origines connues ; le premier clic télécharge, crée et
  ouvre le widget.
- Les tests existants de `test_logo_equipe.py` ouvrent le sélecteur par un clic
  sur la zone : ils éprouvent le chargement au clic avec le faux `all.js`.

## Conséquences sur les données de production

**Aucune.** L'erreur se produisait sur une page déjà quittée, sans écriture.
Le gain est ailleurs : Google Tag Manager, GA4 et Rollbar, tirés par `all.js`,
ne sont plus chargés à l'ouverture d'une fiche d'équipe — ni d'aucune page à
champ d'envoi — mais seulement quand un coach clique pour envoyer une image.

## Terminé quand

Une fiche d'équipe ouverte sans toucher au logo ne charge aucune ressource de
Cloudinary, et la suite complète passe sans erreur de console.
