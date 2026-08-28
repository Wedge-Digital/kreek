# La teinte de catégorie devient un composant

**Épic :** E10 — Référentiels éditables · **Ordre :** 7 · **Dépend de :** rien
**Conception :** `docs/specs/competences-personnalisees/page-de-gestion/`
(`04-dtos.md`, `07-integration.md`)

## Objectif

Une seule définition de « quelle couleur porte cette catégorie », des deux côtés
— Rust et CSS. Sans elle, la page des compétences en créerait une seconde.

## Ce qui existe, et ce qu'il raconte

```rust
// players/io/app_events/team_created_listener.rs:49
pub fn skill_category_css(category: &str) -> &'static str {
    match category {
        "GENERAL" => "type-general",
        …
        "MUTATIONS" => "type-mutation",
        _ => "type-general",
    }
}
```

Son propre commentaire documente sa dérive passée :

> `MUTATION` au singulier quand le corpus dit `MUTATIONS`, si bien que mutations
> et retors portaient la couleur du général. Personne ne l'avait vu : **une
> couleur fausse ne casse rien, elle ment simplement.**

Et la classe **est figée à l'écriture**, dans `players_proj.acquired_skills` —
pas résolue à l'affichage. Une seconde table fausse ne se corrigerait donc pas en
la corrigeant : les compétences déjà acquises garderaient leur teinte erronée,
comme les anciennes l'ont gardée.

**`references` n'a pas le droit d'importer `players`.** La duplication est donc
la voie par défaut — et c'est exactement celle qu'il faut fermer.

## Côté Rust — `references` devient propriétaire

```rust
pub struct SkillCatalogEntryDto {
    …,
    pub category_label: String,
    pub category_css:   String,   // ← neuf, résolu à la ligne d'à côté
}
```

`skill_catalog_adapter.rs` résout déjà `category_label` par
`list_skill_categories()`. **`category_css` y est chez lui**, et `players` n'a
plus de table du tout : le listener lit le DTO.

La fonction `skill_category_css` **se déplace** dans `references` (règle 5 du
`CLAUDE.md` : copier-coller, pas réécriture) — son `MUTATIONS` compris.

## Côté CSS — la teinte devient globale

Les sept teintes vivent dans `widgets/players-widget.css:34-41`. La page des
compétences les emploie sans les redéfinir.

**Mais `tests/e2e/visual/debordements.py` pose exactement cette question** :

> ce sélecteur trouve-t-il du markup sur une page qui ne chargeait pas sa
> feuille ?

`widgets/players-widget.css` trouvera `.type-agility` sur la page des
compétences. **Le contrôle signalera un débordement, et il aura raison.**

La sortie n'est pas une exception : les sept teintes montent dans
`components/skill-tints.css`. Les feuilles de `components/` sont **globales par
construction** pour ce contrôle — le débordement disparaît de lui-même, et les
deux pages consomment la même définition.

## Pourquoi les deux moitiés dans la même carte

C'est **une seule décision appliquée à deux couches** : `references` possède les
catégories, donc il possède leur teinte, en Rust comme en CSS. Les séparer
laisserait la moitié du chemin faite — et c'est la moitié restante qui dérive.

## Ce que la carte ne fait pas

**Elle ne repeint pas le passé.** Les compétences déjà acquises gardent la classe
figée à leur écriture. Le corriger demanderait un rattrapage de projection, qui
n'est pas le sujet ici — et le `MUTATIONS` est déjà juste depuis sa correction.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `aucune_categorie_du_corpus_ne_retombe_sur_le_repli` | **le test qui suit la fonction** |
| `le_dto_porte_la_classe_a_cote_du_libelle` | les deux résolus ensemble |
| `mutations_au_pluriel_donne_type_mutation` | la dérive nommée, fixée |
| `une_categorie_inconnue_retombe_sur_type_general` | le repli reste |

`aucune_categorie_du_corpus_ne_retombe_sur_le_repli` relie la table au corpus
plutôt qu'à une liste écrite de mémoire — c'est lui qui aurait attrapé
`MUTATIONS`. **Il se déplace avec la fonction.**

Le contrôle visuel : `uv run python visual/debordements.py` doit rester muet sur
la page des compétences.

## Checklist

- [ ] `skill_category_css` déplacée dans `references` par **copier-coller**
- [ ] `category_css` sur `SkillCatalogEntryDto`, résolu par l'adapter
- [ ] `players` ne porte plus de table
- [ ] `components/skill-tints.css`, inscrite au bundle
- [ ] Les sept teintes retirées de `widgets/players-widget.css`
- [ ] Le test de corpus déplacé avec la fonction
- [ ] `debordements.py` muet
- [ ] `make lint && make test && make check-arch`
