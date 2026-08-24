# Le magicien empile des `#app-content`, et le bouton « ← Retour » devient inerte

> **⚠️ Cette carte demande ton attention.** Ce n'est pas un test fragile : le
> défaut est visible par un utilisateur, et il est silencieux. Le choix du
> correctif engage la navigation des trois étapes du magicien.

**Priorité : haute** — un bouton mort, sans message, sur un parcours de création
**Dépend de :** rien
**Trouvée par :** l'instabilité de `test_phase4_notifications.py`, dont le vrai
motif n'était pas le test

## Le constat

À chaque passage d'étape, le magicien de création de compétition fait :

```js
htmx.ajax('GET', redirect, { target: '#app-content', swap: 'innerHTML', select: '#app-content' });
```

Il **sélectionne `#app-content` dans la réponse et l'injecte dans
`#app-content`**. Le conteneur se retrouve donc à contenir un élément portant
son propre `id`. Le motif est présent trois fois :

| Fichier | Ligne |
|---|---|
| `new-competition-phase-2.html` | 429 |
| `new-competition-phase-3.html` | 368 |
| `new-competition-phase-4.html` | 275 |

C'est exactement le piège documenté dans `CLAUDE.md`, section « Fragments HTMX :
ne pas répéter l'`id` du conteneur cible ». Il y est décrit pour les fragments
rendus par le serveur ; personne ne l'a rapproché de ces trois appels JS.

## Ce que ça produit

Sur l'étape de validation, interrogé au moment du clic :

```
{ cable: false, n: 2 }
```

- `n: 2` — deux `#app-content` coexistent dans la page.
- `cable: false` — le bouton « ← Retour » **n'a pas été câblé par htmx**
  (aucune donnée interne htmx sur l'élément).

Conséquence : le clic ne déclenche **aucune requête réseau**. Vérifié en
enregistrant tout le trafic après le clic — la liste est vide. La page reste sur
`/validation`, sans message, sans erreur de console, sans rien.

**Un utilisateur qui clique « ← Retour » assez tôt n'obtient rien.** Le
comportement est intermittent : quand le clic arrive plus tard, le bouton
fonctionne. C'est ce qui a fait passer ce défaut pour de l'instabilité de test.

## Ce qui a été essayé et n'a pas marché

Attendre qu'aucune requête htmx ne soit en vol avant de cliquer — le remède qui
a corrigé un cas de même **symptôme** dans `test_player_spp_spending.py`. Ici
il ne change rien : **4 échecs sur 6 passages** avec cette attente en place. Le
bouton n'est pas cliqué trop tôt, il n'est jamais câblé.

Ne pas repartir de cette piste.

## Ce qui est à trancher

**Ne plus dupliquer l'`id`.** Remplacer `select: '#app-content'` par une
sélection du contenu *intérieur*, ou passer en `swap: 'outerHTML'` pour que le
fragment **soit** le conteneur au lieu de s'y loger. C'est la voie qui supprime
la cause.

**Comprendre d'abord pourquoi htmx ne câble pas.** L'imbrication est certaine,
le mécanisme exact par lequel elle laisse un bouton non traité ne l'est pas —
htmx transfère des attributs entre éléments de même `id` lors d'un swap, et
c'est la piste la plus probable. Trancher sans ce point risque de déplacer le
défaut plutôt que de le fermer.

**Traiter les trois étapes d'un bloc.** Le motif est identique aux trois
endroits ; n'en corriger qu'un laisserait le parcours à moitié réparé.

## Ce que la carte ne couvre pas

**Le correctif du test.** `test_phase4_notifications.py` est laissé **tel quel**,
instable — délibérément. Le stabiliser avant de corriger la page reviendrait à
masquer le seul témoin du défaut.

## Questions à trancher au raffinement

- `outerHTML` sur `#app-content` remplace le conteneur du layout : quelque chose
  d'autre en dépend-il (CSS, sélecteurs de menu, `hx-select` des entrées de
  navigation) ?
- Le même motif existe-t-il ailleurs que dans le magicien ? Le grep ci-dessus ne
  couvre que `competitions`.
- Un test E2E doit-il vérifier qu'il n'existe **qu'un seul** `#app-content` dans
  la page ? Ce serait un garde-fou générique, peu coûteux.
