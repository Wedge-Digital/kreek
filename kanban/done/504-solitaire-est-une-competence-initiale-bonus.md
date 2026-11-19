# Solitaire est une compétence initiale bonus

**Priorité : haute — un trait posé que personne ne voit**
**Épic :** E15 — Recruter un journalier (suites)
**Dépend de :** 502, qui a posé le trait au mauvais endroit
**Fichiers :** `src/app/players/io/app_events/player_creation.rs`,
`src/app/players/io/app_events/player_recruited_listener.rs`,
`src/app/players/domain/player.rs`,
`src/app/players/io/repository/player_repository.rs`

## Le défaut

La carte 502 a donné Solitaire (4+) au journalier — en base, c'est vérifiable :
`base_skills = ["LONER_4"]`. **Aucun écran ne l'affiche.**

```rust
// player_table_widget.rs:118
let Some(position) = catalog.find_position(&p.roster_line_id) else { … };
position.base_skills.iter()          // ← les compétences du POSTE
    .filter_map(|uid| catalog.find_skill(uid))
```

Le tableau d'effectif lit les compétences **du poste**, pas celles du joueur.
`p.base_skills` ne lui sert que de repli quand le poste est introuvable au
catalogue.

Le trait a donc été rangé dans le seul champ que l'affichage n'utilise pas.

## Le bon modèle : une compétence initiale bonus

`InitialSkillEarned` existe, `team_created_listener` s'en sert déjà pour les
compétences offertes, et il alimente `acquired_skills` — que le tableau affiche
**par joueur**. Le journal d'évolution le libelle « Compétence initiale bonus ».

**Ce n'est pas un contournement d'affichage, c'est plus juste.** Solitaire n'est
pas une compétence du poste : un Trois-quart embauché ne l'a pas. C'est une
compétence **de ce joueur-là**, qu'il perd en devenant permanent — exactement ce
que `InitialSkillEarned` décrit.

## Le point d'attention : `value_delta` doit valoir zéro

Une compétence offerte augmente la valeur du joueur, et l'événement porte un
`value_delta` pour ça.

**Solitaire ne vaut rien.** C'est un trait, pas un gain, et le prix d'un
journalier **est** sa valeur courante (décision 9). Un delta non nul le
renchérirait sans raison, et la décomposition du prix afficherait « 65 + 20
d'amélioration » pour quelque chose qu'il n'a pas gagné au match.

`spp_cost` vaut zéro pour la même raison : il ne l'a pas acheté.

## Conception

1. **Solitaire devient un `InitialSkillEarned`**, appendu en version 2 juste
   après `PlayerCreated` — le patron que `team_created_listener` suit déjà pour
   les compétences offertes.
2. **`competences_de_naissance` disparaît.** Les compétences de base redeviennent
   celles du poste, ce qu'elles n'auraient jamais dû cesser d'être.
3. **`JourneymanHired` le retire d'`acquired_skills`**, plus de `base_skills` —
   agrégat et projection. La règle du LRB ne change pas : « un Journalier
   embauché perd le Trait Solitaire (X+) ».

## Ce que la carte ne fait pas

**Elle ne touche pas à `build_base_skills`.**

Cette fonction affichant les compétences du poste, **aucun joueur ne peut porter
une compétence de base qui lui soit propre**. Solitaire est le premier cas, et
il se règle mieux autrement — il n'est pas une compétence de poste.

L'élargir sans second cas serait spéculatif. Mais la limite est notée ici : si
un joueur doit un jour porter une compétence de base que son poste n'a pas,
c'est cette fonction qu'il faudra regarder.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `un_journalier_nait_avec_solitaire_en_competence_acquise` | l'événement, pas la base |
| `solitaire_ne_change_pas_la_valeur_du_journalier` | `value_delta` nul — sinon son prix ment |
| `l_embauche_retire_solitaire_des_acquises` | et lui seul |
| **`test_solitaire_s_affiche_sur_la_ligne_du_journalier`** (e2e) | **ce qui manquait** |

Le dernier est le test que la carte 502 aurait dû écrire. Le sien vérifiait la
présence **en base**, jamais **à l'écran** — la même erreur que la 458 avait
faite avec le style, et le même signalement pour la corriger.

## Checklist

- [x] `InitialSkillEarned` à la naissance d'un journalier, `value_delta: 0`
- [x] `competences_de_naissance` retirée — les compétences de base sont celles du poste
- [x] `JourneymanHired` retire la compétence acquise, agrégat **et** projection
- [x] Quatre tests, dont un qui regarde l'écran
- [x] `make lint`, `make check-arch`, `make test` — 1716 tests
- [x] `make e2e` — 367 passés, 7 ignorés

## Ce qui a été fait

### Un `.bind` en trop, laissé par la carte 502

La requête d'embauche portait **trois `bind` pour deux emplacements** — un
reliquat de l'édition qui avait ajouté le retrait de Solitaire au-dessus du
`bind` existant. `cargo check` ne le voit pas : `sqlx::query` (sans macro) ne
compte pas les paramètres à la compilation.

Il n'a jamais fait échouer un test, ce qui est le plus inquiétant : la carte
502 est passée avec, verte de bout en bout.

### La colonne « Amélioration » affiche Solitaire, et c'est l'attendu

Le test e2e attendait « aucune » et a échoué en trouvant « Solitaire (4+) ».
J'ai d'abord filtré le trait, en le prenant pour un défaut — **à tort** : le
coach doit voir ce que le journalier porte avant de décider de le garder. Le
filtre a été retiré, et c'est le test qui a été corrigé.

La leçon est du côté de la conclusion trop rapide : un test rouge dit qu'une
attente et un comportement divergent, pas lequel des deux a tort.

### Ce que la limite laisse ouvert

`build_base_skills` affiche les compétences **du poste**. Aucun joueur ne peut
donc porter une compétence de base qui lui soit propre. Solitaire était le
premier cas ; il se règle mieux en compétence acquise, et cette carte ne touche
pas à la fonction.

Si un second cas apparaît — un joueur dont le poste ne porte pas une de ses
compétences de base —, c'est là qu'il faudra regarder.
