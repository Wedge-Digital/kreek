# La disponibilité des star players rejoint le domaine

**Priorité : moyenne**
**Épic :** E16 — Imprimer la feuille d'équipe · PR 2
**Dépend de :** rien
**Spec :** `06-domaine.md` (`star_player_availability.rs`, R13)
**Fichiers :** `src/app/references/domain/star_player_availability.rs`, `src/app/references/domain/mod.rs`, `src/app/references/io/web/inducement_selector_controller.rs`

## Objectif

« Ce star player est-il disponible pour ce roster ? » n'est répondu que par
`build_star_player_items`, en privé dans un contrôleur
(`inducement_selector_controller.rs:183-188`). L'impression en a besoin : la
règle doit vivre au domaine, à côté de `est_disponible_pour`.

## Conception

- `star_player_available_for(star, roster_id) -> bool` : liste
  `available_for_rosters` vide, ou contenant le roster.
- **Déplacé par copier-coller** (règle 5) ; le contrôleur l'appelle.
- Hors périmètre : le serveur n'applique toujours pas ce filtre à
  l'engagement d'un star player (`build_star_player_spec`).

## Checklist

- [ ] Fonction + tests : liste vide · roster présent · roster absent
- [ ] Le contrôleur l'appelle ; plus aucune copie de la condition (vérifier les consommateurs, règle 4)
- [ ] Tests existants du sélecteur verts
- [ ] `make check-arch`, `make lint`, `make test`
