# BC `players` — Tests E2E dépense de SPP

**Priorité : moyenne**
**Dépend de :** `182-players-spp-spending-widget.md`, `180-teams-player-improvement-app-event.md`
**Contexte :** `tests/e2e`

## Objectif

Couvrir en navigateur réel le slot journal/dépense et les achats.
Spec complète : `docs/specs/player-spp-spending/README.md`.

---

## Scénarios

1. Équipe hors phase `PlayerImprovement` → widget journal affiché, pas de panneau de dépense.
2. Équipe en phase, coach de l'équipe → panneau de dépense visible, achat d'une compétence accessible → SPP/valeur mis à jour, compétence apparaît dans les tags acquis, niveau suivant reflété au prochain achat.
3. Achat au-delà du SPP disponible → compétence non proposée/non cliquable dans `skill_picker` (réutilise son comportement existant).
4. Augmentation de caractéristique → stat + valeur mis à jour.
5. Utilisateur non autorisé (ni coach, ni admin) sur une équipe en phase `PlayerImprovement` → widget journal affiché (pas le panneau de dépense).
6. `team_value` incrémenté côté fiche équipe après achat (vérifie le pipeline app event `players → teams`).

---

## Checklist

- [ ] Fixture : équipe placée en phase `PlayerImprovement` via le vrai parcours (rapport de match publié + validation, cf. `test_team_detail_state_banner.py`)
- [ ] 6 scénarios ci-dessus
- [ ] Mise à jour du README `tests/e2e/README.md`
