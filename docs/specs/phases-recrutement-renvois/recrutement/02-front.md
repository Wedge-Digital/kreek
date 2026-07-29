# Recrutement — Phase 2 : architecture front

**Entrée** : maquette validée `assets/rawpages/html/app-team-recruitment.html`
**Page** : `/app/{space_id}/teams/{team_id}/recruitment`

## Principe

Panier serveur, sur le pattern déjà en place dans la construction d'équipe
(`team_creation`). Chaque ajout ou retrait est un POST qui mute un **panier
persisté** et renvoie un fragment HTML ; un événement DOM resynchronise le second
widget.

**Le JavaScript de la maquette disparaît presque entièrement.** Plus de `state.cart`,
plus de `queued()`, plus de `blockReason()`, plus de recalcul de trésorerie côté
client : tout devient du rendu serveur. Il ne reste en Alpine que le repli du panier
sous 768px, qui est du pur affichage.

## Widgets

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| `recruitment_catalog` | teams | `GET /app/{space_id}/team/widgets/recruitment-catalog` | `load, basketChanged from:body` | `basketChanged` (via ses POST) | lecture + mutation |
| `recruitment_cart` | teams | `GET /app/{space_id}/team/widgets/recruitment-cart` | `load, basketChanged from:body` | `basketChanged` (via ses POST) | lecture + retrait + validation |

### Pourquoi deux widgets et pas cinq

Le catalogue porte **les deux tableaux et la composition de l'effectif**. Ils vivent
dans la même colonne, dérivent tous du même panier, et les séparer multiplierait
les requêtes sans rien apporter.

Surtout : ajouter un joueur ne change pas que sa ligne. Ça peut désactiver **toutes**
les autres — trésorerie épuisée, effectif à 16, limite croisée atteinte. Un swap
chirurgical ligne par ligne serait donc faux ; le tableau se rafraîchit entier.

### Contenu de `recruitment_catalog`

- En-tête de contexte : roster, trésorerie réelle, effectif projeté, valeur d'équipe
- Tableau des postes : nom, caractéristiques, compétences, effectif `possédé (+en attente) / quota`, prix, bouton
- Tableau du staff : fonction, effectif `possédé (+en attente) / quota`, prix, bouton
- Composition de l'effectif : barres par poste, part bleue possédée / part verte en attente
- Bandeau « effectif complet » quand 16 est atteint

### Contenu de `recruitment_cart`

- Lignes du panier avec leur prix et un bouton de retrait
- Reste après achats
- Bouton de validation de phase
- État vide : « Panier vide — rien ne sera débité »

## Événements

Un seul événement DOM, sur `body` :

- **`basketChanged`** — sans payload. Émis par **toutes** les mutations via l'en-tête
  `HX-Trigger`, écouté par les deux widgets.

C'est le décalque de `teamMutated` dans la construction d'équipe
(`team_creation/io/web/widgets/staff_table_widget.rs:101`).

Aucune communication directe entre widgets : chaque mutation renvoie le fragment du
widget qui portait le bouton, et l'événement rafraîchit l'autre. **Une mutation coûte
donc 1 POST + 1 GET.**

## Actions

| Verbe | Route | Corps | Réponse |
|---|---|---|---|
| `POST` | `…/recruitment/players/add` | `roster_line_id`, `version` | fragment catalogue + `HX-Trigger: basketChanged` |
| `POST` | `…/recruitment/players/remove` | `line_id`, `version` | fragment panier + `HX-Trigger: basketChanged` |
| `POST` | `…/recruitment/staff/add` | `staff_uid`, `version` | fragment catalogue + `HX-Trigger: basketChanged` |
| `POST` | `…/recruitment/staff/remove` | `line_id`, `version` | fragment panier + `HX-Trigger: basketChanged` |
| `POST` | `…/validate-recruitment-phase` | `version` | `HX-Refresh: true` — **route existante**, dont le rôle s'élargit |

Les erreurs domaine remontent en **fragment HTML**, jamais en JSON — modèle
`player_error(...)` dans `team_creation/io/web/widgets/player_table_widget.rs`.

## Front / back

| Côté | Contenu |
|---|---|
| **Front (Alpine)** | Repli du panier sous 768px. Rien d'autre. |
| **Front (HTMX)** | `hx-post` des mutations, `hx-trigger="basketChanged from:body"` des deux widgets |
| **Back** | Tout : quotas, trésorerie, limites croisées, effectif maximum, prix de relance doublé, `allowed_staff`, raison de chaque bouton désactivé |

**Chaque bouton désactivé porte sa raison en texte** — « Quota atteint », « Effectif
complet », « Trésorerie », « Indisponible » — calculée côté serveur. C'est ce qui
permet de n'écrire les règles qu'une fois.

## Version optimiste

Chaque bouton de mutation embarque la version avec laquelle il a été rendu, cuite par
Askama dans `hx-vals` :

```html
<button hx-post="{{ routes.add_player(space_id, team_id) }}"
        hx-vals='{"roster_line_id": "{{ line.uid }}", "version": {{ basket.version }}}'>
  Recruter
</button>
```

Pas de `hx-include` vers un champ caché ailleurs dans la page : la règle 4 des
conventions widgets l'interdit, et le `hx-disinherit="*"` de la règle 3 impose que le
widget se suffise à lui-même. **Les deux widgets rendent la version courante**,
puisque tous deux portent des boutons de mutation.

En cas de conflit, le handler **n'applique pas** le clic : il renvoie le fragment
reconstruit depuis le panier à jour, avec `HX-Trigger: basketChanged` pour
resynchroniser l'autre widget, et un bandeau « Le panier a été modifié ailleurs.
Voici l'état à jour — refais ton geste si besoin. »

Pas de réessai automatique : il appliquerait une action contre un état que le coach
n'a pas vu.

## Ports nécessaires

| Cible | Données |
|---|---|
| `references` | postes du roster (uid, nom, caractéristiques, compétences, prix, `max_quantity`, `is_journeyman`), **limites croisées**, `allowed_staff`, prix du staff, prix de base de relance |
| `players` | effectif courant par ligne de roster, statut de participation |

Les limites croisées ne sont **exposées nulle part** aujourd'hui : ni
`references::TeamDefinition`, ni le port de `team_creation` ne portent le champ.

Le port vers `players` recoupe `IPlayerValuePort` (carte 250) — **l'étendre plutôt
que le doubler**.

## Widgets existants — aucun réutilisable

`team_creation` a un `cart_widget`, un `player_table_widget` et un
`staff_table_widget` qui répondent au même besoin. La règle 1 des conventions widgets
interdit formellement à `teams` de les référencer : **ils sont à réécrire**. On
reprend le pattern et la forme des VMs (`CartVm`, `StaffRowVm`), pas le code.

## Règles métier identifiées à cette étape

- La trésorerie affichée dans l'en-tête est la **trésorerie réelle** ; le panier
  affiche le **reste après achats**. Deux nombres distincts, chacun sous son libellé —
  jamais un seul mot qui changerait de sens.
- L'effectif affiché dans l'en-tête est **projeté** (possédé + en attente), parce que
  c'est lui qui décide si le plafond de 16 est atteint.
- Le prix de relance affiché est le **prix de saison** (double), avec le prix de base
  rappelé en dessous — la valeur d'équipe, elle, comptera la relance à son prix de
  base (carte 250).
- Une ligne du panier est retirable **jusqu'à la validation de phase**, sans coût
  ni trace.

## Points ouverts pour la phase 3

- Où vit le panier : table dédiée `teams__recruitment_baskets`, ou table unique
  partagée avec les renvois et discriminée par phase ?
- Le panier est-il un agrégat du domaine `teams` ou un objet applicatif ?
  Il porte des gardes métier (quotas, limites croisées), ce qui plaide pour le
  domaine — mais il a besoin de données d'un autre BC pour les évaluer.
