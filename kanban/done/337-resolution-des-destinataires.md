# Résolution des destinataires

**Spec :** `docs/specs/notifications/envoi/05-use-cases.md`, et R7
**Dépend de :** rien
**Ouvre :** 338, 339

## Objectif

Qui reçoit quoi, borné par l'espace.

## Conception

### R7 est tenue par le chemin de données

| Notification | Ensemble |
|---|---|
| Ouverture — mode `invitation` | invités **∩** membres |
| Ouverture — mode `open` | membres |
| Veille / fin de journée | inscrits **∩** membres |
| Date limite | (invités ou membres) **∩** membres **−** inscrits |

**L'intersection n'est pas un contrôle ajouté : c'est le seul chemin vers une
adresse email.** Ni `invited_coaches` ni `find_enrolled_teams` ne portent
d'adresse ; seul `list_space_members` en a. Un invité qui a quitté l'espace tombe
donc naturellement.

Sans cette borne, « tous ceux qui peuvent s'inscrire » désignerait la plateforme
entière.

### La date limite se calcule par différence

Aucun port ne rend « les invités qui ne se sont pas inscrits ». C'est ce qui
justifie l'existence de ce service : sans lui, la soustraction finirait dans la
CLI.

### Le piège des trois tables vides

`competitions__user_cache`, `competitions__space_cache` et
`competitions__user_space_cache` contiennent emails et appartenance, et ne sont
**ni lues ni écrites nulle part**. Les brancher ferait un envoi silencieusement
vide. Passer par le port.

## Checklist

- [x] `ICompetitionSpaceMemberPort` : `list_space_members(space_id)` +
      `SpaceMemberDto`
- [x] `space_member_adapter.rs` : implémentation via `list_members_for_space.sql`,
      déjà écrite dans `spaces`
- [x] `use_cases/notification_recipients.rs` : `resolve()`, les cinq cas
- [x] `Recipient`, `Fixture`, et `RoundParticipation { NotPlaying,
      Playing(Vec<Fixture>) }`
- [x] Un coach à **deux équipes** rend **un** destinataire portant **deux**
      fixtures — ne pas retomber sur un `Option`
- [x] Test `#[sqlx::test]` : un coach hors espace n'est jamais destinataire
- [x] Test : date limite → les inscrits sont exclus
- [x] Test : coach inscrit sans match → `NotPlaying`
- [x] `make check-arch`

## Ce qui a été fait, et le trou de la spec qu'il a fallu combler

**La spec ne disait pas d'où viennent les appariements.** Elle demande que
`RoundParticipation` croise « les appariements de la journée » avec les équipes
du coach, mais `resolve()` ne reçoit qu'un `RoundRef` — figé en phase 4, écrit à
la carte 336 — qui n'en porte aucun. Le mot n'apparaît qu'une fois dans les
phases 3 à 7.

Tranché : **`RoundRef` gagne ses `pairings`**. Ils sont déjà chargés — `MatchDay`
les porte, et `due_today()` construit `RoundRef` à partir de lui — donc les
recopier coûte moins qu'un port de lecture pour une donnée déjà en main, et
aucune des deux signatures que la spec fixe ne bouge.

**`Fixture` perd son `match_url`.** Le construire obligerait un service de
`use_cases/` à connaître `AppRoutes`, qui est de la couche web. Le gabarit reçoit
déjà `app_url` et sait composer.

## Deux tests plutôt qu'un pour R7

La carte demande un `#[sqlx::test]` « un coach hors espace n'est jamais
destinataire ». Écrit avec des doublures, ce test ne prouve rien de R7 : une
doublure rend ce qu'on lui a écrit. Il en faut donc **deux**, et ils ne gardent
pas la même chose :

| Test | Ce qu'il garde |
|---|---|
| `notification_recipients` (doublures) | le service n'invente aucun destinataire hors de la liste reçue |
| `test_space_member_adapter` (`#[sqlx::test]`) | **la liste elle-même est bornée par l'espace** |

Le second monte un espace voisin avec son propre coach. Vérifié en le faisant
échouer exprès : en remplaçant `WHERE m.space_id = $1` par une condition
toujours vraie, il tombe sur le coach voisin. C'est là que R7 se serait perdue
sans bruit.

## Axe 11

`resolve()` porte un `arch:no-instrument` motivé : c'est un service de
résolution — deux ports, aucune mutation, aucun évènement, aucune persistance.
Le marqueur doit être sur la ligne **immédiatement précédente**, le verrou ne
regardant que celle-là.
