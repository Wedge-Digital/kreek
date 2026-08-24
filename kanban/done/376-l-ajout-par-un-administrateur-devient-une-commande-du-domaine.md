# L'ajout par un administrateur devient une commande du domaine

**Priorité : haute**
**Dépend de :** 365 et 375
**Conception :** `docs/specs/space-admin/ajout-direct/06-domaine.md`
**Fichiers :** `src/app/spaces/domain/{space.rs, membership_error.rs, domain_event.rs}`

## Objectif

Troisième et dernière commande de l'agrégat `Space`, après celles de la carte
365 :

```rust
pub fn add_member(&mut self, acteur: &CoachId, nouveau: &CoachId,
                  profil: SpaceProfile)
    -> Result<ChangementDAppartenance, SpaceMembershipError>;
```

1. `nouveau` est déjà dans `coaches` → `DejaMembre`
2. l'ajouter, produire `UserAddedToSpaceByAdmin`

`acteur` **ne sert aucune règle** — rien n'interdit à un administrateur
d'ajouter qui il veut. Il est là pour la **trace** : l'ajout se passe du
consentement du coach, et une opération sans consentement doit dire qui l'a
ordonnée.

## `DejaMembre` n'est pas une redondance avec la clé primaire

Sans cette vérification, le doublon serait refusé par la clé composite de
`spaces__user_space` — une règle métier rendue par une erreur SQL brute,
illisible et intraduisible en 409.

Le badge « Déjà membre » de la liste des candidats est une **politesse**, comme
`role_locked` de l'onglet Membres. Un POST direct doit être refusé par le
domaine.

## L'événement

```rust
UserAddedToSpaceByAdmin { event_id, user_id, space_id, profile, added_by }
    → SpacesAppEvent::UserSubscribed
```

**Un événement domaine distinct, un app event partagé** avec
`UserSubscribedToSpace`. Le domaine sépare les deux faits — adhésion spontanée
et ajout sans consentement — parce que le journal doit les distinguer d'un
`grep`. L'extérieur n'a besoin que de l'effet : un coach est membre.

`notifier` **n'y figure pas** : c'est l'état d'une case à cocher au moment d'un
clic, et `event_log_feeder` persiste chaque enveloppe pour toujours.

## Deux non-règles à écrire, pas à deviner

**Les deux profils sont attribuables à l'ajout**, Membre comme Admin.
**Aucun plafond de membres par espace** — décidé en phase 5, pas oublié. Un
plafond ajouté plus tard, une fois des espaces au-delà du seuil en production,
coûte bien plus qu'un plafond posé d'emblée.

Les deux se commentent dans le code, sinon quelqu'un les ajoutera en croyant
réparer un oubli.

## Checklist

- [x] ~~`Space::add_member(acteur, nouveau: &CoachId, profil)`~~ →
      **`add_member(acteur, nouveau: Coach)`**. L'agrégat stocke des `Coach`, qui
      portent un pseudo et une icône : un identifiant seul ne permet pas d'en
      construire un. Le use case le bâtira depuis `find_user_by_id`, qui existe
      au port du cache
- [x] `DejaMembre` ajoutée à `SpaceMembershipError`, traduite en **409**
- [x] `UserAddedToSpaceByAdmin` : variante, type, tags, mapping vers
      `SpacesAppEvent::UserSubscribed` — le même app event que l'adhésion
      spontanée
- [x] Aucune primitive nue dans l'événement
- [x] Les deux non-règles commentées sur place
- [x] Cinq tests unitaires, dont celui qui distingue l'ajout d'une promotion
- [x] Les deux tests clés **vus échouer** sur une implémentation qui met à jour
      le profil au lieu de refuser
- [x] `make lint`, `make check-arch`, `make test` passent — 1149 tests

## Ce qu'on a appris en la faisant

**Le verrou de la carte 364 avait dérivé, en silence.** Il vérifiait qu'aucune
variante ne partage son type d'événement, mais énumérait les variantes **à la
main** : trois ajoutées depuis — deux par la carte 365, une par celle-ci — n'y
figuraient pas. Il ne couvrait plus que cinq cas sur huit, et rien ne l'a dit.

**Un test qui énumère ne sait pas qu'il est incomplet.** La liste est complétée,
mais surtout un `match` exhaustif a été ajouté : il **ne compile plus** dès
qu'une variante apparaît, le compilateur amène dessus, et son commentaire renvoie
à la liste à tenir. La dérive silencieuse devient une erreur de compilation.

**`DejaMembre` est atteignable**, contrairement à `DernierAdministrateur` — un
administrateur peut poster un ajout pour un coach déjà membre, la garde passe, et
rien ne l'arrête avant le domaine. Vérifié **avant** de coder, après trois cartes
où un test annoncé s'est révélé impossible à écrire.
