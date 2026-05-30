# Chargement de 20+ fichiers CSS sur toutes les pages

**Priorité : faible**
**Fichier :** `src/web/templates/app-layout.html`

## Problème

Le layout principal charge inconditionnellement tous les CSS de l'application, quelle que soit la page visitée :

```html
<link rel="stylesheet" href="/static/css/pages/new-competition-phase-2.css">
<link rel="stylesheet" href="/static/css/pages/new-competition-phase-3.css">
<link rel="stylesheet" href="/static/css/pages/competition-detail.css">
<!-- ... 17 autres ... -->
```

Un utilisateur qui ouvre le fil d'actualité télécharge aussi le CSS de la phase 4 de création de compétition. C'est 20 requêtes HTTP et du CSS inutile sur chaque page.

## Action recommandée

Migrer vers l'héritage de templates Askama (`extends`) — voir ticket #17 (migration extends).

Chaque template de page déclare ses dépendances CSS dans un bloc `{% block extra_css %}`. Le layout ne charge que le CSS commun (`common.css`, `layout-app.css`, `tom-select.css`).

Cette approche co-localise la dépendance CSS avec le template qui l'utilise et supprime le double rendu actuel (`content: String`).
