# `team_selection_tester.rs` contourne la souveraineté vers `spaces`

**Priorité : basse**
**Dépend de :** —
**Fichier :** `src/app/teams/io/web/widgets/team_selection_tester.rs:25`

## Problème

Jumelle exacte de la carte 296, côté `teams` :

```rust
let spaces = state
    .spaces
    .space_repository
    .find_all()
    .await
    ...
    .map(|s| SpaceDefinition {
        id: SpaceId::try_new(&s.id).expect(""),
        name: SpaceName::try_new(&s.name).expect(""),
    })
```

Découverte en même temps que les autres, à la réparation de l'axe 3 (carte 297).
Tolérée par la ligne de base de l'axe 3 en attendant cette carte.

## Pourquoi une carte séparée de la 296

La 296 a pu être corrigée en dix lignes : `competitions` disposait déjà d'un
port vers `spaces` (`ICompetitionSpaceMemberPort`) et de son adapter, il a suffi
d'y ajouter une méthode.

**`teams` n'a aucun port vers `spaces`** — ni trait dans `ports.rs`, ni adapter
dans `src/infrastructure/teams/`. Le corriger demande d'en créer un de toutes
pièces, pour une page de test développeur et un seul appel. C'est ce
déséquilibre entre le coût et l'enjeu qui justifie de la traiter à part, et
d'en discuter avant de coder.

## Action — à trancher au démarrage

**Créer le port** — cohérent avec la 296, mais un trait, un adapter, une
injection dans `TeamsContext` et une ligne de `main.rs` pour une page de test.

**Supprimer le sélecteur d'espaces de la page de test** — si le testeur de
widgets peut prendre son `space_id` autrement (paramètre d'URL, saisie libre),
la violation disparaît sans port du tout. À regarder en premier : c'est la seule
issue qui ne laisse aucune dette.

**Supprimer la page** — si elle ne sert plus. À vérifier avant tout le reste.

## Note

Le `expect("")` — un panic sans message, sur des données venant de la base —
est à remplacer quelle que soit l'issue retenue, comme il l'a été côté 296.
Sauf si la page disparaît.

## Checklist

- [ ] Utilité de la page vérifiée (est-elle encore utilisée ?)
- [ ] Issue tranchée : port créé, sélecteur supprimé, ou page supprimée
- [ ] Plus aucun `state.spaces` dans `teams`
- [ ] `expect("")` remplacé (si la page survit)
- [ ] Entrée `team_selection_tester` retirée de `AXE3_BASELINE_REGEX`
- [ ] `make check-arch` passe sans elle
