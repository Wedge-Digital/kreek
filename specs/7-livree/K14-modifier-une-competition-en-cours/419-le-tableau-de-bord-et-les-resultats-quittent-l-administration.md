# Le tableau de bord et les résultats quittent l'administration

**Épic :** E14 · **Ordre :** 2 · **Dépend de :** rien
**Conception :** `docs/specs/modifier-une-competition/onglet-parametres/03-back.md`

## Objectif

Retirer deux onglets de l'administration de compétition, et faire du Résumé
l'onglet par défaut. Aucune fonctionnalité nouvelle — de la place faite.

## Ce qui part

| Fichier | Autre consommateur ? |
|---|---|
| `io/web/admin/dashboard.rs` | aucun |
| `io/web/admin/results_tab.rs` | aucun |
| `use_cases/admin/dashboard_query.rs` | `dashboard.rs` et `admin_page.rs` seuls |
| `templates/admin/dashboard.html` | aucun |
| `templates/admin/results.html` | aucun |
| `assets/static/css/pages/competition-admin-dashboard.css` | entièrement portée par `.competition-admin-dashboard` |
| `tests/e2e/test_competition_admin_dashboard.py` | ses trois cas testent l'onglet qui part |

Vérifié : **aucun de ces fichiers n'a de consommateur hors de ce périmètre**
(règle 4 du `CLAUDE.md`).

**`io/web/resultats_view.rs` reste** — il sert aussi les onglets publics
Calendrier et Résultats.

## Les routes

Disparaissent de `routes.rs` : les constantes `COMPETITION_ADMIN_DASHBOARD` et
`COMPETITION_ADMIN_RESULTS`, les méthodes `admin_dashboard()` et
`admin_results()`. Et leurs deux `.route(...)` dans `router.rs`.

Leurs seuls appelants sont les deux onglets de `admin-page.html`, qui partent
avec elles.

## L'aiguillage

`admin_page.rs` :

- les branches `"dashboard"` et `"results"` du `match active_tab` disparaissent ;
- **le défaut `_` devient `summary`** — il rend aujourd'hui le tableau de bord ;
- `admin_page()` passe `"summary"` au lieu de `"dashboard"` (ligne 53).

`admin-page.html` perd ses deux entrées d'onglet (lignes 19-24 et 46-50).

## Le bundle CSS

`pages/competition-admin-dashboard.css` sort de `src/web/css_bundle.rs:87`.
L'axe 14 de `check-arch` refuse une feuille absente du bundle — il refuserait
aussi une entrée du bundle sans feuille.

## Tests

- Supprimer `tests/e2e/test_competition_admin_dashboard.py`.
- Ajouter à la suite e2e existante : ouvrir l'administration mène au Résumé.
- `make e2e` doit rester vert : aucun autre test ne référence les deux routes.

## Deux fichiers que cette carte avait manqués

La liste « ce qui part » était juste sur les sept fichiers, et la vérification de
la règle 4 les a tous confirmés sans consommateur hors périmètre. Deux autres
fichiers référençaient pourtant ce qui disparaît.

**`tests/e2e/visual/urls.py`** — trois entrées : les URL `admin-dashboard` et
`admin-resultats`, plus le sélecteur `.competition-admin-dashboard`. Trois
scripts la lisent : `debordements.py`, `decalages.py`, `releve.py`.

Ce suivi visuel n'est branché **ni à une cible `make`, ni à la CI** — il se lance
à la main. Les deux URL mortes auraient donc rendu `404` en silence, sans qu'aucun
build ne rougisse. Et `debordements.py` est justement l'outil que le `CLAUDE.md`
cite pour attraper les feuilles CSS qui débordent : le laisser pointer vers du
vide l'aurait rendu moins fiable sans que rien ne le dise.

**`tests/impact-map.toml`** — l'entrée de l'e2e supprimé. Celle-là n'aurait pas
échappé longtemps : l'**axe 8** de `check-arch` refuse une « entrée orpheline
(fichier de test inexistant) ». Le verrou existait, la carte l'ignorait.

## L'aiguillage : le défaut passe en dernier

La carte dit « le défaut `_` devient `summary` ». Concrètement, la branche
`"summary"` **devient** le défaut — et doit donc **descendre en dernière
position**. Elle vivait au milieu du `match` ; l'y laisser en `_` aurait rendu
`"groups"` et `"schedule"` inatteignables, ce que `-D unreachable-patterns`
refuse à la compilation.

## Deux scénarios e2e, pas un

La carte demandait « ouvrir l'administration mène au Résumé ». Un second
l'accompagne : **les deux routes retirées rendent `404`, et la barre ne les
propose plus**.

Masquer un lien sans retirer la route — ou l'inverse — laisserait la moitié du
travail faite, et le premier test seul ne verrait ni l'un ni l'autre.

Le premier a été **vu échouer** : en remettant un autre onglet comme défaut, il
tombe sur l'absence de `.admin-summary`. Il observe donc le contenu servi, et non
le simple fait que la page réponde.

## Checklist

- [ ] Sept fichiers supprimés
- [ ] Deux constantes de route, deux méthodes, deux `.route(...)`
- [ ] Deux branches d'aiguillage, le défaut, la ligne 53
- [ ] Deux entrées d'onglet dans `admin-page.html`
- [ ] La feuille retirée du bundle
- [ ] `grep -r "admin_dashboard\|admin_results\|dashboard_query"` ne rend plus rien
- [ ] `make lint && make test && make check-arch && make e2e`
