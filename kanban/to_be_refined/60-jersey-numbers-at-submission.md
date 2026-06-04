# Attribution des numéros de maillot à la soumission de l'équipe

**Priorité : basse**
**Contexte :** gestion des joueurs (post-création d'équipe)

## Situation actuelle

Sur la page de finalisation, des numéros de maillot par défaut sont calculés côté serveur
(séquentiels à partir de 1, en évitant les numéros déjà attribués) et affichés dans l'UI.
Ces numéros sont **purement visuels** : ils ne sont pas renvoyés au serveur et le champ
`HiredPlayer.jersey` reste `None` en base pour les joueurs sans numéro explicite.

## Comportement cible

À la soumission de l'équipe (POST finalize), les numéros affichés à l'utilisateur doivent
être persistés pour les joueurs qui n'en avaient pas encore.

## Action

**Frontend (`finalize-team.html`)** — inclure les numéros dans le payload POST :
```js
const payload = this.players.map(p => ({
  player_id: p.id,
  jersey:    p.jersey,
}));
// envoyer avec les skill assignments
```

**Backend (`post_finalize_team`)** — pour chaque joueur dont le jersey est `None` en base,
appeler le use case `set_player_identity` avec le numéro reçu, avant ou dans la même
transaction que `batch_finalize`.

## Option à trancher

- **Auto-assign silencieux** : les inputs jersey ne sont pas éditables sur la page finalize,
  les numéros séquentiels sont envoyés tels quels. Simple.
- **Éditables sur la page finalize** : inputs sur chaque ligne joueur, l'utilisateur peut
  ajuster avant soumission. Plus de travail UI.