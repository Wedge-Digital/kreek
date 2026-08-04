# Phase 4 — Contrats de données — page "Mes équipes"

Le mapping statut+phase → libellé/classe réutilise **exactement** les
libellés déjà en prod dans `team_detail.rs::status_display()`
(lignes 242-260), pas ceux inventés par l'ancien kanban 44 ni ceux de la
maquette (`status-post-match` en bleu) qui divergeraient de l'existant.

## Mapping statut + phase → libellé/classe (repris de `team_detail.rs`)

| `status` | `game_phase` | Groupe | Libellé | Classe CSS |
|---|---|---|---|---|
| PendingEnrollment | — | active | En attente d'inscription | `pending` |
| Enrolled | ReadyToPlay | active | Prête à jouer | `ready` |
| Enrolled | MatchReporting | active | Rapport en cours | `phase` |
| Enrolled | PlayerImprovement | active | Phase d'amélioration | `phase` |
| Enrolled | Recruitment | active | Phase de recrutement | `phase` |
| Enrolled | Dismissals | active | Phase de renvois | `phase` |
| Enrolled | TemporaryRetirement | active | Retraite temporaire | `phase` |
| Enrolled | OffSeason | active | Off-season | `offseason` |
| Enrolled | *(null)* | active | Inscrite | `ready` |
| Rejected | — | archivée | Inscription refusée | `dismissed` |
| Dismissed | — | archivée | Renvoyée | `dismissed` |

Conséquence sur la maquette : la distinction visuelle bleue "Amélioration
joueurs" vs les autres sous-phases n'existe pas dans le badge canonique
(tout est `phase`, une seule couleur) — CSS du widget simplifié en phase 7
pour rester cohérent avec `team_detail.rs`.

Nouvelle fonction pure `status_label_and_class(status: &str, game_phase:
Option<&str>) -> (String, String)`, colocalisée dans `my_teams_widget.rs`
(≤ 20 lignes), commentaire renvoyant vers `team_detail.rs::status_display`
comme source canonique du texte — petite duplication assumée, même pattern
que `initials()` déjà dupliquée entre widgets existants.

## DTOs d'entrée

Aucun — les deux endpoints (`my_teams`, `my_teams_widget`) ne prennent que
`Path(space_id)` et `AuthSession`, pas de query params (filtres supprimés).

## VMs de sortie

```rust
// team_creation/io/web/my_teams.rs
pub struct DraftTeamCardVm {
    pub id: String,
    pub initials: String,
    pub name: String,
    pub logo: Option<String>,
    pub roster: Option<String>,   // None si roster pas encore choisi
    pub link: String,
}
```
Émis par : handler `my_teams` (fusion `DraftTeam` + `Option<RosterSelectedTeam>`).
Consommé par : `my-teams.html` (markup `draft-card` inline).

```rust
// teams/ports.rs — DTO de lecture du repository
pub struct MyTeamRow {
    pub team_id: String,
    pub team_name: String,
    pub roster_name: String,
    pub logo_url: Option<String>,
    pub status: String,
    pub game_phase: Option<String>,
}
```
Émis par : `TeamRepository::find_by_coach_and_space`.
Consommé par : handler `my_teams_widget`.

```rust
// teams/io/web/widgets/my_teams_widget.rs
pub struct TeamCardVm {
    pub team_id: String,
    pub initials: String,
    pub team_name: String,
    pub roster_name: String,
    pub logo: Option<String>,
    pub status_class: String,
    pub status_label: String,
    pub link: String,
}
pub struct ArchivedTeamCardVm {
    pub team_id: String,
    pub initials: String,
    pub team_name: String,
    pub roster_name: String,
    pub status_label: String,
    pub link: String,
}
```
Émis par : handler `my_teams_widget` (depuis `MyTeamRow` + `status_label_and_class()`).
Consommé par : `my-teams-widget.html`.

## Changement de contrat sur le composant partagé

`src/web/templates/components/team-card.html` — macro `card(...)` : ajout
d'un paramètre `initials: &str`, rendu comme fallback texte quand `logo` est
`None` (aujourd'hui le cercle reste vide). Askama ne supporte pas les
paramètres optionnels sur les macros — deux consommateurs à mettre à jour :

- `my-teams-widget.html` *(nouveau, utilise le paramètre)*
- `competition-teams.html` *(existant)* — doit calculer et passer ses
  propres initiales pour ne pas casser l'appel, même si ce n'est pas demandé
  par cette feature (contrat partagé).
