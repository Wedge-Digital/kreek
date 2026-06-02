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
