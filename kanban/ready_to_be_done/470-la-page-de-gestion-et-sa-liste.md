# La page de gestion des compétences, et sa liste

**Épic :** E10 — Référentiels éditables · **Ordre :** 8 · **Dépend de :** 466, 469
**Conception :** `docs/specs/competences-personnalisees/page-de-gestion/`
(`02-front.md`, `04-dtos.md`, `07-integration.md`)
**Maquette :** `assets/rawpages/html/app-custom-skills.html`

## Objectif

La page, son assemblage, et le widget de liste. Le formulaire vient avec la
carte 471.

## Où la page vit — et le piège évité

L'écran appartient à l'administration d'un espace. **Il ne peut pas être un
onglet de `space-admin.html`** : `spaces` est un BC extractible, et le
`CLAUDE.md` lui interdit de référencer un autre BC, ses routes comprises. Un
onglet « Compétences » obligerait `spaces` à connaître `references` — l'axe 9 de
`check-arch` le refuserait, et avec raison.

**La page est donc autonome**, servie par `references`, sous
`/app/{space_id}/admin/skills`. L'administration d'espace peut y mener par un
lien — un lien sortant est une `String` que l'hôte injecte, pas un import.

## Deux routes

```
references/io/web/skill_admin/
├── mod.rs
├── skill_admin_page.rs   GET /app/{space_id}/admin/skills
└── skill_rows.rs         GET /app/{space_id}/admin/skills/list
```

Ce sont les **premières routes de `references` sous `/app/{space_id}/`**.

## La page hôte ne porte pas de logique

```rust
pub struct CustomSkillsPageTemplate { pub routes: Routes, pub space_id: String }
```

Deux conteneurs `hx-get`, rien d'autre. Aucune donnée de compétence n'y transite.

```html
<div id="cs-form" hx-get="…/skills/form" hx-trigger="load, customSkillSelected from:body"></div>
<div id="cs-list" hx-get="…/skills/list" hx-trigger="load, customSkillsChanged from:body"></div>
```

**Deux événements, dans les deux sens** — c'est ce qui distingue cet écran de
celui des points manuels, où le formulaire n'écoutait rien. Le formulaire écoute
parce qu'il a deux modes, et cliquer « Modifier » dans la liste doit le remplir.

## La liste

```rust
pub struct CustomSkillRowVm {
    pub uid: String, pub name: String,
    pub category_label: String, pub category_css: String,
    pub is_elite: bool, pub activation: String, pub description: String,
    pub usage_count: u32,
}
```

| Usage | Actions |
|---|---|
| zéro | Modifier · **Supprimer** |
| au moins un | Modifier le libellé — et un badge « 🔒 Non supprimable » |

**Le badge dit ce qui est verrouillé**, pas « verrouillée » tout court :
justement, le libellé se modifie.

**Le bouton Supprimer est absent, pas grisé.** Griser inviterait à chercher
comment réactiver alors qu'il n'y a rien à réactiver.

**Aucune section « compétences du règlement ».** Contrairement aux rosters, où
les deux listes cohabitaient : il y en a 43 au corpus de démonstration, bien plus
en production, et les lister ici n'aiderait personne — le sélecteur les montre
déjà là où on les choisit.

**Pas de `can_manage`** : la page entière est réservée à l'administrateur
d'espace, contrairement à la liste des rosters que tout membre consulte. Un
booléen qui vaut toujours `true` invite à croire qu'il peut valoir `false`.

## Le compteur d'usage

`usage_count` vient de `ISkillUsagePort` (carte 466), une fois par ligne. Pour
une poignée de compétences par espace, c'est sans conséquence — et **c'est ce
compteur qui décide des actions affichées**, donc il ne peut pas être approximé.

## CSS

`pages/references-custom-skills.css`, portée par `.cs-page`, **inscrite dans
`src/web/css_bundle.rs`** parmi les pages — l'axe 14 refuse une feuille absente
du bundle.

**Les teintes de catégorie ne sont pas redéfinies** : elles viennent de
`components/skill-tints.css` (carte 469).

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `la_liste_rend_les_competences_de_l_espace` | le cas nominal |
| `la_liste_ignore_celles_d_un_autre_espace` | le cloisonnement |
| `la_liste_ne_rend_aucune_competence_du_corpus` | la décision de la phase 2 |
| `une_competence_portee_n_a_pas_de_bouton_supprimer` | absent, pas grisé |
| `une_competence_libre_a_les_deux_boutons` | le cas passant |
| `un_non_admin_recoit_403` | P1 |

## Checklist

- [ ] Les deux contrôleurs, les deux routes, `routes.rs` et le routeur
- [ ] `references-custom-skills.html` et `references-custom-skill-list.html`
- [ ] `pages/references-custom-skills.css`, **inscrite au bundle**
- [ ] Aucune teinte redéfinie
- [ ] Aucun `style="…"` dans les gabarits
- [ ] Les six tests
- [ ] `make lint && make test && make check-arch`
