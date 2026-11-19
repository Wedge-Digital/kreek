# Le widget d'identité d'équipe

**Priorité : moyenne — prérequis de la carte 567**
**Épic :** aucune — livrable d'un bloc avec la 567
**Dépend de :** rien
**Maquette :** `assets/rawpages/html/app-competition-standings-teams.html` (bloc `.team-identity`)
**Fichiers :**
`src/app/teams/io/web/status_view_models.rs` *(nouveau)*,
`src/app/teams/io/web/team_detail.rs`, `src/app/teams/io/web/mod.rs`,
`src/app/teams/io/web/tests/fixtures.rs` *(nouveau)*,
`src/app/teams/io/web/widgets/team_identity_widget.rs` *(nouveau)*,
`src/app/teams/io/web/widgets/mod.rs`, `src/app/teams/routes.rs`,
`src/app/teams/router.rs`,
`src/app/teams/io/web/templates/widgets/team-identity.html` *(nouveau)*,
`assets/static/css/widgets/team-identity.css` *(nouveau)*, `src/web/css_bundle.rs`

## L'objectif

`teams` expose un widget qui présente une équipe comme les Résultats la
présentent — logo, nom, roster · coach — et y ajoute son statut en badge.
N'importe quelle page peut le charger par un `hx-get`, sans rien savoir de
l'équipe au-delà de son identifiant.

## Ce qui l'a fait naître

Le classement n'affiche que le nom des équipes, quand les Résultats montrent
logo, roster et coach. Le classement appartient à `ranking`, ces données à
`teams` : la souveraineté des données interdit à `ranking` de les lire. La
composition de widgets est la réponse du projet à ce cas (carte 567).

## Le changement

**Le statut se traduit en un seul endroit.** `status_display()` vivait dans le
contrôleur de la fiche (`team_detail.rs`). Elle est **déplacée par
copier-coller** (règle 5), corps inchangé, dans `status_view_models.rs` — et
non `view_models.rs`, qui est le fichier de la phase de recrutement. La fiche
l'appelle à son nouvel endroit ; le widget aussi. Les deux écrans ne peuvent
pas diverger.

**Le widget lit l'agrégat**, par `find_by_id`, comme la fiche : le statut est
celui que calcule le domaine (`participation_status` × `game_phase`), pas une
relecture de chaînes de la projection.

- Route : `GET /app/{space_id}/teams/{team_id}/widgets/identity`.
- VM `TeamIdentityVm::from_domain(&Team)` : nom, initiales, logo (transformé
  Cloudinary comme sur la fiche), roster, coach, statut.
- Gabarit `widgets/team-identity.html`, racine `.team-identity` avec
  `hx-disinherit="*"`. Sans logo, les initiales sur dégradé.
- Équipe introuvable : 404 — l'hôte garde ce qu'il affichait.
- Feuille `widgets/team-identity.css`, portée par `.team-identity`, inscrite
  au bundle. Sous 768px : logo 32px, roster masqué (comme les Résultats),
  badge compact.

## Ce que la carte ne couvre pas

- L'insertion dans le classement : carte 567.
- Le rafraîchissement du badge en direct : il reflète le statut au chargement,
  comme la fiche équipe.

## Tests

Unitaires : `status_display` sur chaque statut de participation et
chaque phase de jeu ; `TeamIdentityVm::from_domain` avec et sans logo ; le
rendu du gabarit porte nom, coach et badge.

E2E : porté par la carte 567, qui affiche le widget en situation.

## Terminé quand

`GET /app/{space}/teams/{team}/widgets/identity` rend le logo (ou les
initiales), le nom, le badge de statut et « roster · coach » d'une équipe, et
la fiche équipe affiche toujours le même libellé de statut.
