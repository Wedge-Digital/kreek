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

- [ ] `Space::add_member(acteur, nouveau, profil)`
- [ ] `DejaMembre` ajoutée à `SpaceMembershipError`
- [ ] `UserAddedToSpaceByAdmin` : variante, type, tags, mapping `to_app_event()`
      vers `SpacesAppEvent::UserSubscribed`
- [ ] Aucune primitive nue dans l'événement — quatre value objects
- [ ] Les deux non-règles commentées sur place
- [ ] Tests unitaires :
  - [ ] ajout d'un non-membre en Membre → événement, compte d'admins inchangé
  - [ ] ajout d'un non-membre en Admin → compte +1
  - [ ] coach déjà membre → `DejaMembre`, **`coaches` inchangé**
  - [ ] coach déjà membre, **avec un autre profil** → `DejaMembre` — ce n'est
        pas une promotion déguisée. Sans ce test, l'ajout devient un chemin
        détourné pour changer un rôle, sans la règle du dernier administrateur
  - [ ] `added_by` porte bien l'acteur
- [ ] `make lint`, `make check-arch`, `make test` passent
