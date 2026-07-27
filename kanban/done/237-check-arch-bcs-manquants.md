# `check-arch` — Trois BCs échappent au contrôle de souveraineté

**Priorité : haute**
**Dépend de :** —
**Fichiers :** `scripts/check-arch.sh`, `src/app/match_report/ports.rs`

## Problème

`scripts/check-arch.sh:29` énumère les BCs à contrôler :

```sh
BCS="auth competitions news players references spaces team_creation teams"
```

`src/app/` en contient onze (hors `shared_kernel`, partagé par construction).
**Trois sont absents** : `match_report`, `ranking`, `spp_calculator`.

L'axe 3 (souveraineté des données entre BCs) boucle sur cette liste **dans les
deux sens** — `for bc in $BCS` pour les fichiers scannés, `for other in $BCS`
pour les cibles interdites. Un BC absent est donc doublement invisible :

- ses propres références croisées ne sont jamais signalées
- les références **vers** lui depuis les autres BCs ne le sont pas non plus

Concrètement, `src/app/match_report/io/web/recap_controller.rs` a atteint
`state.spaces.space_repository` directement pendant des mois sans qu'aucune
vérification ne le relève. La violation a été trouvée à la main, pas par
l'outil. Les autres axes (2, 5, 6, 7) ne dépendent pas de `BCS` et couvrent bien
tout `src/app/*`.

## Action

Ajouter les trois BCs manquants :

```sh
BCS="auth competitions match_report news players ranking references spaces spp_calculator team_creation teams"
```

**La dette révélée est nulle.** La matrice complète sur les onze BCs a été
simulée avant d'écrire cette carte : elle ne remonte **qu'un seul hit**, et
c'est un faux positif.

## Le faux positif à corriger

`src/app/match_report/ports.rs:111` — un commentaire de documentation contient
la chaîne `state.spaces` pour expliquer pourquoi le port `ISpaceAdminPort`
existe :

```rust
/// recap devrait atteindre `state.spaces` directement — une référence croisée
```

Le contrôle est un `grep` sur le texte brut : il ne distingue ni les
commentaires, ni les chaînes littérales. Reformuler le commentaire sans citer
l'expression littérale.

## Limite connue, à ne pas traiter ici

Cette insensibilité aux commentaires et aux littéraux est **structurelle** :
`check-arch.sh` est un ensemble de `grep`, pas un analyseur syntaxique. Le même
piège se reproduira. Le noter ici suffit — passer à une analyse de l'AST est un
autre sujet, sans rapport avec cette carte.

## Checklist

- [ ] Les trois BCs ajoutés à `BCS`, liste maintenue par ordre alphabétique
- [ ] Commentaire de `ports.rs:111` reformulé sans citer l'expression littérale
- [ ] `make check-arch` passe, axe 3 au vert
- [ ] Vérifier qu'aucun BC de `src/app/` n'est absent de la liste, hors `shared_kernel`
- [ ] Envisager un garde-fou : faire dériver `BCS` du contenu de `src/app/`
      plutôt que d'une liste écrite à la main, pour qu'un futur BC soit couvert
      d'office
