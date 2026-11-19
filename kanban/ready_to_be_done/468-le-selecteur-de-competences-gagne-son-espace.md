# Le sélecteur de compétences gagne son espace

**Épic :** E10 — Référentiels éditables · **Ordre :** 6 · **Dépend de :** 465
**Conception :** `docs/specs/competences-personnalisees/page-de-gestion/03-back.md`

## Objectif

Le sélecteur qu'un coach ouvre pour dépenser ses SPP doit montrer les compétences
de son espace. Aujourd'hui il ne sait pas où il est.

## Le problème

```rust
// references/routes.rs
pub const SKILL_PICKER: &str = "/references/roster-lines/skill-picker";
```

Pas de `{space_id}`, pas de `Query` qui en porte un. Et c'est pourtant l'écran
qui doit montrer les compétences personnalisées.

**C'est l'inverse du cas des rosters** : là-bas `find_team_by_uid` résolvait par
identifiant et le préfixe décidait ; `list_teams()` n'avait besoin que du corpus.
Ici `list_skills()` sert le sélecteur.

## Le changement

```
/references/roster-lines/skill-picker
→ /app/{space_id}/references/roster-lines/skill-picker
```

```rust
pub async fn skill_picker(
    Path(space_id): Path<String>,          // ← neuf
    Query(params): Query<SkillPickerParams>,
    State(state): State<AppState>,
) -> impl IntoResponse
```

Ligne 158 : `repo.list_skills()` devient `repo.list_skills_for_space(&space_id)`.

## Un seul consommateur

`team_creation/io/web/templates/finalize-team.html:81`, via
`app_routes.references.skill_picker_base()`. Le gabarit connaît son `space_id` —
le changement est mécanique.

`Routes::skill_picker_base()` devient `Routes::skill_picker(space_id: &str)`.

## Ce que ça apporte au-delà du besoin

Le sélecteur entre sous `space_scope`, alors qu'il sert aujourd'hui le catalogue
à qui le demande, sans notion d'espace. **C'est un gain en soi**, indépendant des
compétences personnalisées.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `le_selecteur_rend_les_competences_du_corpus` | rien n'a régressé |
| `le_selecteur_rend_aussi_celles_de_l_espace` | l'objet de la carte |
| `le_selecteur_ignore_celles_d_un_autre_espace` | le cloisonnement |
| `le_filtre_par_acces_s_applique_aux_deux` | une compétence d'espace en catégorie secondaire reste secondaire |

Le dernier compte : le filtre d'accès (`accessible.contains(s.category)`) ne
distingue pas l'origine, et il ne doit pas se mettre à le faire.

## Checklist

- [ ] La route et son `Path`
- [ ] `list_skills_for_space` à la ligne 158
- [ ] `Routes::skill_picker(space_id)`
- [ ] `finalize-team.html:81` adapté
- [ ] La route déplacée sous le groupe scopé du routeur
- [ ] Les quatre tests
- [ ] `make lint && make test && make check-arch`
