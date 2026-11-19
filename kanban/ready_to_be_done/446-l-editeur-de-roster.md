# L'éditeur de roster

**Épic :** E10 · **Ordre :** 4 · **Dépend de :** 443, 445
**Conception :** `docs/specs/roster-personnalise/editeur-de-roster/`
(`02-front.md`, `04-dtos.md`, `07-integration.md`) · **Maquette :**
`assets/rawpages/html/app-roster-editor.html`

## Objectif

L'écran. C'est le plus gros morceau de l'épic.

## Conception

### Les cinq routes qui restent

```
GET    …/admin/rosters/new
GET    …/admin/rosters/{roster_uid}
POST   …/admin/rosters
PUT    …/admin/rosters/{roster_uid}
DELETE …/admin/rosters/{roster_uid}
```

**Aucun identifiant de ressource hors du chemin.** C'est la leçon de la carte
416 : `delete_match` et ses voisins prennent leur cible dans le corps, hors de
portée de `space_scope`. Ici `{roster_uid}` est dans l'URL, donc couvert par le
résolveur de la carte 445.

`is_space_admin` en première ligne des trois mutations.

### Les sorties

| Cas | Réponse |
|---|---|
| POST/PUT réussi | `HX-Redirect` vers la liste |
| `Invalid(DomainError)` | `422` + la page, l'erreur **nommant le poste fautif** |
| `InUse { teams }` | `409` + le message **portant le nombre** |
| `Forbidden` / `NotFound` | `403` / `404` |
| `UsageUnavailable` | `503` |

**`409` et non `422` pour `InUse`** : la requête est bien formée, c'est l'état du
système qui s'y oppose. **`503` pour `UsageUnavailable`** : le refus est
temporaire, réessayer a du sens — ce qu'un `500` ne dit pas.

### Les trois modes

```rust
pub enum EditorMode {
    Create,
    Edit,
    ReadOnly { teams_using: u32 },
}
```

Une énumération et non deux booléens `can_edit` / `can_delete` : les trois états
sont exclusifs, et deux booléens en autorisent quatre — dont un qui n'existe
pas.

**`ReadOnly` n'est pas un formulaire désactivé, c'est une fiche.** Griser cent
champs donne un écran illisible là où une fiche se lit d'un coup. Et le bandeau
**nomme la cause** — « 3 équipes de cet espace jouent ce roster » — parce qu'un
écran qui dit « non modifiable » sans dire pourquoi envoie chercher.

**Le bouton Supprimer n'existe pas dans ce mode**, il n'est pas grisé.

### Les catalogues, rendus une fois avec la page

146 compétences, 38 mots-clefs, les catégories, le staff, les règles spéciales.

**Les compétences viennent de `list_skills_for_space`, pas de `list_skills`**
(carte 465). Un poste de roster d'espace doit pouvoir poser une compétence
personnalisée de ce même espace — sans quoi les deux fonctionnalités se livrent
le même jour en s'ignorant, et c'est pourtant leur emploi le plus évident
ensemble.
Quelques dizaines de kilooctets, contre un aller-retour par touche pour filtrer
une liste qu'on tient déjà.

```rust
pub struct SkillOptionVm { uid, name, category, is_elite, description }
pub struct KeywordOptionVm { uid, label, is_species, hate_note }
pub struct TraitOptionVm { uid: Option<String>, name, family: Option<TraitFamily>, description }
```

**`is_species` est dérivé, pas lu tel quel.** Le corpus porte
`league_hate_selectable` ; les deux coïncident pour 37 mots-clefs sur 38.
**`BIG_GUY` est l'exception** — `false`, et pourtant un `hate_skill_uid` non nul.
C'est un rôle, et `hate_note` doit le dire au lieu de laisser croire à une
incohérence.

**Les quatre familles sont repliées au builder**, pas au gabarit : `HATRED_*`
compte 31 entrées, 42 % des traits. Et **le gabarit nu du corpus est écarté** —
`LONER` sans nombre existe à côté de `LONER_3` et `LONER_4` : c'est un modèle de
rédaction, pas un trait attribuable.

### Le JS n'est pas un `<script>` inline

Trois sélecteurs, un état à une centaine de champs, un pied de cohérence dérivé.
C'est un composant Alpine dans un fichier servi, comme `kreek-select.js`.

Ce qu'il tient :

| Front | Serveur |
|---|---|
| ajouter, dupliquer, retirer un poste | les catalogues, au chargement |
| déplier l'éditeur d'un poste | le roster complet, au clic sur Enregistrer |
| chercher et filtrer dans les trois sélecteurs | |
| le pied de cohérence | |

**Le pied avertit, il n'autorise pas** : le serveur refait tout.

### CSS

`pages/references-roster-editor.css`, portée par `.re-page`, inscrite dans
`css_bundle.rs`.

## Tests

Unitaires, sur les builders :

| Test | Ce qu'il prouve |
|---|---|
| `les_familles_de_traits_sont_repliees` | une entrée « Haine (…) », pas 31 |
| `le_gabarit_nu_est_ecarte` | `LONER` n'est pas proposé, `LONER_3` oui |
| `big_guy_est_un_role_avec_une_note` | l'exception du corpus |
| `les_quatre_competences_elite_sont_marquees` | `is_elite` |

Les tests de navigateur sont la carte 447.

## Checklist

- [ ] Les cinq routes, `is_space_admin` sur les trois mutations
- [ ] `EditorMode` et ses trois rendus
- [ ] Les VMs et les builders, familles repliées
- [ ] Le gabarit d'après la maquette
- [ ] Le composant Alpine, en fichier servi
- [ ] La feuille + le bundle
- [ ] Les quatre tests
- [ ] `make lint && make test && make check-arch`
