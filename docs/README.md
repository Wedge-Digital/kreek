# docs/

Ce dossier décrit **l'application** : sa forme d'ensemble, ses spécifications,
et le format de ses données de référence.

Il ne porte ni les règles de travail — elles vivent dans `CLAUDE.md` à la
racine, et nulle part ailleurs — ni le backlog, qui vit dans `kanban/`.

---

## Index

| | |
|---|---|
| [`atlas-des-contextes.html`](atlas-des-contextes.html) | Les onze bounded contexts en planches : les blocs de chaque couche, les ports, les app events, les tables. Page autonome, à ouvrir dans un navigateur. |
| [`reference-data-schema.md`](reference-data-schema.md) | Le format attendu des JSON du catalogue Blood Bowl (`teams_fr.json`, `skills_fr.json`, `inducements_fr.json`, …) et les règles de cohérence référentielle entre eux. |
| [`specs/`](specs/) | Une spécification par fonctionnalité — 29 à ce jour, 43 pages, 315 fichiers. |

---

## `specs/` — une fonctionnalité par dossier, une page par sous-dossier

Ces fichiers sont produits par le workflow « Nouvelle fonctionnalité », dont
**`.claude/workflows/new-feature.md` est la référence** : ce qui suit n'en est
qu'un repère de lecture, et c'est lui qui fait foi si les deux divergent.

Le numéro d'un fichier **est** sa phase, donc son ordre de lecture :

```
specs/<fonctionnalité>/
├── README.md              vue d'ensemble de la fonctionnalité
└── <page>/
    ├── 01-mockup.md       phase 1 — design
    ├── 02-front.md        phase 2 — architecture front
    ├── 03-back.md         phase 3 — architecture back
    ├── 04-dtos.md         phase 4 — contrats de données
    ├── 05-use-cases.md    phase 5 — use cases
    ├── 06-domaine.md      phase 6 — logique métier
    ├── 07-integration.md  phase 7 — persistance, événements, réponses
    └── 08-cards.md        phase 8 — cartes kanban
```

**Toutes les phases ne sont pas toujours là**, et c'est normal : une page sans
logique métier propre n'a pas de `06-domaine.md`, une page sans use case n'a
pas de `05-use-cases.md`. Seul `08-cards.md` est présent partout — c'est la
phase qui produit le travail. Deux fonctionnalités anciennes condensent les
phases 2 à 7 dans un `02-07-conception.md`, une troisième dans un
`00-conception.md` : ce sont des vestiges, pas un second format à suivre.

---

## L'atlas est daté

`atlas-des-contextes.html` porte un relevé : **9 septembre 2026, branche
`demo`, commit `4c7b635`**. Les chiffres qu'il affiche — lignes, routes,
tables, ports, app events, listeners — décrivent le dépôt à cette date et **ne
se mettent pas à jour tout seuls**.

C'est le défaut de ce genre de page, et il vaut mieux le nommer que l'oublier :
une planche d'architecture qui ne dit pas sa date se lit comme si elle était
vraie aujourd'hui. Le cartouche en tête de page et le pied de page répètent
donc tous deux le relevé.

Trois des écarts qu'il signale sont des dettes réelles au moment du relevé
(deux franchissements de souveraineté en SQL, une émission d'app event hors
publisher) ; les vérifier avant de s'en servir comme argument.

Le remettre à jour se fait par `grep` sur `src/app/` et `src/infrastructure/`,
et par les commandes de recensement citées dans `CLAUDE.md`. Aucun outil ne le
régénère : c'est un document écrit, pas une sortie de build.
