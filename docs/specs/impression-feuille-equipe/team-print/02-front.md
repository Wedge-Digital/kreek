# Page « Imprimer » — Phase 2 : architecture front

Maquette : `assets/rawpages/html/app-team-print-modal.html`. Elle montre une
modale ; l'application en fait une **page dédiée**, même disposition : le
projet n'a aucune modale et les écarte explicitement
(`teams-treasury-tab.html:61`, « jamais une modale »).

## Principe : la page est sa propre vue d'impression

Pas d'iframe. La page « Imprimer » contient les options, l'aperçu et la barre
d'actions ; l'aperçu est une zone rechargée par HTMX, qui contient les feuilles
A4.

| Média | Ce qui est rendu |
|---|---|
| écran | options à gauche, feuilles réduites à la largeur de l'aperçu à droite |
| impression | **les feuilles seules**, à taille réelle, une par page A4 — chrome de l'application, options et barre d'actions masqués |

« Imprimer » appelle `window.print()`. Le navigateur propose « Enregistrer en
PDF » ; il n'y a pas de téléchargement côté serveur.

## Widgets

| Widget | BC | Endpoint (indicatif, fixé en phase 3) | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| Page « Imprimer » (hôte) | teams | `GET /app/{space_id}/teams/{team_id}/print` | navigation depuis le bouton de la fiche | — | assemblage |
| Options d'impression | teams (dans la page) | — | — | `change` du formulaire | sélection, état local |
| Option « prochain adversaire » | competitions | `GET …/competitions/teams/{team_id}/print/next-opponent` | `load` | — | lecture |
| Choix d'une autre équipe | competitions + teams | routes JSON existantes : compétitions, saisons, `team-selection/json` | cascade `kreek-select` | `printCompetitionSelected`, `printSeasonSelected` | sélection |
| Aperçu (assemblage des feuilles) | teams | `GET …/teams/{team_id}/print/preview?<options>` | `load`, `change from:#print-options delay:300ms` | — | lecture |
| En-tête de fiche | teams | `GET …/teams/{team_id}/print/sheet-head` | `load` | — | lecture |
| Tableau des joueurs | players | `GET …/players/by-team/{team_id}/print` | `load` | — | lecture |
| Annexe des compétences | players | à fixer en phase 3 | `load` | — | lecture |
| Coups de pouce et star players | à fixer en phase 3 | — | `load` | — | lecture |
| Feuille de match | match_report | à fixer en phase 3 | `load` | — | lecture |

Chaque fragment pose `hx-disinherit="*"` et reçoit ses paramètres **dans
l'URL**, fabriquée par l'hôte via `AppRoutes`.

## Événements

- `printCompetitionSelected` / `printSeasonSelected` : `{ id, name }` — émis
  par les `kreek-select` de la cascade, écoutés par le `kreek-select` suivant.
  Préfixés `print` pour ne croiser aucun écouteur existant.
- `printPreviewReady` : `{ pages }` — émis par le script de la page quand
  **tous** les fragments de l'aperçu sont chargés et les polices prêtes
  (`document.fonts.ready`), après le contrôle de débordement. Écouté par la
  barre d'actions : active « Imprimer », affiche le nombre de pages.

Aucune action en écriture : la page ne fait que des `GET`.

## Front ou back

| Côté front (script de la page, fonctions ≤ 20 lignes) | Côté back |
|---|---|
| état des options, réduction des feuilles à l'écran | toutes les données et tous les calculs |
| suivi du chargement → `printPreviewReady` | découpage des annexes en pages |
| contrôle de débordement → densité compacte automatique | prix, disponibilités, barème SPP, favori et petite monnaie |
| `window.print()` | contrôle d'appartenance des équipes à l'espace |

## Décisions de cette phase

1. **Page dédiée, pas de modale, pas d'iframe.**
2. **Le CSS d'impression ne touche que cette page.**
   - `@page` passe par des **pages nommées** (`a4-portrait`, `a4-landscape`) que seules les feuilles déclarent : le Ctrl+P des autres pages ne change pas.
   - Le chrome du layout est masqué par un sélecteur limité à la racine de la page d'impression.
   - Tout le reste est limité à `.team-print`, conformément au bundle et aux axes 14, 15 et 17.
3. **Aucun `style="…"`.**
   - Le logo de l'équipe est son vrai logo, ou ses initiales.
   - La réduction à l'écran est appliquée par le script, pas par le gabarit.
4. **Aucune donnée coupée en silence.**
   - Une feuille qui déborde passe automatiquement en densité compacte.
   - Si elle déborde encore, un avertissement s'affiche dans l'aperçu.
5. **Une vue transpose, elle ne dérive pas.**
   - VE, joueurs alignables, favori, petite monnaie, prix des coups de pouce et barème SPP viennent de méthodes du domaine.
   - Rien n'est recalculé dans un gabarit ou en JavaScript.
6. **Sécurité.** Toute équipe reçue, la mienne comme l'autre, est **vérifiée comme appartenant à l'espace** de l'URL.
   - `space_scope` ne contrôle que les paramètres de chemin : les identifiants d'équipe passent donc dans le chemin, ou sont vérifiés explicitement.
   - On ne reproduit pas le défaut de `team-selection/json`.
7. **« Imprimer » attend `printPreviewReady`.** Pas d'impression d'un aperçu à moitié chargé : c'est le défaut actuel, et la fenêtre « visible mais pas câblé » de HTMX est documentée dans le `CLAUDE.md`.

## Règles métier identifiées

| # | Règle |
|---|---|
| R1 | Le bouton « Imprimer » de la fiche n'apparaît que pour une équipe prête à jouer (`ReadyToPlay`), comme aujourd'hui. |
| R2 | « Prochain adversaire » : si l'équipe est inscrite dans plusieurs compétitions, le coach choisit la compétition ciblée. Sans match à venir, l'option est grisée avec son motif. |
| R3 | « Autre équipe » : compétition, puis saison, puis équipe, dans l'espace de mon équipe ; seules les équipes prêtes à jouer sont proposées. |
| R4 | VE imprimée : la VE courante, celle que le rapport de match figera, journaliers compris. Elle sert aussi au favori et à la petite monnaie. |
| R5 | Colonne SPP : le **solde disponible** (`spp_remaining`), pas le cumul gagné. |
| R6 | Les journaliers sont imprimés dans la liste des joueurs, marqués comme tels. |
| R7 | Coups de pouce : un catalogue par équipe, celui du palier de **sa** compétition, au prix de son roster. |
| R8 | La trésorerie n'apparaît que sur la fiche de mon équipe, et seulement si l'option est cochée. |
| R9 | Rappels du MVP et des fans, et liste des météos : lus dans `match_reminders_fr.json`, absents si le corpus ne le fournit pas. |

## Découpage en deux PR

Les deux PR passent chacune `make lint`, `check-arch`, `test` et `e2e`.

| PR | Contenu |
|---|---|
| 1 | la page, les options, l'aperçu, les fiches d'équipe (en-tête et joueurs), le choix des équipes, l'impression |
| 2 | les annexes (compétences, coups de pouce, star players), la feuille de match, `match_reminders_fr.json` |
