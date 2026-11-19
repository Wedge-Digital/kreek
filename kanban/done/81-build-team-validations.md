# BC `team_creation` — Validations et redirections build-team / finalize-team

**Priorité : haute**
**Dépend de :** rien
**Contexte :** BC `team_creation` — pages build-team + finalize-team

## Objectif

1. Empêcher le passage en phase de finalisation si l'équipe ne respecte pas les prérequis
2. Implémenter les règles de redirection entre build-team, finalize-team et page équipe

---

## Validations à appliquer avant finalisation

| Règle | Source | Message d'erreur |
|---|---|---|
| Minimum 11 joueurs recrutés | `MIN_PLAYERS_FOR_SUBMISSION` (domaine) | "Vous devez recruter au moins 11 joueurs." |
| Un roster doit être sélectionné | `roster.id` non vide | "Sélectionnez un roster." |
| Budget non dépassé | `remaining_budget() >= 0` | "Le budget est dépassé." |

Le handler GET `finalize_team` doit valider ces prérequis **avant** de rendre la page. Si non respectés, retourne une erreur visible depuis build-team.

---

## Règles de redirection

### Depuis build-team (bouton "Terminer la construction")

| Condition | Redirection |
|---|---|
| Prérequis non remplis (< 11 joueurs, pas de roster, budget dépassé) | Reste sur build-team avec message d'erreur |
| Prérequis OK + SPP à dépenser (`spp_pool > 0`) | Redirige vers la page de finalisation |
| Prérequis OK + pas de SPP + plusieurs ligues | Redirige vers la page de finalisation (choix de ligue) |
| Prérequis OK + pas de SPP + une seule ligue | Auto-submit → redirige vers la page de l'équipe (BC teams) |

### Depuis finalize-team (bouton "Soumettre l'équipe")

| Condition | Redirection |
|---|---|
| Soumission réussie | Redirige vers la page de l'équipe (BC teams) : `/app/{space_id}/team/{team_id}/detail` |
| Erreur de validation | Reste sur finalize-team avec messages d'erreur |

### Depuis finalize-team (auto-skip, pas de finalisation nécessaire)

Le handler GET `finalize_team` détecte que la finalisation n'est pas nécessaire → auto-set league + auto-submit → redirige vers la page de l'équipe.

---

## Plan

1. Handler GET `finalize_team` : ajouter validation des prérequis avant de rendre la page
2. Handler GET `finalize_team` : changer la redirection auto-skip de `my_teams` vers `team_detail`
3. Handler POST `finalize_team` : changer la redirection de `my_teams` vers `team_detail`
4. Handler `submit_team` (build-team) : changer la redirection de `my_teams` vers `team_detail`
5. Cart widget : afficher le compteur de joueurs (ex. "7/11 joueurs")

---

## Checklist

- [ ] Handler GET `finalize_team` : valider minimum 11 joueurs avant de rendre la page
- [ ] Si validation échoue : retourner un message d'erreur visible depuis build-team
- [ ] Redirection auto-skip : `my_teams` → `team_detail`
- [ ] Redirection POST finalize : `my_teams` → `team_detail`
- [ ] Redirection submit_team (build) : `my_teams` → `team_detail`
- [ ] Cart widget : afficher le compteur de joueurs recrutés vs minimum
- [ ] Test : impossible de passer en finalisation avec moins de 11 joueurs
