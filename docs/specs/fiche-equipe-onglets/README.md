# Les onglets de la fiche d'équipe

**Carte :** 434 — « La fiche équipe accueille des onglets »
**Épic :** E06 — La fiche d'équipe complétée

## Pourquoi cette spec vit ici et non dans celle de la trésorerie

Le mécanisme d'onglets a été conçu dans
`docs/specs/tresorerie-equipe/onglet-tresorerie/`, parce que la trésorerie était
la première à en avoir besoin. Il **n'a plus un seul consommateur** :

| Onglet | Contenu servi par | Carte |
|---|---|---|
| Joueurs & Staff | `teams` (et le widget joueurs par `players`) | **434** |
| Trésorerie | `teams` | **436** |
| Matchs | **`competitions`** | **477** |

Trois cartes en dépendent, dont une d'un autre chantier. Laisser sa conception
dans le dossier de la trésorerie obligerait à l'y chercher pour brancher les
matchs — et à la contredire sans le voir, ce qui vient d'arriver (§ « La
correction »).

## L'état actuel

```html
<!-- teams-team-detail.html:145 -->
<div class="tabs team-tabs">
  <div class="tab active">Joueurs &amp; Staff</div>
  <div class="tab">Matchs</div>
  <div class="tab">Trésorerie</div>
</div>
```

**Trois `<div>` inertes.** Aucun `hx-get`, aucun lien, aucune route. La fiche
équipe n'a qu'une route — `TEAM_DETAIL` — et tout ce qui suit les onglets (le
widget joueurs, le panneau staff) est le contenu de « Joueurs & Staff » sans que
rien ne le dise.

## Le patron, déjà en place ailleurs

L'administration de compétition le pose (`admin-page.html:19`, `admin_page.rs`
et son `match active_tab` ligne 209) :

```html
<a class="tab{% if active_tab == "treasury" %} active{% endif %}"
   hx-get="{{ routes.team_treasury(space_id, team_id) }}"
   hx-target="#team-tab-content"
   hx-swap="innerHTML"
   hx-push-url="{{ routes.team_treasury(space_id, team_id) }}">Trésorerie</a>
```

Trois propriétés qu'on ne réinvente pas :

- **le contenu seul est remplacé** — `#team-tab-content` ;
- **l'URL suit l'onglet**, donc il est partageable et rechargeable ;
- **le serveur aiguille** sur `active_tab`, il n'y a pas d'état d'onglet côté
  client.

**Aucun JS.** La maquette bascule par un `showTab()` en JavaScript ; ce n'est pas
ce qu'on livre. Un onglet en JS ne survit ni au partage d'un lien, ni au
rechargement, ni au bouton Précédent.

**`innerHTML` sur `#team-tab-content`** : le fragment est le *contenu* du
conteneur, jamais le conteneur lui-même. C'est la forme que le `CLAUDE.md`
exige — l'erreur qu'il proscrit est le fragment qui **répète l'`id`** de sa
cible.

## Les routes — une par onglet

```rust
// teams/routes.rs
TEAM_DETAIL    "/app/{space_id}/teams/{team_id}"                 // Joueurs & Staff
TEAM_TREASURY  "/app/{space_id}/teams/{team_id}/tresorerie"      // carte 436
TEAM_MATCHES   "/app/{space_id}/teams/{team_id}/matchs"          // carte 477
```

Toutes portent `{team_id}` : `space_scope` les couvre **sans rien ajouter**, le
résolveur de `teams` existant s'en charge (`infrastructure/teams/space_ownership.rs`).

**Un onglet n'a pas de route de fragment séparée.** C'est la route de page que
le `hx-get` appelle, et le handler distingue les deux usages à l'en-tête
`HX-Request` — comme l'administration de compétition. Une seconde route
`…/matchs/fragment` doublerait la surface pour la même réponse.

Un `active_tab` inconnu ou absent rend « Joueurs & Staff ». C'est le
`_ => render_squad(…)` du `match`, et non une erreur : une URL tapée à la main
ne doit pas produire un 404 sur une page qui existe.

## Le contenu d'un onglet peut venir d'un autre BC

C'est ce que la conception d'origine n'avait pas à prévoir, et qui compte le
plus ici.

L'onglet **Matchs** affiche des données de `competition_match_display_proj`,
table du BC `competitions`. `teams` n'a pas le droit de la lire.

### Ce qu'on ne fait pas

**Pointer l'onglet directement sur la route de `competitions`.** Le
`hx-push-url` mettrait alors une URL `/competitions/…` dans la barre d'adresse
pendant qu'on regarde une fiche d'équipe, et un rechargement livrerait le
fragment nu, sans la fiche autour.

### Ce qu'on fait

`teams` possède la route et rend une **coquille**, qui charge le fragment de
l'autre BC :

```html
<!-- teams-matches-tab.html — servi par teams sur TEAM_MATCHES -->
<div id="team-matches"
     hx-get="{{ vm.matches_widget_url }}"
     hx-trigger="load"
     hx-target="this"
     hx-swap="outerHTML">
  <div class="loading-placeholder">Chargement des matchs…</div>
</div>
```

```rust
matches_widget_url: app_routes.competitions.team_matches_widget(space_id, team_id),
```

**C'est le patron déjà en place à la ligne 153 de la même fiche**, où le widget
joueurs est délégué à `players` — même structure, même `hx-trigger="load"`, même
placeholder.

Le prix est **un aller-retour de plus au premier clic** sur cet onglet. C'est le
prix de la souveraineté des données, et il est déjà payé pour les joueurs.

## La règle qui décide quand un onglet se câble

> Un onglet ne devient cliquable **que lorsque son contenu existe.** Une route
> qui répond « rien » se lit comme une panne.

C'est la règle de la carte 434, et elle est juste. Elle donne l'ordre de
livraison :

```
434 → « Joueurs & Staff » cliquable ; Trésorerie et Matchs restent des <div>
436 → « Trésorerie » devient un <a>
477 → « Matchs » devient un <a>
```

Chaque carte câble **son** onglet. La 434 pose le mécanisme et n'en câble qu'un.

### La correction

La carte 434 écrit aujourd'hui :

> **« Matchs » reste inerte définitivement** : hors périmètre, et une route qui
> répond « rien » se lirait comme une panne.

La seconde moitié reste vraie ; **« définitivement » ne l'est plus**. La carte
477 câble cet onglet, et la 434 est écrite comme si personne ne devait jamais le
faire — c'est exactement le genre de phrase qu'on relit six mois plus tard en
concluant qu'il ne faut pas y toucher.

## Le style

**Rien à ajouter.** `.tabs` et `.tab` existent dans `pages/team-page.css` —
c'est ce qui fait que les trois `<div>` s'affichent correctement aujourd'hui. Un
`<a>` apporte son `cursor: pointer` de lui-même.

## Le piège de test

Le contenu d'un onglet arrive par swap HTMX. Tout clic qui suit tombe dans la
fenêtre où l'élément est peint mais **pas encore câblé** — le clic s'y perd sans
requête, sans erreur de console, sans rien.

```python
from htmx_helpers import cliquer_quand_cable
cliquer_quand_cable(page, '.tab[href*="/matchs"]')
```

Et pour l'onglet Matchs, la fenêtre est **double** : le clic charge la coquille,
qui charge à son tour le fragment de `competitions`. Un test qui vérifie le
contenu doit attendre le second chargement, pas le premier.

**Aucun `sleep`.**

## Ce que le mécanisme ne fait pas

- **Aucun préchargement** des onglets non visités.
- **Aucune mémoire** de l'onglet visité d'une visite à l'autre — l'URL suffit.
- **Aucun quatrième onglet.** Le bilan V/N/D de la carte `match-01` est un
  widget *dans* un onglet existant, pas un onglet de plus.

## Ce qui en dépend

| Carte | Ce qu'elle attend |
|---|---|
| 436 — Le relevé de trésorerie s'affiche | `#team-tab-content`, `TEAM_TREASURY`, l'aiguillage |
| 477 — L'onglet Matchs d'une équipe | les mêmes, plus `TEAM_MATCHES` et la coquille |
| 478 — Les tests e2e de l'onglet Matchs | par la 477 |

**La 434 n'a elle-même aucune dépendance** : c'est la première de E06 à livrer.
