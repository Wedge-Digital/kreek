# La page de gestion des points manuels

**Ordre :** 3 · **Dépend de :** 450, 451
**Conception :** `docs/specs/points-classement-manuels/page-de-gestion/`
(`02-front.md`, `04-dtos.md`, `07-integration.md`) · **Maquette :**
`assets/rawpages/html/app-manual-ranking-points.html`

## Objectif

L'écran qui attribue et qui liste.

## Pourquoi elle vient après l'affichage

À rebours de l'ordre naturel — on écrirait d'abord la saisie, puis l'affichage.
Mais la **451 rend les points visibles** : sans elle, cette carte livre un écran
dont on ne peut vérifier aucun effet. On attribue trois points, et rien ne bouge
nulle part.

## Conception

### Cinq routes

```
GET    …/manual-points            la page
GET    …/manual-points/form       le widget d'attribution
GET    …/manual-points/list       le widget de liste
POST   …/manual-points            attribuer
DELETE …/manual-points/{point_id} retirer
```

Le routeur du BC n'en compte que **deux** aujourd'hui.

| Route | Qui |
|---|---|
| les trois `GET` | **tout membre** — les points sont publics |
| `POST`, `DELETE` | admin de compétition ou d'espace |

**`{point_id}` dans le chemin, jamais dans le corps** (carte 416). Sa portée est
tenue par le `AND season_id` du `DELETE`, posé en carte 450.

### Deux widgets, un événement

| Widget | Endpoint | Trigger | Écoute |
|---|---|---|---|
| `#mp-form` | `manual_points_form` | `load` | **rien** |
| `#mp-list` | `manual_points_list` | `load` | `manualPointsChanged from:body` |

**Le formulaire n'écoute rien** : rien de ce qui se passe dans la liste ne change
ce qu'il propose.

**Deux widgets et non un fragment unique** — non pas à cause du nombre de
sections, mais parce que le formulaire doit **garder son état** pendant qu'on
attribue plusieurs lignes d'affilée. C'est le geste réel : un arbitre traite les
forfaits d'une journée en quatre attributions.

### Le retour du POST

Le formulaire est re-rendu **vidé de ses points et de son motif, l'équipe
restant choisie**, plus `HX-Trigger: manualPointsChanged`.

Tout réinitialiser ferait re-choisir l'équipe à chaque fois ; ne rien
réinitialiser ferait attribuer deux fois le même nombre par inadvertance.

| Cas | Réponse |
|---|---|
| POST réussi | le formulaire + `HX-Trigger` |
| DELETE réussi | `204` + `HX-Trigger` |
| value object refusé | `422` + le formulaire, l'erreur nommant le champ |
| `TeamNotEnrolled` | `422` |
| `Forbidden` / `NotFound` | `403` / `404` |

### Le formulaire

**`kreek-select` pour l'équipe**, alimenté par `url` — composant imposé par le
`CLAUDE.md`.

**Le sens est une bascule**, « + Bonus » / « − Pénalité », et le champ Points ne
prend que des entiers positifs. Un `-3` tapé au clavier se tape aussi bien par
erreur qu'exprès ; le handler compose, et un `direction` inconnu est un `400`.

**Le motif est facultatif**, et l'indication sous le champ le dit sans l'exiger.

**Le pied de conséquence** annonce l'effet avant qu'il ne se produise — état
d'écran pur, dérivé des trois champs, sans aller-retour.

### La liste

**Groupée par équipe, en accordéon, repliée par défaut.** Plié : le total et le
nombre de lignes. Déplié : le détail.

**L'en-tête de colonnes vit dans chaque bloc**, replié avec lui — posé une seule
fois en haut, il surplomberait des sections fermées et ne désignerait rien.

**`can_manage` dans le VM** : le gabarit rend ou non la colonne de suppression,
il ne décide pas.

Le `✕` est le geste de retrait du reste du site — six occurrences dans les
gabarits de production contre une seule poubelle.

### CSS

`pages/ranking-manual-points.css`, portée par `.mp-page`, **inscrite dans
`src/web/css_bundle.rs`** — l'axe 14 refuse une feuille absente du bundle.

### Responsivité

Sous 768 px, **« Attribué par » disparaît en premier** : c'est la colonne dont
l'absence coûte le moins, le motif et la date restant lisibles. Le formulaire
passe en colonne.

## Tests

Unitaires, sur les builders et le handler :

| Test | Ce qu'il prouve |
|---|---|
| `le_groupement_par_equipe_somme_les_lignes` | le total d'un bloc |
| `le_pluriel_des_lignes_suit_le_nombre` | « 1 ligne » / « 2 lignes » |
| `direction_penalty_donne_un_point_negatif` | la composition du handler |
| `un_direction_inconnu_est_un_400` | pas de repli silencieux |
| `can_manage_faux_ne_rend_pas_la_suppression` | le contrôle dans le VM |

## Checklist

- [ ] Les cinq routes et le contrôle d'accès par route
- [ ] Les deux widgets, `HX-Trigger` sur les deux mutations
- [ ] `kreek-select`, la bascule de sens, le pied de conséquence
- [ ] L'accordéon et son en-tête par bloc
- [ ] La feuille + son inscription au bundle
- [ ] Les cinq tests
- [ ] `make lint && make test && make check-arch`
