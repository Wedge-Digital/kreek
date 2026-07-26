# Phase 8 — Cartes kanban (`detailed-standings`)

Cinq cartes, numérotées 220 à 224. Dernière phase de conception : l'implémentation suit,
carte par carte, sous le régime normal du CLAUDE.md.

## Ordre et dépendances

```
220 (domaine)  ─┐
                ├─► 223 (mise en évidence) ──► 224 (E2E)
221 (refactos) ─┴─► 222 (onglet) ───────────┘
```

| # | Carte | Dépend de | Nature |
|---|---|---|---|
| **220** | Résolution du critère décisif dans le domaine | — | additive, aucun appelant |
| **221** | Refactos préparatoires | — | refacto pure, zéro changement de comportement |
| **222** | Onglet complet, sans mise en évidence | 221 | rend l'onglet livrable |
| **223** | Mise en évidence du critère décisif | 220 + 222 | **reportable** |
| **224** | Tests E2E | 223 | — |

**220 et 221 sont parallélisables** : aucune ne dépend de l'autre.

## Justification du découpage

### Pourquoi 220 avant tout

`tiebreak_outcomes` est la seule vraie logique métier de l'unité. Livrée seule et sans
appelant, elle se teste intégralement dans le domaine, et le **test de sous-groupe**
(`+5 / +2 / +2`) qui distingue la version révisée de R21 de la formulation initiale peut
être écrit puis vu échouer sur la variante naïve — ce qui serait impossible à faire
proprement si le câblage arrivait dans le même diff.

### Pourquoi 221 est isolée

Deux extractions indépendantes réunies par leur signature commune : préparer le terrain
sans rien changer. Leur critère d'acceptation — **les tests existants restent verts sans
être modifiés** — n'a de sens que dans un commit qui ne contient rien d'autre. Mêlées à la
222, on ne saurait plus si un test retouché l'a été par nécessité ou par commodité.

### Pourquoi 222 exclut la mise en évidence

L'onglet sans couleurs affiche déjà chaque nombre composant le total et les compteurs de
départage dans l'ordre : il est utile en l'état. Cela permet à la 223 d'être réellement
reportable, ce qui était la condition posée en phase 2 pour garder R21 et R22 au périmètre.

Le champ `CellState` est introduit **dès la 222**, avec `Neutral` partout. La 223 se réduit
alors à le peupler et à ajouter deux classes CSS ; l'introduire en 223 la ferait traverser
VM, builder, template et CSS d'un coup, à rebours de l'objectif d'une carte reportable.

Conséquence : **la légende est scindée**. La 222 n'explique que Bonus et Total ; les
phrases sur la mise en évidence et l'ex æquo arrivent avec la 223, faute de quoi l'onglet
décrirait des couleurs qui n'existent pas encore.

### Pourquoi la CSS n'a pas sa carte

Découpée séparément, elle produirait une carte ni testable ni livrable seule. Les classes
`.sd-*` vont dans la 222, les deux classes de mise en évidence dans la 223.

### Pourquoi 224 exige une vérification par mutation

La carte 219 a livré un test E2E vert et inutile : il passait aussi avec le départage
neutralisé. La checklist de la 224 impose donc de **voir échouer** le scénario du critère
décisif avant de le commiter, et rappelle que le redémarrage effectif du processus doit
être vérifié — la seule comparaison des dates de fichiers avait induit en erreur.

## Ce que les cartes portent en propre

Chaque carte reprend les pièges déjà payés ailleurs plutôt que de les laisser à la
mémoire :

| Carte | Piège consigné |
|---|---|
| 220 | La résolution à plat désigne un critère décisif sur des lignes qu'il n'a pas départagées |
| 221 | Rappel de la règle 5 du CLAUDE.md : déplacer du code, c'est copier-coller, pas réécrire |
| 222 | `build_vm` sous 20 lignes ; signe moins typographique ; trophée dans la cellule équipe |
| 223 | `css_class()` seul point de correspondance ; résolution **par poule** |
| 224 | Bonus cochés par défaut ; décochage sans glisser-déposer ; pas de `reset_db` |
