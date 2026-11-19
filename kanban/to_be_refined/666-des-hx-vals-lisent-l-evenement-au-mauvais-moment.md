# Des `hx-vals` lisent l'événement au mauvais moment

**Priorité : à décider**
**Épic :** aucune · **Dépend de :** 649 (le patron du correctif)
**Trouvée par :** le diagnostic de la carte 649 (2026-11-11).

## Le constat

htmx 2.0.4 évalue un `hx-vals='js:{…}'` par `Function("return (…)")()`, sans
paramètre : `event` y désigne `window.event`, l'événement en cours **au moment de
l'évaluation**. Quand une requête est mise en file puis rejouée, cet événement
n'est plus le déclencheur, et la valeur lue est fausse — la 649 l'a prouvé sur
`#player-table-container` (le tableau des joueurs part sans roster).

Le même usage existe ailleurs :

- `build-team.html` : `#league-selector-zone`, `#special-rule-selector-zone` —
  `event.detail?.…` sans `?.` sur `event` : au rejeu, `TypeError` possible ;
- `finalize-team.html`, lignes 70 et 84-86.

## Ce qu'il faudrait faire

Appliquer le patron de la 649 : `hx-on::config-request` lit
`event.detail.triggeringEvent`, qui reste le vrai déclencheur au rejeu. Chercher
tous les `hx-vals` en `js:` qui lisent `event` avant de décider.

## Conséquences sur les données de production

Aucune attendue : seuls des paramètres de requêtes GET changent. À confirmer par
site.
