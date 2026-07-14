# Tests E2E — Fiche joueur

**Priorité : moyenne**
**Dépend de :** `167-players-routing-player-detail.md`
**Contexte :** `tests/e2e/` — Playwright, contrairement à la feature « player match impact » précédente (100% backend), cette page a une vraie surface HTML/HTMX, la couverture E2E standard (CLAUDE.md) s'applique donc normalement ici.

## Objectif

Vérifier le parcours réel navigateur : clic sur une ligne du tableau roster
d'une équipe → affichage de la fiche joueur avec les bonnes données.

---

## Conception

Nouveau fichier `tests/e2e/test_player_detail.py`, même conventions que les
tests existants (`conftest.py`, fixtures `space_id`/équipe).

Scénarios :
1. Depuis la fiche équipe, cliquer sur une ligne du tableau roster → navigation vers `/app/{space_id}/players/{player_id}/detail`, contenu attendu visible (nom, poste, stats, compétences).
2. Portefeuille SPP affiche bien deux nombres distincts (gagnés/dépensés) et la réserve.
3. Résumé de carrière affiche les compteurs (essais/passes/interceptions/sorties/MVP) cohérents avec les données de test.
4. Si des matchs ont été enregistrés (fixture avec au moins un `TeamMatchConcluded` + actions) : la carte d'historique correspondante s'affiche avec le bon adversaire/score/actions.
5. Bouton "✏️ Customiser" visible uniquement si le coach de test est admin de l'espace ou de la compétition ; absent sinon.
6. Bouton "▶ Activer la dépense de SPP" toujours visible (feature à part, pas de vérification de câblage).

---

## Checklist

- [ ] `test_player_detail.py` — scénario 1 à 6
- [ ] Fixture(s) nécessaires pour un joueur avec au moins un match enregistré (historique non vide)
- [ ] Documenté dans `tests/e2e/README.md` si un pattern nouveau est introduit (ex. fixture historique de match)
