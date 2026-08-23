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

- [ ] `USER_INVITED_IN_SPACE` vaut `"UserInvitedInSpace"`
- [ ] `USER_SUBSCRIBED_TO_SPACE` inchangé, avec un commentaire disant pourquoi
- [ ] Test : les cinq variantes de `SpacesDomainEvent` rendent cinq chaînes
      **distinctes** — c'est le test qui aurait attrapé le défaut
- [ ] Vérifié qu'aucun listener ni aucune requête ne filtre sur
      `"UserRegisteredInSpace"` en attendant les deux
- [ ] `make lint`, `make check-arch`, `make test` passent
