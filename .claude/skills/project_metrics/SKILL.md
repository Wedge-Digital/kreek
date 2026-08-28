---
name: project_metrics
description: >
  Mesure la vélocité d'un dépôt dans le temps — cadence des commits, coût d'un
  commit, taille des commits, et répartition du temps entre production de code,
  tests, outillage et écrit — puis publie le résultat en artefact. À utiliser
  quand l'utilisateur demande d'analyser la vélocité, la deliverabilité, l'effet
  de la taille du projet sur le rythme, où est passé le temps, ou toute autre
  lecture de l'historique git dans la durée.
argument-hint: "[--since '12 months ago'] [--cut N] [--repo .]"
---

# Vélocité — mesurer ce que la taille du projet coûte

## Ce que le skill produit

1. `velocity.json` + un rapport lisible, par `scripts/extract.py`.
2. Un **artefact web** publié, avec les courbes et le tableau complet.

Le script fait toute l'extraction. Ne jamais recalculer ces métriques à la main
dans le terminal : c'est long, et chaque reconstruction réintroduit les pièges
listés plus bas.

## Marche à suivre

```bash
python3 .claude/skills/project_metrics/scripts/extract.py \
        --repo . --since "2026-04-20" --out <scratchpad>/velocity.json
```

`--repo` désigne la racine du dépôt (défaut : le répertoire courant).
`--since` borne l'historique — la resserrer sur la période réellement active :
un prototype abandonné dix-huit mois plus tôt fausse toutes les échelles.
`--cut N` déplace la frontière entre les deux plateaux comparés (par défaut :
moitié des semaines pleines). Le rapport part sur stderr, le JSON dans `--out`
(à écrire dans le scratchpad, jamais dans le dépôt).

Puis lire le JSON, écrire l'artefact (voir « Forme de l'artefact »), le publier,
et **rendre la conclusion en clair dans le terminal** — l'artefact est le
support, pas la réponse.

## Les métriques, et ce qu'elles disent

| Champ | Lecture |
|---|---|
| `min_per_commit` | ce que coûte un commit. La mesure centrale. |
| `commits_per_hour` | son inverse : la cadence. |
| `lines_per_commit` | médiane des lignes ajoutées. **Indispensable** : sans elle, un commit plus lent peut n'être qu'un commit plus gros. |
| `min_per_100_lines` | le coût rapporté à la ligne. C'est *lui* qui tranche entre ralentissement et regroupement. |
| `hours` | heures actives, reconstruites par sessions. |
| `kloc` / `prod_kloc` / `test_kloc` | masse du code, tests unitaires séparés. |
| `time.*` | heures de la semaine réparties par catégorie de fichiers. |
| `fit` | régression `min/commit ≈ a + b·log2(kLOC)`, avec `r` et `R²`. |

La courbe attendue est **logarithmique, pas linéaire** : le coût monte, de moins
en moins vite. `b` se lit « minutes ajoutées par commit à chaque doublement de
la base de code ».

## Les pièges — tous rencontrés, aucun théorique

**Les commits de documentation ne sont pas des commits de code.** Un dépôt qui
range ses cartes kanban dans git a 30 % de commits qui ne portent aucune ligne
de code ; les compter écrase la cadence. Le script les écarte du rythme, mais
les garde pour la répartition du temps — écrire une carte prend du temps aussi.

**Compter par jour ne montre rien.** Les commits par jour calendaire restent
plats alors que la cadence s'effondre : la journée de travail s'allonge en même
temps. Il faut des **sessions** — une suite de commits espacés de moins de
90 minutes — et un forfait d'amorce pour le premier de chaque session.

**Un ratio s'agrège, il ne se moyenne pas.** La moyenne des `min/commit`
hebdomadaires laisse une semaine à 16 commits peser autant qu'une semaine à 100.
Toujours sommer les numérateurs et les dénominateurs. Écart mesuré entre les
deux méthodes sur kreek : ×1,87 contre ×2,28 sur le coût à la ligne.

**Une semaine creuse mesure une absence, pas un rythme.** En dessous de dix
commits, la semaine est marquée `sparse` et sort des moyennes — elle reste sur
les courbes pour la continuité, en point creux.

**`git cat-file --batch` donne des tailles en octets.** Lire la sortie en mode
texte fait dériver les décalages dès le premier accent, et le parcours casse
plusieurs fichiers plus loin, sans message clair. Lire en binaire, décoder après.

**Les tests unitaires Rust vivent dans les fichiers de production.** Le script
compte les lignes des blocs `#[cfg(test)]` par comptage d'accolades, et scinde
le temps `*.rs` de chaque semaine selon la progression respective des deux
parts. Sans ça, « code de production » absorbe les tests et la courbe ment.

**Le temps d'attente n'a pas de trace.** Une compilation, une suite qui tourne,
un e2e qu'on relance gonflent l'intervalle entre deux commits et sont donc
imputés à la catégorie du commit **qui suit**. Ce coût est bien compté dans le
total, mais il n'est pas isolable de l'historique seul. Le dire.

**La corrélation avec la taille n'est pas isolable.** Sur un projet en
croissance, tout ce qui croît corrèle avec tout ce qui croît : la taille du
code, la suite de tests, le corpus de règles, l'outillage de CI. Toujours
présenter les trois courbes ensemble et **refuser la causalité** — puis
raisonner sur la mécanique, qui elle est vérifiable (« un commit d'août doit
franchir N portes qui n'existaient pas en juin »).

## Vérifier la robustesse avant de conclure

Le script produit `control_dominant` : le même découpage du temps, mais chaque
commit versé **en entier** à sa catégorie dominante au lieu d'être réparti au
prorata. Si les deux méthodes divergent, la conclusion tient à la répartition et
non aux faits — le dire, ou ne pas conclure. Sur kreek elles donnent les mêmes
ordres de grandeur, ce qui est ce qui autorise la conclusion.

## Forme de l'artefact

Charger `artifact-design` puis `dataviz` avant d'écrire la page.

**Structure** — titre + chapô + quatre tuiles de chiffres clés, puis :

1. **La courbe** — trois graphiques **empilés et alignés sur le même axe de
   temps** : coût d'un commit, masse de code, taille d'un commit. Un survol
   désigne la même semaine dans les trois.
2. **La corrélation** — nuage de points `min/commit` × `log2(kLOC)` avec la
   droite d'ajustement, `r` et `R²`.
3. **Ce qui a grossi en même temps** — la réserve, avec les courbes des tests,
   de l'outillage et du corpus de règles normalisées à 100 %.
4. **Où est passé le temps** — barres empilées à 100 % par semaine, plus le
   tableau des parts par plateau et le contrôle par catégorie dominante.
5. **La compensation** — barres des heures actives par semaine.
6. **Méthode et limites** — sessions, forfait, périmètre, ce qui n'est pas mesuré.
7. **La série complète** — le tableau de toutes les semaines.

**Jamais deux axes verticaux sur un même graphique.** Deux grandeurs d'échelles
différentes se lisent en petits multiples alignés, pas superposées.

**Couleurs** — les emplacements du nuancier de `dataviz` (`references/palette.md`),
dans l'ordre, avec les deux variantes claire et sombre. Pour l'empilement à sept
segments : production `slot 1`, templates `slot 2`, tests unitaires `slot 3`,
tests e2e `slot 4`, outillage `slot 5`, écrit `slot 6`, divers en neutre. Cet
ordre est validé par `scripts/validate_palette.js` sur les paires adjacentes,
en clair comme en sombre — le vérifier après toute modification, ne jamais
l'estimer à l'œil.

**Écarter honnêtement les valeurs hors échelle** plutôt que de les écraser : les
premières semaines d'un projet ont des commits de plusieurs milliers de lignes
(collage de squelette) qui rendent illisible toute échelle linéaire. Démarrer la
courbe après, et le dire en légende.

## Adapter à un autre dépôt

Tout ce qui est spécifique au projet tient dans l'en-tête de `extract.py` :
`DOC_PREFIXES`, `CATEGORIES`, `CODE_GLOB`, et les trois seuils
(`SESSION_GAP_MIN`, `SESSION_LEAD_MIN`, `WEEK_MIN_COMMITS`). Le découpage des
modules de test est propre à Rust — pour un autre langage, remplacer
`split_test_modules`, ou ranger les tests dans `CATEGORIES` s'ils vivent dans
des fichiers séparés.
