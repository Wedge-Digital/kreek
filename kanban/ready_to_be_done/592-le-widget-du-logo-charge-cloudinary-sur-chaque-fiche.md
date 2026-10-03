# Le widget du logo charge Cloudinary sur chaque fiche d'équipe

**Priorité : moyenne** — un test instable, et trois traceurs tiers sur chaque
fiche d'équipe
**Épic :** aucune
**Dépend de :** rien
**Fichiers :** `src/web/templates/components/upload_widget.html`,
`src/app/teams/io/web/templates/widgets/team-logo.html`

## Le constat (2026-10-04)

**Un test instable.** `test_dismissals_phase.py::test_01_banniere_ouvre_l_effectif`
échoue une fois sur cinq, à sa clôture, sur une erreur de console :

```
pageerror: Cannot read properties of null (reading 'addEventListener')
```

Le test ouvre la fiche d'équipe et clique aussitôt sur le bandeau, qui remplace
`#app-content` par la page des renvois.

**La cause.** Le widget du logo (PR #11, cartes 508 à 512) se charge en différé
sur la fiche et embarque la macro `cloudinary_upload`. Son script télécharge
d'abord `all.js` depuis `widget.cloudinary.com`, **puis** appelle
`document.getElementById('zone-logo_url').addEventListener(…)`. Si l'on a quitté
la fiche pendant le téléchargement, la zone n'existe plus : `null`, et l'erreur.
L'instabilité suit le temps de réponse de Cloudinary.

**Le problème plus grave que le test.** Le commit `38c02228` (2026-08-22) avait
sorti ce chargeur du layout, pour une raison écrite dans la macro : il tire avec
lui **Google Tag Manager, GA4 et Rollbar**, et ne devait suivre un coach que sur
les deux pages qui ont un champ d'envoi. Depuis la PR #11, il se charge sur
**chaque fiche d'équipe** ouverte par qui peut modifier le logo — même s'il n'y
touche jamais.

## Le changement

1. **Le défaut** : le script de la macro vérifie que sa zone existe encore avant
   d'y brancher le clic. Une zone disparue n'est pas une erreur — l'utilisateur
   est parti.
2. **Le chargement** : Cloudinary n'est téléchargé qu'au passage en mode édition
   du logo, pas à l'ouverture de la fiche. C'est la décision de `38c02228`
   rétablie, et la course disparaît avec elle.

À trancher en conception : si le point 2 se fait dans la macro (chargement au
premier clic sur la zone, pour toutes les pages qui l'utilisent) ou dans le
seul widget du logo.

## Tests

- E2E : ouvrir la fiche d'équipe ne télécharge pas `widget.cloudinary.com` ;
  passer en mode édition du logo le télécharge.
- E2E : quitter la fiche aussitôt ouverte ne produit aucune erreur de console.
- `test_01_banniere_ouvre_l_effectif` passe vingt fois de suite seul.

## Terminé quand

Une fiche d'équipe ouverte sans toucher au logo ne charge aucune ressource de
Cloudinary, et la suite complète passe sans erreur de console.
