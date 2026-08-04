# Phase 2 — Architecture front — page "Mes équipes"

**Maquette de référence :** `assets/rawpages/html/app-my-teams.html`
(validée, sections brouillons / actives / archivées, TV retirée des cartes
actives).

## Widgets

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| Section brouillons | `team_creation` | *(rendu direct par le handler hôte, pas d'endpoint séparé)* | — | — | lecture seule |
| Section équipes actives + archivées | `teams` | `GET /app/{space_id}/team/widgets/my-teams` *(nouveau)* | `hx-trigger="load"` au chargement de la page | — | lecture seule |

- La section brouillons n'est pas un widget HTMX séparé : c'est la donnée
  propre de la page hôte (BC `team_creation`), rendue dans la même réponse
  que la page — comme aujourd'hui.
- La section active/archivée est un seul widget BC `teams` : une requête
  `find_by_coach_and_space`, regroupée en deux blocs dans le même fragment
  HTML retourné (décision validée, cf. README).

## Communication entre widgets

Aucune. Page intégralement en lecture seule : pas de filtre, pas
d'interaction croisée entre sections. Chaque carte (brouillon, active,
archivée) est un lien de navigation HTMX (`hx-get` + `hx-target="#app-content"`
+ `hx-select="#app-content"` + `hx-swap="outerHTML"` + `hx-push-url="true"`,
pattern déjà utilisé partout dans l'appli) vers la page de build ou la page
de détail d'équipe. Aucun événement DOM sur `body` requis.

## Front vs back

Tout est backend : aucune logique JS/Alpine requise (pas de toggle, pas de
filtre local, pas de drag & drop).

## Widgets existants réutilisables

Vérifié dans `src/app/teams/io/web/widgets/` : `pending_enrollment_widget.rs`
et `enrolled_teams_widget.rs` existent mais sont paramétrés par
`competition_id`/`season_id` (vue admin), pas par coach — aucun n'est
réutilisable tel quel, mais leur structure (`Template` + `IntoResponse` + VM
dédiée) sert de patron pour le nouveau widget.

Composant de carte partagé `src/web/templates/components/team-card.html`
(macro `card(name, logo, roster, coach, tv, status, status_label, link)`) :
masque déjà `coach` si vide et `tv` si `0` → réutilisable tel quel pour les
cartes actives. Les brouillons et les archivées ont un style visuel distinct
dans la maquette (ligne horizontale vs carte compacte grisée) → nouveaux
gabarits à prévoir, détaillé en phase 3/7.

## Note

`kanban/ready_to_be_done/44-my-teams-page.md` existe déjà mais date d'avant
les simplifications validées (budget, filtres, pas de section archivée, pas
de statut "Refusée"). Il sera remplacé en phase 8, pas complété.

## Règle métier ouverte pour la phase 6

À quel moment une équipe passe-t-elle d'"active" à "archivée" ? Pas de
statut `ParticipationStatus` existant pour ça — traité formellement en
phase 6 (domaine).
