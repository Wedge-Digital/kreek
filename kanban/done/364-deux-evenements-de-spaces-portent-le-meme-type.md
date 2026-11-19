# Deux événements de `spaces` portent le même type

**Priorité : haute** — préalable à l'onglet Invitations, qui sera le premier à
déclencher le défaut
**Dépend de :** rien
**Fichiers :** `src/app/spaces/domain/domain_event.rs`

## Le problème

```rust
pub const USER_SUBSCRIBED_TO_SPACE: &str = "UserRegisteredInSpace";
pub const USER_INVITED_IN_SPACE:    &str = "UserRegisteredInSpace";
```

Deux événements distincts partagent leur chaîne de type. `to_event_type()` rend
la même valeur pour `UserSubscribedToSpace` et `UserInvitedInSpace`, donc tout
listener qui filtre sur le type attrape les deux — et toute lecture du journal
d'événements les confond.

## Pourquoi ça n'a jamais explosé

`UserInvitedInSpace` **n'est émis nulle part**. Le défaut est latent depuis sa
définition.

Il cessera de l'être à l'onglet Invitations, qui sera le premier à l'émettre. À
ce moment-là, un `UserInvitedInSpace` persisté serait relu comme une
souscription — un coach invité compterait comme un membre.

## La correction

`USER_INVITED_IN_SPACE` prend sa propre valeur, `"UserInvitedInSpace"`.

`USER_SUBSCRIBED_TO_SPACE` **garde** `"UserRegisteredInSpace"`, malgré la
discordance avec son nom Rust : cette chaîne est dans le journal d'événements
depuis l'origine, et la changer réécrirait le sens de l'historique. La
discordance se commente sur place.

## Pourquoi une carte à elle seule

La carte 365 touche ce fichier. Mêler la correction d'un défaut préexistant à
une fonctionnalité produit un commit où plus personne ne sait lequel des deux a
cassé quoi.

## Checklist

- [x] `USER_INVITED_IN_SPACE` vaut `"UserInvitedInSpace"`
- [x] `USER_SUBSCRIBED_TO_SPACE` inchangé, avec un commentaire disant pourquoi —
      cinq lignes existent en base sous ce nom, et un type d'événement persisté
      est un identifiant public, pas un nom de variable
- [x] Test : les cinq variantes rendent cinq chaînes **distinctes** — vérifier
      qu'une variante rend *sa* chaîne ne suffisait pas, les deux rendaient
      chacune la leur et c'était la même
- [x] Le test **vu échouer** sur le défaut réintroduit, avec un message qui nomme
      les deux types en collision plutôt que « 4 != 5 »
- [x] Second test verrouillant `"UserRegisteredInSpace"` pour la souscription,
      pour qu'un futur zélé ne « répare » pas la discordance
- [x] Vérifié qu'aucun listener ni aucune requête ne filtre sur
      `"UserRegisteredInSpace"` en attendant les deux — la seule autre
      occurrence est une assertion de test dans `join_spaces.rs`, qui reste juste
- [x] `make lint`, `make check-arch`, `make test` passent — 1090 tests

## Ce qu'on a appris en la faisant

Le journal contenait **cinq** lignes `UserRegisteredInSpace` et vingt et une
`SpaceCreated`. Les cinq sont des souscriptions, `UserInvitedInSpace` n'ayant
jamais été émis — c'est ce qui a rendu la correction possible sans migration :
la seule valeur présente en base est celle qu'on ne touche pas.
