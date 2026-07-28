# Page et widgets de recrutement

**Priorité : haute**
**Dépend de :** 263
**Bloque :** 266
**Spec :** `recrutement/02-front.md`, `04-dtos.md` §5, `07-integration.md` §3-4
**Maquette validée :** `assets/rawpages/html/app-team-recruitment.html`
**Fichiers :** `src/app/teams/io/web/recruitment.rs`,
`io/web/widgets/recruitment_catalog_widget.rs`,
`io/web/widgets/recruitment_cart_widget.rs`, `io/web/view_models.rs`,
`templates/recruitment.html`, `templates/widgets/recruitment-catalog.html`,
`recruitment-cart.html`, `draft-error.html`, `routes.rs`, `router.rs`

## Problème

La bannière de phase promet « Achetez des joueurs ou du staff » et n'offre qu'un bouton
« Terminer les achats ». Aucune interface d'achat n'existe.

## Action

### 1. Deux widgets, un événement

| Widget | Endpoint | Trigger |
|---|---|---|
| `recruitment_catalog` | `GET …/team/widgets/recruitment-catalog` | `load, draftChanged from:body` |
| `recruitment_cart` | `GET …/team/widgets/recruitment-cart` | `load, draftChanged from:body` |

Le catalogue porte **les deux tableaux et la composition de l'effectif** : même
colonne, même brouillon, les séparer multiplierait les requêtes sans bénéfice.

Ajouter un joueur ne change pas que sa ligne — ça peut désactiver **toutes** les autres.
Le tableau se rafraîchit donc entier, jamais ligne par ligne.

`draftChanged` est émis par chaque mutation via `HX-Trigger`, sur le modèle de
`teamMutated` (`team_creation/io/web/widgets/staff_table_widget.rs:101`). Une mutation
coûte **1 POST + 1 GET**.

### 2. Quatre routes de mutation

`…/recruitment/players/add`, `players/remove`, `staff/add`, `staff/remove`.
Réponse : fragment du widget cliqué + `HX-Trigger: draftChanged`.

### 3. La version voyage dans `hx-vals`

```html
<button hx-post="{{ routes.add_player(space_id, team_id) }}"
        hx-vals='{"roster_line_id": "{{ line.uid }}", "version": {{ draft.version }}}'>
  Recruter
</button>
```

Pas de `hx-include` vers un champ caché ailleurs : la règle 4 des conventions widgets
l'interdit, et `hx-disinherit="*"` impose que le widget se suffise à lui-même. **Les
deux widgets** rendent la version, tous deux portant des boutons.

### 4. `ConcurrentWrite` répond 200

Le geste n'est pas appliqué, mais l'utilisateur reçoit une **page cohérente** :
fragment reconstruit depuis le brouillon à jour, `HX-Trigger: draftChanged`, et un
bandeau « Le panier a été modifié ailleurs. Voici l'état à jour — refais ton geste si
besoin. » Pas de réessai automatique.

C'est la première fois dans ce projet qu'un `ConcurrentWrite` remonte jusqu'à
l'interface : le fragment d'erreur est à concevoir.

### 5. VMs — aucun `builders.rs`

Le brouillon portant ses données après hydratation, **toutes les VMs se construisent
depuis des types domaine** : `from_domain()` co-localisés dans `view_models.rs`. Aucun
fichier `builders.rs` n'est nécessaire.

`ActionVm` reprend `ActionState` du domaine : la couche web **formule** la raison du
blocage, elle ne la calcule pas.

### 6. Conventions de la maquette à respecter

`hx-disinherit="*"` sur la racine de chaque widget, CSS embarqué, aucun `style=`
inline, `kreek-select` si un sélecteur apparaît. Le seul JavaScript est le repli du
panier sous 768px, en `x-data` Alpine avec `init()`/`destroy()`.

Bascule mobile au breakpoint **768px**, chrome repris de `layout-app.css`, barre du bas
calée à `bottom: 57px`, cibles tactiles à **44px minimum** — y compris les `×` du
panier, dont la zone de clic est étendue par pseudo-élément.

### 7. Bannière de la fiche équipe

Le bouton « Recruter → » pointe vers la nouvelle page au lieu du placeholder.

## Checklist

- [ ] Deux widgets, `draftChanged` unique, 1 POST + 1 GET par mutation
- [ ] Version cuite dans `hx-vals` des deux widgets
- [ ] `ConcurrentWrite` → 200 + fragment à jour + bandeau
- [ ] Aucun `builders.rs`, tous les VMs en `from_domain()`
- [ ] Raison du blocage affichée sur chaque bouton désactivé
- [ ] Responsive 768px, cibles 44px, panier fixe repliable
- [ ] Aucun `style=` inline, `hx-disinherit="*"` posé
- [ ] Bannière de la fiche équipe recâblée
- [ ] `make check-arch` au vert, `make test` au vert
