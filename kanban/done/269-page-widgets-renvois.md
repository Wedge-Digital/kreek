# Page et widgets de renvois

**Priorité : haute**
**Dépend de :** 268
**Bloque :** 271
**Spec :** `renvois/02-front.md`, `04-dtos.md` §5, `07-integration.md` §3-4
**Maquette validée :** `assets/rawpages/html/app-team-dismissals.html`
**Fichiers :** `src/app/teams/io/web/dismissals.rs`,
`io/web/widgets/dismissals_roster_widget.rs`,
`io/web/widgets/dismissals_cart_widget.rs`, `io/web/view_models.rs`,
`templates/dismissals.html`, `templates/widgets/dismissals-roster.html`,
`dismissals-cart.html`, `routes.rs`, `router.rs`

## Problème

La bannière promet « Renvoyez les joueurs dont vous ne voulez plus » et n'offre qu'un
bouton « Valider les renvois ». Aucune interface de renvoi n'existe.

## Action

### 1. Deux widgets, même schéma qu'au recrutement

| Widget | Endpoint | Trigger |
|---|---|---|
| `dismissals_roster` | `GET …/team/widgets/dismissals-roster` | `load, basketChanged from:body` |
| `dismissals_cart` | `GET …/team/widgets/dismissals-cart` | `load, basketChanged from:body` |

Le tableau se rafraîchit **entier** : marquer un joueur peut faire basculer tous les
autres en « Minimum 11 » d'un seul coup.

### 2. Routes en `mark` / `unmark`, pas `add` / `remove`

`…/dismissals/players/mark`, `players/unmark`, `staff/mark`, `staff/unmark`.

Sur une page de renvois, une route nommée `players/add` se lirait « ajouter un joueur à
l'équipe » — l'inverse exact de son effet. La symétrie avec le recrutement ne vaut pas
ce risque de contresens dans le code.

### 3. Trois états par ligne

| État | Bouton | Couleur |
|---|---|---|
| Renvoyable | « Renvoyer » | neutre, rouge au survol |
| Marqué | « Annuler » | **bleu** — seule action réversible de l'écran |
| Bloqué | « Minimum 11 », désactivé | neutre atténué |

La ligne marquée reste **lisible** : barrée, jamais estompée à l'opacité — c'est la
trace de ce que le coach vient de décider.

### 4. Pas de boîte de confirmation

Elle protégeait d'un geste irréversible. Le panier rendant chaque ligne annulable
jusqu'à la validation, elle n'a plus d'objet.

### 5. Le bouton de validation suit l'état

Navy quand le panier est vide, **rouge dès qu'une ligne est en attente** : la couleur
ne devient destructrice que lorsque le bouton va réellement détruire quelque chose.

### 6. L'avertissement de phase

« Une fois la phase validée, un renvoi est définitif et ne rembourse rien. »
Version **courte au mobile** — la version longue occupait un quart du premier écran
avant qu'on voie le moindre joueur.

### 7. L'alerte journaliers est informative

Elle ne peut plus signaler une conséquence des renvois, le plancher l'interdisant :
elle informe d'un déficit **déjà causé par les blessures**.

### 8. Conventions communes

`hx-disinherit="*"`, CSS embarqué, aucun `style=` inline, version cuite dans
`hx-vals`, `basket-error.html` **partagé** avec le recrutement. Responsive 768px, barre
du bas à `bottom: 57px`, cibles tactiles à 44px minimum.

### 9. Bannière de la fiche équipe

Le bouton « Gérer les renvois → » pointe vers la nouvelle page au lieu d'une `alert()`.

## Checklist

- [ ] Deux widgets, `basketChanged`, 1 POST + 1 GET par mutation
- [ ] Routes en `mark` / `unmark`
- [ ] Trois états par ligne, ligne marquée lisible et barrée
- [ ] Annulation possible **depuis la ligne et depuis le panier**
- [ ] Aucune boîte de confirmation
- [ ] CTA rouge uniquement quand le panier n'est pas vide
- [ ] Avertissement en version courte sous 768px
- [ ] Bannière de la fiche équipe recâblée
- [ ] `make check-arch` au vert, `make test` au vert
