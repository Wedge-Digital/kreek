# BC `team_creation` — Validations de la page build-team avant finalisation

**Priorité : haute**
**Dépend de :** rien
**Contexte :** BC `team_creation` — page build-team + handler finalize GET

## Objectif

Empêcher le passage en phase de finalisation si l'équipe ne respecte pas les prérequis. Aujourd'hui le bouton "Terminer la construction" est toujours actif et le handler finalize_team GET ne valide pas le nombre de joueurs avant de rendre la page.

---

## Validations à appliquer

### Avant de quitter la page build-team

| Règle | Source | Message d'erreur |
|---|---|---|
| Minimum 11 joueurs recrutés | `MIN_PLAYERS_FOR_SUBMISSION` (domaine) | "Vous devez recruter au moins 11 joueurs." |
| Un roster doit être sélectionné | `roster.id` non vide | "Sélectionnez un roster." |
| Budget non dépassé | `remaining_budget() >= 0` | "Le budget est dépassé." |

### Dans le handler GET finalize_team

Le handler doit valider **avant** de rendre la page de finalisation (pas seulement dans le chemin auto-skip). Si les prérequis ne sont pas remplis, il redirige vers la page build-team avec un message d'erreur.

---

## Conception

### Option A — Validation serveur uniquement

Le handler GET `finalize_team` vérifie les prérequis. Si non respectés, retourne un fragment d'erreur HTMX (ex. `HX-Retarget` vers une zone d'erreur dans la page build-team, ou `HX-Redirect` vers build-team avec un query param `?error=...`).

### Option B — Validation serveur + indication visuelle

En plus de la validation serveur :
- Le bouton "Terminer la construction" est visuellement désactivé quand les prérequis ne sont pas remplis
- Le cart widget affiche le nombre de joueurs recrutés et le minimum requis
- Un message sous le bouton indique ce qui manque

L'option B est meilleure pour l'UX mais nécessite que le cart ou un widget dédié connaisse le nombre de joueurs.

---

## Plan recommandé

1. Dans le handler GET `finalize_team` : ajouter une validation `hired_players.len() < MIN_PLAYERS_FOR_SUBMISSION` → retourner une erreur ou redirect
2. Dans le cart widget : afficher le compteur de joueurs (ex. "7/11 joueurs") et le statut (prêt ou non)
3. Le bouton "Terminer la construction" pourrait être dans le cart widget (qui connaît l'état) plutôt qu'en dur dans la page

---

## Checklist

- [ ] Handler GET `finalize_team` : valider le nombre de joueurs avant de rendre la page
- [ ] Si validation échoue : retourner un message d'erreur visible depuis build-team
- [ ] Cart widget : afficher le compteur de joueurs recrutés vs minimum
- [ ] Indication visuelle que l'équipe n'est pas prête à être finalisée
- [ ] Test : impossible de passer en finalisation avec moins de 11 joueurs
