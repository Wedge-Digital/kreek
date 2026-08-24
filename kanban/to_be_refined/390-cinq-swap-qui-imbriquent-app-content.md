# Cinq `swap` qui imbriquent `#app-content` dans lui-même

> **⚠️ Cette carte demande ton attention** — pour être fermée sans travail
> inutile. Elle a d'abord été écrite sur un diagnostic faux, en priorité haute,
> en annonçant un défaut visible par un utilisateur. Ce n'en est pas un. Ce qui
> reste est un écart à une règle du projet, **sans conséquence mesurée**.

**Priorité : basse** — hygiène, aucun symptôme observable à ce jour
**Dépend de :** rien
**Trouvée par :** le diagnostic de `test_phase4_notifications`, dont elle n'était
finalement pas la cause

## Le constat

Naviguer, dans kreek, c'est **remplacer** le conteneur de contenu :

```html
hx-target="#app-content"     ← où
hx-select="#app-content"     ← quoi extraire de la réponse
hx-swap="outerHTML"          ← le fragment DEVIENT le conteneur
```

Le fragment porte lui-même l'`id` `app-content`, puisque c'est le layout qui le
rend. `outerHTML` fait qu'il prend la place de l'ancien : il y en a toujours
exactement un dans la page. **42 usages sur 46** suivent cet idiome.

Cinq endroits emploient les deux premières lignes et `innerHTML` à la place de
la troisième. Le fragment ne remplace plus le conteneur, il s'y **loge** :

| Fichier | Ligne | Élément |
|---|---|---|
| `new-competition-phase-2.html` | 429 | passage d'étape (`htmx.ajax`) |
| `new-competition-phase-3.html` | 368 | passage d'étape (`htmx.ajax`) |
| `new-competition-phase-4.html` | 275 | passage d'étape (`htmx.ajax`) |
| `news-feed.html` | 99 | pagination « ← » |
| `news-feed.html` | 126 | pagination « → » |

Dans `news-feed.html`, les boutons **numérotés** encadrés par ces deux flèches
utilisent `outerHTML` et sont corrects : la bonne forme et la mauvaise sont à
quinze lignes l'une de l'autre.

C'est le piège documenté dans `CLAUDE.md` — « Fragments HTMX : ne pas répéter
l'`id` du conteneur cible ». Il y est décrit pour les fragments rendus par le
serveur ; personne ne l'a rapproché de ces cinq-là.

## Ce que ça produit, et ce que ça ne produit pas

Mesuré sur le magicien : `document.querySelectorAll('#app-content').length`
vaut **2**, dès l'étape 4 et jusqu'à la validation.

Jamais plus de 2, et c'est important : `innerHTML` vide le conteneur avant
d'insérer, donc chaque transition écrase l'imbrication précédente au lieu de s'y
ajouter. Il n'y a pas d'accumulation.

**Aucun symptôme n'a pu être attribué à cette imbrication.** C'est le point qui
a changé depuis la première rédaction de la carte.

## Ce qui a été écarté — à ne pas refaire

Cette carte accusait l'`id` dupliqué d'empêcher htmx de câbler le contenu
injecté, et donnait pour preuve six éléments morts sur l'étape de validation,
dont « 🏆 Créer la compétition ». Les éléments morts existent bien. La cause
était ailleurs :

```
t=0ms    6 morts sur 31
t=50ms   0
t=500ms  0
```

htmx câble le contenu quelques dizaines de millisecondes après l'avoir rendu
visible. La fenêtre est **constante**, l'imbrication aussi — y compris lors des
passages où les 31 éléments étaient correctement câblés. L'`id` dupliqué n'y
est donc pour rien.

Playwright cliquait dans cette fenêtre ; un humain ne le peut pas. Le vrai
défaut était dans les tests, et il est corrigé (`tests/e2e/htmx_helpers.py`).

**Ne pas rouvrir cette piste :** l'imbrication ne casse pas le câblage.

## Ce qui reste à trancher

**Corriger, pour la seule raison qu'un `id` dupliqué est interdit ici.** Les
cinq sites passent en `outerHTML`, comme les 42 autres. Rien ne devrait changer
à l'écran — c'est bien le problème : la correction ne sera validée par aucun
symptôme qui disparaît.

**Assumer par écrit**, en notant dans `CLAUDE.md` que la règle souffre cinq
exceptions historiques sans conséquence. Honnête, mais laisse une divergence
que le prochain lecteur prendra pour un modèle à suivre — c'est déjà ce qui
s'est passé entre les flèches et les numéros de la pagination.

## Questions à trancher au raffinement

- `outerHTML` remplace le conteneur du layout : quelque chose en dépend-il —
  CSS, sélecteurs de menu, `hx-select` des entrées de navigation ? La question
  se pose surtout pour les trois appels `htmx.ajax`, qui passent leurs options
  en JS et échappent à toute relecture de gabarit.
- Un test e2e doit-il refuser plus d'un `#app-content` ? Ce serait le seul moyen
  de constater la correction, et un garde-fou générique peu coûteux. Global dans
  `conftest.py`, ou local aux pages concernées ?
- La pagination des actualités relève-t-elle de cette carte ou d'une carte
  propre au BC `news` ?
