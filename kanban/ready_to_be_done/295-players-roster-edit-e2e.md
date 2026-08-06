# Tests E2E — édition de l'effectif (team-detail)

**Priorité : haute**
**Dépend de :** `293-players-roster-edit-widget.md`, `294-players-roster-edit-save-endpoint.md`
**Contexte :** `players` + `teams` — tests Playwright

## Objectif

Couvrir en E2E le comportement réel du mode édition : c'est une
fonctionnalité à coordination cross-BC par événements DOM (bandeau `teams` ↔
widget `players`) — précisément le genre d'interaction qu'aucun test
unitaire ne couvre seul.

**Spec de référence :** `docs/specs/player-edition/team-detail/07-integration.md`.

---

## Scénarios

1. Renommer un joueur, enregistrer, recharger la page → le nom persiste.
2. Changer un numéro de maillot, enregistrer, recharger → persiste.
3. Vider un numéro de maillot → affiché `—` en lecture après sauvegarde.
4. Réordonner deux joueurs par glisser-déposer, enregistrer, recharger → le
   nouvel ordre persiste.
5. Saisir un numéro déjà pris par un autre joueur actif → « Enregistrer »
   désactivé, message de doublon visible, sans requête réseau.
6. Renvoyer un joueur puis attribuer son ancien numéro à un autre joueur
   actif → succès (un `Dismissed` ne bloque rien).
7. Quitter l'état « Prête à jouer » (vraie transition de phase, pas le
   sélecteur de démo) pendant l'édition → mode édition fermé proprement.
8. Utilisateur sans droit (ni coach, ni admin d'espace/compétition) →
   requête refusée (403).

---

## Checklist

- [ ] Fixture : équipe `Active` en état « Prête à jouer », effectif avec
      plusieurs joueurs `Active` + au moins un `Dismissed`
- [ ] Scénario 1 — renommage persiste
- [ ] Scénario 2 — renumérotation persiste
- [ ] Scénario 3 — retrait de numéro
- [ ] Scénario 4 — réordonnancement persiste
- [ ] Scénario 5 — doublon bloqué front, pas de requête
- [ ] Scénario 6 — numéro d'un `Dismissed` réutilisable
- [ ] Scénario 7 — sortie propre si l'état change en cours d'édition
- [ ] Scénario 8 — autorisation refusée
- [ ] Carte ajoutée à la carte d'impact tests↔bounded-contexts (skill `test-impact`)
