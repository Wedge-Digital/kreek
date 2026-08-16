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

- [ ] `ICompetitionSpaceMemberPort` : `list_space_members(space_id)` +
      `SpaceMemberDto`
- [ ] `space_member_adapter.rs` : implémentation via `list_members_for_space.sql`,
      déjà écrite dans `spaces`
- [ ] `use_cases/notification_recipients.rs` : `resolve()`, les cinq cas
- [ ] `Recipient`, `Fixture`, et `RoundParticipation { NotPlaying,
      Playing(Vec<Fixture>) }`
- [ ] Un coach à **deux équipes** rend **un** destinataire portant **deux**
      fixtures — ne pas retomber sur un `Option`
- [ ] Test `#[sqlx::test]` : un coach hors espace n'est jamais destinataire
- [ ] Test : date limite → les inscrits sont exclus
- [ ] Test : coach inscrit sans match → `NotPlaying`
- [ ] `make check-arch`
