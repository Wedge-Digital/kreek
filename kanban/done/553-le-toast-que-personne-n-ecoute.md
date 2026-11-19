# Le toast que personne n'écoute

**Priorité : haute — préalable aux cartes 551 et 552, qui signalent des refus**
**Épic :** aucune — un composant, livrable d'un bloc
**Dépend de :** rien
**Fichiers :**
`src/web/templates/app-layout.html`,
`assets/static/css/components/toast.css` *(nouveau)*,
`src/web/css_bundle.rs`,
`src/app/competitions/io/web/templates/admin/schedule.html`,
`tests/e2e/test_pairing_deletion.py`, `tests/impact-map.toml`

## L'objectif

Un composant de notification **global**, qui écoute `showToast` et remplace les
`alert()` natifs de l'administration du calendrier.

## Ce qui l'a fait naître

Trois handlers de `team_creation` répondent déjà :

```rust
r#"{"showToast":"Équipe soumise avec succès !"}"#
```

**Personne n'écoute.** Aucun écouteur de `showToast` dans `src/web/`, aucun dans
`assets/static/js/`, rien dans le layout. Un coach qui soumet son équipe ne voit
aucune confirmation, et cela depuis que ces trois en-têtes existent.

C'est la **forme B** que le `CLAUDE.md` décrit à propos des app events — un
événement émis, sans bras pour le recevoir — transposée au navigateur. Elle a le
même symptôme : tout a l'air correct dans le code émetteur, et rien n'arrive.

L'autre moitié est l'inverse : `handleScheduleActionResponse`
(`admin/schedule.html`) affiche ses quatre messages par `alert()`. Ça marche,
mais ça bloque la page, ça ne ressemble à rien du reste de l'interface, et ça
oblige chaque test e2e qui passe par là à capter un dialogue natif.

## Les trois pièces

**1 · Le composant, dans le layout.** Un conteneur en fin de `app-layout.html`,
piloté par Alpine, sur le modèle éprouvé du toast d'`actions-step.html` — dont
le `x-transition` et les classes d'entrée/sortie sont à reprendre.

**2 · L'écouteur.** `document.body` écoute `showToast`. Les trois émissions de
`team_creation` deviennent visibles **sans toucher une ligne de Rust** — c'est la
mesure de ce qui manquait.

**3 · `handleScheduleActionResponse` appelle le toast** au lieu d'`alert()`. Ses
quatre messages y passent : refus 4xx, équipes ignorées, journées non
régénérées, rencontres conservées.

## Les deux régimes — le point à ne pas rater

Un toast qui s'efface au bout de deux secondes et demie est **moins bon qu'un
`alert()` pour un refus** : l'admin clique, regarde ailleurs, et l'information
est perdue sans que rien ne se soit passé à l'écran.

| régime | comportement |
|---|---|
| succès | s'efface seul après ~2,5 s |
| erreur | **reste jusqu'au clic**, avec une fermeture explicite |

Sans cette distinction, on dégrade le signalement d'erreur en croyant
l'embellir, et la régression ne se voit qu'en production — quand quelqu'un dit
« j'ai cliqué et il ne s'est rien passé ».

## Le CSS

`assets/static/css/components/toast.css`, **inscrit dans `css_bundle.rs`** parmi
les `components/` — l'axe 14 de `check-arch` refuse toute feuille absente du
bundle. `components/` et non `widgets/` : le toast n'est pas exposé par un BC, il
appartient au layout.

Les valeurs partent de `.mr-toast` (`pages/match-report-shared.css`) : position
fixe en bas à droite, `--green` pour le succès, ombre portée, transition de 0,2 s.
La variante d'erreur s'ajoute, et `pointer-events` redevient actif pour elle —
un toast qu'on doit fermer doit pouvoir être cliqué.

## Ce que la carte ne touche pas

**Le toast local d'`actions-step.html`.** Il fonctionne, il est scopé à sa page,
et l'aligner sur le composant global est un autre chantier. Le faire ici
mêlerait une correction à une uniformisation, et obligerait à re-tester la
saisie d'actions pour un changement qui ne la concerne pas.

Les deux `alert()` restants vivent dans des pages de test de widgets
(`competitions-widget-tester-page.html`, `team-selection-tester.html`). Ils n'y
gênent personne.

## Tests

**E2E**

| | |
|---|---|
| le refus du planning | `test_pairing_deletion.py` : le `page.on("dialog")` cède la place à une attente sur le toast — plus simple et plus robuste que la boucle `wait_for_timeout` que le dialogue imposait |
| l'erreur persiste | le toast d'erreur est encore visible après 4 s |
| le succès s'efface | le toast de succès a disparu après 4 s |

Le test de la soumission d'équipe, s'il en existe un, gagne une assertion : la
confirmation s'affiche enfin.

**À ne pas oublier** : l'axe 14 de `check-arch` sur `toast.css`, et
`tests/impact-map.toml` si un test e2e change de périmètre.

## Terminé quand

Un coach qui soumet son équipe voit une confirmation à l'écran. Un commissaire
dont l'action de planification est refusée voit un toast d'erreur **qui reste**
jusqu'à ce qu'il le ferme. Et plus aucun `alert()` ne subsiste hors des pages de
test de widgets.
