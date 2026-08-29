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

- [x] Les cinq routes **et une sixième** (cf. ci-dessous) et le contrôle d'accès
- [x] Les deux widgets, `HX-Trigger` sur les deux mutations
- [x] `kreek-select`, la bascule de sens, le pied de conséquence
- [x] L'accordéon et son en-tête par bloc
- [x] La feuille + son inscription au bundle
- [x] Les cinq tests — **dix-sept**
- [x] `make lint && make test && make check-arch`
- [x] Le lien et le bouton **reportés de la carte 451**

## Une sixième route

`kreek-select` — imposé par le `CLAUDE.md` — se nourrit d'une `url` JSON, et
**aucun endpoint ne servait les équipes inscrites d'une saison**. `competitions`
expose compétitions, saisons et journées ; `teams` expose une sélection hors
contexte de saison.

`GET …/manual-points/teams.json` est donc servi par `ranking`, depuis son propre
port — celui-là même dont le classement se nourrit déjà. Le BC qui possède la
page sert ses propres données, plutôt que d'ajouter une dépendance vers un autre.

## Le motif « facultatif » ne l'était pas

**Contradiction entre deux cartes.** La 450 a défini `ManualPointsReason` avec
`not_empty` ; la 452 dit « le motif est facultatif ». La colonne accepte `NULL`
et `ManualPointRow.reason` est un `Option<String>` — mais **le chemin d'écriture
ne pouvait jamais produire d'absence**, rendant cet `Option` inatteignable.

Trouvé à l'écran : un POST au motif vide rendait `422` au lieu de `200`.

L'optionalité a été portée d'un cran : `AwardManualPointsCommand.reason` devient
`Option<ManualPointsReason>`, `insert_manual_points` prend `Option<&str>`. Le
value object **garde** son `not_empty` — *si* un motif est donné, il ne peut pas
être blanc. Sans cette distinction, « facultatif » se serait traduit par une
chaîne vide en base, indistinguable d'un motif effacé par mégarde.

## Deux défauts que seul l'écran pouvait montrer

### `selected-value`, et non `value`

Le composant lit `selected-value`. Écrit `value`, il l'ignorait **en silence** :
l'équipe restait dans l'attribut sans jamais s'afficher, et le formulaire
réaffichait « Choisir une équipe… » après chaque enregistrement. Le commissaire
l'aurait re-choisie à chaque attribution — précisément ce que la conservation de
l'état devait éviter.

### Des classes inventées que la feuille ne connaissait pas

Le premier jet du formulaire portait `mp-field--team`, `mp-field--points`,
`mp-field--reason` et `mp-toggle` : quatre noms absents de la feuille reprise de
la maquette. Aucune erreur, juste des champs sans style. Le gabarit emploie
désormais `mp-field--grow`, `mp-input--pts` et `mp-sign`, et un contrôle compare
les classes du gabarit à celles de la feuille — seule `mp-line` reste sans
règle, ce que la feuille dit explicitement.

## La structure vient de la maquette, pas de mon premier jet

Mon relevé faisait **une table par équipe** : les largeurs de colonnes auraient
sauté d'un bloc à l'autre. La maquette n'en fait qu'une, avec des lignes de
groupe en `colspan` — les colonnes s'alignent. Le gabarit a été refait sur elle,
et l'ordre des colonnes suit le sien (Points, Motif, Date, Attribué par), ce qui
rend cohérente la responsivité que la carte demande : c'est bien la quatrième
colonne qui disparaît.

Deux écarts assumés à la maquette : l'accordéon passe par Alpine plutôt que par
un `toggleTeam` global — le `CLAUDE.md` proscrit les scripts à identifiants
globaux — et les lignes de groupe reçoivent `tabindex` et `keydown` pour être
ouvrables au clavier, ce que le `<tr onclick>` de la maquette n'était pas.

## La maquette précédait la carte 448

Elle référence `--dark-7` (quatre fois) et `--white-1` (une fois), supprimés
depuis. Elle s'affiche encore grâce à son `shared.css` propre ; copiés tels
quels, ils auraient rendu des fonds transparents **sans qu'aucune erreur ne le
signale** — le contrôle C vérifie les valeurs des tokens, pas les références à
des tokens absents. Traduits en `--dark-6` et `--white`, dont ils ne se
distinguaient pas.

## Vérification à l'écran — les neuf cas de la table de la carte

```
POST bien formé, membre simple  -> 403
POST direction inconnue         -> 400
POST points négatifs            -> 422
POST motif vide                 -> 200   (facultatif) · motif écrit : NULL
   HX-Trigger : manualPointsChanged
DELETE membre simple            -> 403
DELETE DevCoach                 -> 204   HX-Trigger : manualPointsChanged
DELETE une seconde fois         -> 404
```

Le parcours complet, dans un navigateur : le pied de conséquence suit les deux
champs sans aller-retour (« L'équipe gagnera 2 points » → « perdra »), le
sélecteur charge ses quatre équipes, l'enregistrement écrit la ligne, **la liste
se rafraîchit sur l'événement** (« 2 lignes +2 » → « 3 lignes +3 »), les points
et le motif se vident, l'équipe reste choisie. Aucune erreur JS.

À 375 px, « Attribué par » disparaît et la page ne défile pas horizontalement.
