# Scripts inline dans les fragments HTMX

**Priorité : faible**
**Fichiers :** `competition-widget.html`, `space-members-widget.html`

## Problème

Les fragments HTMX contiennent des blocs `<script>` qui initialisent TomSelect :

```html
<script>
(function () {
  var el = document.getElementById('competition-season-select');
  if (!el || el.tomselect) return;
  new TomSelect(el, { ... });
})();
</script>
```

Chaque swap HTMX ré-injecte ce script dans le DOM. Si le fragment est swappé plusieurs fois (changement d'espace, refresh partiel), le script s'exécute à chaque fois. Le guard `if (!el.tomselect)` protège contre la double initialisation, mais :

- Le script est exécuté inutilement à chaque swap
- HTMX n'exécute pas les scripts injectés de la même façon selon la version et la configuration (`hx-swap`, `hx-boost`)
- Difficile à tester

## Action

Utiliser l'événement HTMX `htmx:afterSwap` ou `htmx:load` pour initialiser TomSelect depuis un script global, plutôt que d'injecter un script dans chaque fragment :

```javascript
// app.js (chargé une seule fois dans le layout)
document.body.addEventListener('htmx:afterSwap', function(e) {
    e.detail.target.querySelectorAll('select[data-tomselect]').forEach(el => {
        if (!el.tomselect) new TomSelect(el, { ... });
    });
});
```

Les selects à initialiser sont marqués avec `data-tomselect` dans le HTML, sans script inline.
