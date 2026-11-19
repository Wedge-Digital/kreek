# `check-arch.sh` axe 3 — aveugle aux chaînages `state.<bc>` coupés par rustfmt

**Priorité : moyenne**
**Fichier :** `scripts/check-arch.sh` (axe 3, section « souveraineté des données entre BCs »)

## Problème

L'axe 3 détecte les références croisées `state.<bc>` avec un grep **ligne par ligne** :

```bash
hits=$(printf '%s\n' "$prod_code" | grep -nE "\bstate\.${other}\b" || true)
```

`rustfmt` casse un chaînage de méthodes trop long **un appel par ligne** :

```rust
state
    .spaces
    .space_repository
    .find_member_profile(&user.id, space_id)
```

`state` termine une ligne, `.spaces` commence la suivante : la sous-chaîne littérale `state.spaces` n'existe sur aucune ligne prise isolément, donc le regex ne matche jamais — quel que soit le BC visé, pas seulement `spaces`.

C'est exactement ce qui a laissé passer la violation corrigée en carte 277 (`resultats_view.rs`) et celle ouverte en carte 296 (`widget_tester_controller.rs`) : les deux étaient invisibles à `make check-arch` uniquement à cause de ce retour à la ligne.

## Action

Ajouter un helper qui recolle les lignes de continuation d'un chaînage (celles qui commencent par `.`, après indentation) sur la ligne précédente, dans le même esprit que `strip_test_code`/`strip_comments` déjà présents dans le script :

```bash
# Recolle les lignes de continuation d'un chaînage rustfmt (celles qui
# commencent par `.`) sur la ligne précédente. Sans ça, `state\n.spaces\n...`
# — la mise en forme par défaut d'un appel trop long — ne matche jamais
# `\bstate\.${other}\b` : la sous-chaîne littérale n'existe sur aucune ligne
# prise isolément.
join_chains() {
    awk '{
        if ($0 ~ /^[[:space:]]*\./) { gsub(/^[[:space:]]+/, ""); line = line $0; next }
        if (line != "") print line
        line = $0
    }
    END { if (line != "") print line }'
}
```

Appliquer `join_chains` à `prod_code` avant les deux `grep` de la boucle axe 3 (celui sur `use crate::app::${other}::` et celui sur `state.${other}`) :

```bash
prod_code="$(strip_test_code "$f")"
joined="$(printf '%s\n' "$prod_code" | join_chains)"
...
hits=$(printf '%s\n' "$joined" | grep -nE "\bstate\.${other}\b" || true)
```

Le regex existant n'a pas besoin de changer : une fois les lignes recollées sans espace inséré, la sous-chaîne `state.spaces` réapparaît telle quelle.

**Contrepartie assumée** : les numéros de ligne rapportés deviennent ceux du **début** du chaînage joint, pas la position exacte de `.${other}`. Acceptable — l'axe désigne déjà `file:line` comme repère à vérifier à la main, pas une position cliquable exacte.

**Piste écartée** : un `tr -d '\n'` global sur tout le fichier serait plus simple mais casse tout — fusionnerait des instructions sans rapport à travers les frontières de fonctions, produirait des faux positifs, et détruirait complètement les numéros de ligne pour le reste du fichier.

## Checklist

- [x] Helper `join_chains` ajouté à `scripts/check-arch.sh`
- [x] Appliqué à `prod_code` avant les deux `grep` de l'axe 3
- [x] `make check-arch` détecte bien la violation encore ouverte de la carte 296 (`widget_tester_controller.rs`) — sert de cas de test réel avant qu'elle ne soit corrigée
- [x] `make check-arch` ne régresse pas sur le reste du projet (pas de nouveau faux positif introduit par le recollage de lignes)

---

## Notes d'implémentation

**Il y avait un second bug, plus grave que celui décrit par la carte.**

`BCS` était dérivée par `find src/app -printf '%f\n'`. `-printf` est une
extension **GNU** que le `find` BSD de macOS ignore : la commande échouait, la
liste restait **vide**, et les boucles des axes 3 et 9 ne parcouraient rien —
en affichant `✓ PASS`. Un verrou qui rassure sans jamais regarder.

La CI tournant sous Linux, l'écart ne se voyait qu'en local, c'est-à-dire
précisément là où on l'exécute avant de commiter. Ça n'a été repéré que parce
que les tests manuels passaient quand le script échouait : dans un shell
interactif, `find` peut être une fonction enveloppant `bfs`, qui accepte
`-printf`.

Remplacé par un glob shell, avec un **arrêt franc** si la liste est vide — une
étape sautée doit échouer, pas rassurer.

**`strip_comments` appliqué à l'axe 3.** Le commentaire qui explique pourquoi on
ne fait plus `state.<bc>` déclenchait l'axe qu'il documente. Même raison qu'à
l'axe 9, où le garde-fou existait déjà.

## Ce que la réparation a découvert

Sept violations, dont **six inconnues** : cinq `team_creation` → `competitions`
(carte 300) et une `teams` → `spaces` (carte 301), plus celle de la carte 296.

Corriger les six dépassait de loin ces deux cartes. Elles entrent dans une
**ligne de base explicite** (`AXE3_BASELINE_REGEX`), datée et référencée à une
carte : l'axe redevient bloquant pour toute violation **nouvelle**, ce qu'un
axe rendu non bloquant n'aurait pas permis.

Vérifié par test de mutation : une référence croisée ajoutée dans un fichier
hors ligne de base fait bien échouer l'axe, chaînage multiligne compris.
