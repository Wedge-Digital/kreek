# Le tiroir de déplacement dans l'onglet Calendrier

**Priorité : haute — c'est le geste que l'administrateur cherchait**
**Dépend de :** 537, 538
**Épic :** E17 — Corriger le calendrier sans perdre la saisie
**Maquette :** `assets/rawpages/html/app-competition-admin-schedule-transfert.html`
**Fichiers :** `src/app/competitions/io/web/admin/schedule_actions.rs`,
`.../templates/admin/widgets/schedule-round-detail.html`,
`src/app/competitions/routes.rs`,
`assets/static/css/pages/competition-admin-schedule.css`

## L'écran

Sur chaque ligne de rencontre, une action **Déplacer** ouvre un tiroir **dans la
ligne** : un sélecteur de journée, un bouton de confirmation, un lien
d'annulation. La maquette en montre cinq états.

**Un tiroir, pas un calque.** Le panneau de journée vit dans un conteneur en
`overflow` — un menu flottant y serait rogné, et c'est déjà pour cette raison
que l'onglet Présences a renoncé à son menu « ⋯ ». Le tiroir pousse le contenu.

**Le refus s'affiche dans le tiroir**, pas en `alert()`. Il commente le choix
qu'on vient de faire, à l'endroit où on l'a fait — même parti que l'onglet
Présences a pris contre les boîtes du Calendrier.

**Le bouton remplace « Reporter »**, une icône déjà présente sur ces lignes et
qui n'a jamais rien fait.

## Les journées éligibles viennent du serveur

`<kreek-select>` charge sa liste par `url`. **Les journées de repos et la
journée courante en sont écartées à la source**, pas masquées côté client : on
ne propose pas un choix pour le refuser ensuite.

Un endpoint JSON de plus, sur le patron des autres sélecteurs du BC.

## Ce que la maquette a appris, et qu'il faut reprendre

Trois pièges vus au rendu, pas au code :

- **`shared.css` impose `width: 100%` à `.btn-primary`** — réutiliser ce nom
  étire le bouton et écrase le sélecteur. Les classes du tiroir sont nommées à
  part.
- **`kreek-select` a une largeur intrinsèque nulle** (`display:block` avec un
  libellé en `flex:1; min-width:0`) : toute base `auto` l'écrase. Il lui faut
  une base ferme.
- **Le bloc d'actions décale l'axe de la ligne de 37 px** : une gouttière de
  largeur égale à gauche remet le score au centre.

## Le CSS

Les règles vont dans `pages/competition-admin-schedule.css`, déjà inscrite au
bundle, sous la portée `.competition-admin-schedule`. Aucun `style="…"`, tokens
`--p0` à `--p5`, breakpoint `768px`.

## Tests e2e

| Scénario | Ce qu'il éprouve |
|---|---|
| déplacer une rencontre en attente | elle quitte la journée 1, apparaît en 15 |
| l'onglet Résultats suit | la rencontre y figure sous sa nouvelle journée |
| déplacer une rencontre **publiée** | même geste, et le classement ne bouge pas |
| viser une journée qui porte déjà l'affiche | le refus s'affiche dans le tiroir, rien n'est déplacé |
| la liste des journées | ni repos, ni journée courante |

`cliquer_quand_cable` sur les boutons du tiroir : il est injecté par htmx.
`tests/impact-map.toml` mis à jour dans le même commit.

## Checklist

- [ ] Deux routes : la liste des journées éligibles, le déplacement
- [ ] Le handler, `require_admin_access` puis `appariement_de_la_saison`
- [ ] Le tiroir dans `schedule-round-detail.html`, `hx-post` ordinaire —
      **pas d'`onclick="fetch(…)"`**, ce que cet écran fait encore ailleurs
- [ ] Le refus rendu dans le tiroir
- [ ] Le CSS scopé, la gouttière, les classes nommées à part
- [ ] Les cinq scénarios e2e + `impact-map.toml`
- [ ] `make lint`, `make check-arch`, `make test`, `make test-impacted`
