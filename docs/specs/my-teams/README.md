# My teams — Spec index

Restructuration de la page "Mes équipes" (`/app/{space_id}/team/list`) en
trois sections respectant la souveraineté des données entre BCs :
brouillons (BC `team_creation`), équipes actives et équipes archivées
(BC `teams`, statut réel `ParticipationStatus` + `game_phase`).

Contexte : le statut affiché sur cette page ne reflétait pas le vrai statut
domaine (booléen local `submitted_at` côté `team_creation`, jamais
resynchronisé avec `ParticipationStatus` côté `teams`). La correction retenue
n'est pas une lecture cross-BC (port/adapter), mais une séparation en widgets
propres à chaque BC — cf. maquette validée
`assets/rawpages/html/app-my-teams.html`.

## Pages

| Page | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| mes-equipes | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (5 cartes créées : 285-289) |

## Règles métier transverses (identifiées phase 1-2)

- **Un seul widget pour actives + archivées** : une requête
  `find_by_coach_and_space` côté `teams`, regroupée en deux blocs dans le même
  fragment HTML — pas deux endpoints séparés. Décision prise en phase 2 :
  volume archivé jugé modeste (quelques équipes par coach sur plusieurs
  saisons), pas de lazy-load nécessaire pour l'instant.
- **Statut "archivée" sans équivalent domaine actuel** : `ParticipationStatus`
  ne connaît que `PendingEnrollment` / `Enrolled` / `Dismissed` / `Rejected`.
  À trancher en phase 6 (domaine) : nouveau statut, ou dérivé d'autre chose
  (ex. saison terminée) ?
- **Page intégralement en lecture seule** : pas de filtre, pas de mutation,
  pas d'événement DOM entre sections — chaque carte est un lien de
  navigation HTMX standard.
