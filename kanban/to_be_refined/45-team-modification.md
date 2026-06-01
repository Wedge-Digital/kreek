# BC `teams` — Modification d'une équipe

**Priorité : moyenne**
**Dépend de :** `29-teams-repository.md`
**Contexte :** `teams` — action coach

## Objectif

Permettre au coach de modifier les informations d'identité de son équipe : nom, initiales et logo. Possible à tout moment quelle que soit la phase de jeu. La modification des numéros de joueurs est déléguée au BC `players` via widget.

---

## Ce qui est défini

- **Nom** → `TeamRenamed` event
- **Initiales** (2 lettres affichées dans le cercle logo) → `TeamInitialsChanged` event
- **Logo** → upload Cloudinary → `TeamLogoChanged` event (URL stockée)
- **Couleurs du gradient** → auto-générées, non stockées (dérivées du `team_id` par hash → teinte HSL)
- **Numéros de joueurs** → widget fourni par BC `players` dans la même fenêtre de modification
- Disponible à tout moment, sans contrainte de phase de jeu

---

## Événements domaine produits

```rust
TeamDomainEvent::TeamRenamed    { name: String },
TeamDomainEvent::InitialsChanged { initials: String }, // 2 caractères max
TeamDomainEvent::LogoChanged    { logo_url: String },
```

## Commandes

```rust
pub struct RenameTeamCommand     { pub team_id: TeamId, pub name: String }
pub struct ChangeInitialsCommand { pub team_id: TeamId, pub initials: String }
pub struct ChangeLogoCommand     { pub team_id: TeamId, pub logo_url: String }
```

## Routes

```
GET  /app/{space_id}/teams/{team_id}/edit          → formulaire de modification (fragment)
POST /app/{space_id}/teams/{team_id}/rename
POST /app/{space_id}/teams/{team_id}/initials
POST /app/{space_id}/teams/{team_id}/logo
```

## Impact sur la projection

`teams_projection` doit stocker `name`, `initials` et `logo_url` pour les cartes de liste.

---

## Ce qui reste à définir

- Format de validation des initiales : exactement 2 caractères ? Majuscules forcées ?
- Le pattern Cloudinary utilisé ici est-il identique à celui des autres uploads du projet ?
- La fenêtre de modification est un modal, un panneau latéral, ou une page dédiée ?
- Le widget BC `players` pour les numéros : quel endpoint expose-t-il ?

---

## Checklist (à compléter après raffinage)

- [ ] `TeamDomainEvent::TeamRenamed` + `InitialsChanged` + `LogoChanged`
- [ ] Commandes + use cases (validate → append event → update projection)
- [ ] Mise à jour `teams_projection` : `name`, `initials`, `logo_url`
- [ ] Couleurs auto-générées : fonction déterministe `team_id → (color1, color2)`
- [ ] Routes GET (fragment formulaire) + POST (chaque champ)
- [ ] Upload logo via Cloudinary (pattern existant dans le projet)
- [ ] Slot widget BC `players` pour les numéros de joueurs
