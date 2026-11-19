# Dépendances CDN en production

**Priorité : faible**
**Fichier :** `src/web/templates/app-layout.html`

## Problème

TomSelect et le widget Cloudinary sont chargés depuis des CDN externes :

```html
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/tom-select@2.4.1/...">
<script src="https://cdn.jsdelivr.net/npm/tom-select@2.4.1/..."></script>
<script src="https://widget.cloudinary.com/v2.0/global/all.js"></script>
```

Conséquences :
- Si `cdn.jsdelivr.net` est indisponible, tous les selects (TomSelect) sont cassés
- Si le widget Cloudinary est indisponible, l'upload d'images ne fonctionne pas
- Les requêtes vers des domaines tiers sont visibles des utilisateurs (vie privée)
- Impossible de faire fonctionner l'app hors-ligne ou en environnement isolé

## Action

Télécharger TomSelect et le bundler localement dans `assets/static/js/` et `assets/static/css/`, et servir depuis `/static/`. Le widget Cloudinary est plus complexe — évaluer s'il peut être remplacé par un upload direct vers l'API Cloudinary géré côté serveur.

## Ce qui a été fait

La carte citait deux dépendances et un fichier. Il y en avait **quatre**, dans
**deux** layouts : TomSelect (CSS et JS) et Alpine depuis jsdelivr, les trois
familles de polices depuis `fonts.googleapis.com` — par un `@import` en tête de
`common.css`, donc découvert après le bundle et bloquant — et le widget
Cloudinary.

Les trois premières sont servies depuis `/static/`. Les polices sont des fontes
**variables** : dix fichiers au lieu de quarante-deux, sous-ensembles `latin` et
`latin-ext` seulement. Elles couvrent 100 à 900, ce que l'ancien `@import` ne
faisait pas — le 600 n'existait dans aucune famille et le CSS l'emploie 128
fois. Changement de rendu assumé, borné à 3 règles (Montserrat 500 et 600 ; le
corps de texte reste à Roboto, dont la largeur ne bouge pas).

**Le remplacement du widget Cloudinary a été fait, puis annulé.** L'évaluation
que la carte demandait a été menée jusqu'au code : un champ maison postant sur
l'API avec le preset non signé déjà en place. Il fonctionnait, mais faisait
perdre la source URL, le glisser-déposer et le recadrage — pour supprimer une
requête CSS qui ne stylait que la boîte de dialogue du widget, donc sans rapport
avec le clignotement que vise l'épic E03.

Ce qui a tranché est apparu en écrivant le test de garde-fou : **`all.js` tire
de lui-même Google Tag Manager, GA4 et Rollbar**. Deux mesureurs d'audience et
un traqueur d'erreurs, posés chez les coachs sans que rien dans notre code ne
les nomme. Le widget est donc conservé — pour ses fonctions — mais son chargeur
quitte les layouts : la macro `components/upload_widget.html` le pose
elle-même, sur les deux seules pages qui portent un champ d'upload. Ailleurs,
les trois traqueurs ne se chargent plus du tout.

Contrepartie tenue : `all.js` n'est plus présent quand le script en ligne du
champ s'exécute, donc son corps est différé jusqu'à ce que `cloudinary` existe.
Un test e2e clique la zone et attend l'iframe — aucun test de réseau n'aurait
vu ce report casser.

Mesuré après coup : le compte de `<link>` du document ne bouge pas, même sur les
pages d'upload (Cloudinary charge sa feuille pour son iframe, pas pour notre
DOM), et le déplacement vertical y vaut 316 et 369 px, **exactement les valeurs
d'avant la carte** — ce sont deux des dix pages que la carte 361 doit traiter.
