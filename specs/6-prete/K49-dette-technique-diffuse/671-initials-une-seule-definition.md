---
id: 671-initials-une-seule-definition
parent: K49
status: approved
---

# Fonction `initials()` recopiée dans treize fichiers

**Priorité : faible**
**Dépend de** : —
**Fichier partagé :** `src/common/initials.rs`
**Remplace :** la carte 11 (`11-initials-dupliquee`), écrite quand la fonction n'existait
qu'en deux copies (`competition_detail.rs`, `coach_search.rs`)

## Problème

La première partie de la carte 11 est faite : `src/common/initials.rs` expose
`pub fn initials(name: &str) -> String`, que `competition_detail.rs`, `coach_search.rs` et
d'autres modules importent.

Mais la même fonction privée a été recopiée depuis, à l'identique, dans treize fichiers,
sous deux noms :

```rust
fn initials(name: &str) -> String {          // ou initials_from
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}
```

`initials` :

- `src/app/team_creation/io/web/my_teams.rs`
- `src/app/competitions/io/web/admin/groups_widgets.rs`
- `src/app/teams/io/web/widgets/competition_teams_widget.rs`
- `src/app/teams/io/web/widgets/pending_enrollment_widget.rs`
- `src/app/teams/io/web/widgets/enrolled_teams_widget.rs`
- `src/app/teams/io/web/widgets/my_teams_widget.rs`

`initials_from` :

- `src/app/match_report/io/web/builders.rs`
- `src/app/match_report/io/web/actions_step_controller.rs`
- `src/app/match_report/io/web/live_match_controller.rs`
- `src/app/match_report/io/web/mvp_step_controller.rs`
- `src/app/match_report/io/web/pre_match_controller.rs`
- `src/app/match_report/io/web/step5_controller.rs`
- `src/app/match_report/io/web/team_step5_controller.rs`

Toute évolution (tirets, accents) devra être faite quatorze fois, et il est facile d'en
oublier une.

## Action

Supprimer ces copies et importer `crate::common::initials::initials`, comme le font déjà
les modules de `spaces` et de `competitions`.

## Hors périmètre

La copie `initials_from` du domaine (`src/app/teams/domain/team.rs`, avec ses tests) :
l'importer depuis `crate::common` ferait dépendre le domaine d'un module hors du noyau,
ce que la pureté du domaine vérifiée par `make arch` peut refuser. Elle reste en place
tant que ce point n'est pas tranché.

## Terminé quand

- `grep -rn "fn initials" src/app --include='*.rs'` ne trouve plus que
  `src/app/teams/domain/team.rs`.
- `make test`, `make lint`, `make arch` et `make dup` passent.
