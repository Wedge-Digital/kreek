# Trois toasts émis, aucun affiché

**Priorité : basse — personne ne s'en plaint, et c'est le problème**
**Épic :** aucune — dette d'interface
**Fichiers :** `src/app/team_creation/io/web/build_team/submit_team.rs`,
`.../finalize_team.rs`, `src/web/templates/app-layout.html`

## Le constat

Trois handlers de `team_creation` annoncent un succès au navigateur :

```rust
r#"{"showToast":"Équipe soumise avec succès !"}"#
```

**Personne n'écoute.** `grep -rn "toast" src/web/templates/ assets/static/js/
assets/static/css/common.css` ne rend rien : ni composant global, ni écouteur
d'événement, ni feuille de style. Le seul `showToast` vivant du projet est une
méthode Alpine **locale** à `actions-step.html`, qui n'a rien à voir ; l'autre
occurrence est une maquette (`app-space-admin.html`).

L'en-tête part, htmx déclenche bien un événement `showToast` sur le document, et
il s'y perd.

## Pourquoi ça n'a pas été vu

Parce que le parcours **marche quand même** : l'équipe est soumise, l'écran
suivant le montre. Le toast n'était qu'une confirmation supplémentaire, et son
absence ne casse rien — elle retire seulement l'accusé de réception qu'un
développeur a cru poser.

C'est la même famille que la carte 544, qui a fait découvrir celle-ci : une
information produite par le serveur et qui n'atteint aucun écran. La différence
est que la 544 concerne un compte qu'on ne peut pas retrouver autrement, tandis
qu'ici l'utilisateur voit de toute façon le résultat de son action.

## Ce qu'il faut décider

**Poser le composant, ou retirer les trois en-têtes.** Les deux sont
défendables, et la carte n'a pas à trancher seule :

- Un **toast global** dans `app-layout.html` — un écouteur `showToast` sur
  `body`, un conteneur, une feuille — rendrait vivantes les trois annonces
  existantes et donnerait au reste de l'application un moyen d'accuser réception
  d'une action sans occuper l'écran. C'est le composant qui manquait à la 544.
- **Retirer les trois en-têtes** coûte trois lignes et supprime un mensonge. Le
  jour où un toast sera vraiment voulu, il sera conçu pour de bon.

Ce qu'il ne faut pas, c'est les laisser : un en-tête qui n'a pas d'effet se
recopie, et le prochain écran qui « annonce » quelque chose le fera dans le vide
en croyant suivre un usage établi.

## Question ouverte

Si l'on pose le composant : quels écrans doivent l'utiliser, et lesquels doivent
au contraire montrer leur résultat **dans** la page ? Le panneau des présences a
tranché pour la seconde voie (carte 544) parce que la question « est-ce parti ? »
se repose le lendemain — un toast, par nature, ne survit pas à la seconde qui
suit.
