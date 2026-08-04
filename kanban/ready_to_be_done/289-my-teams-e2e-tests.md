# Tests E2E — page "Mes équipes"

**Priorité : haute**
**Dépend de :** `287-teams-my-teams-widget.md`, `288-team-creation-my-teams-page.md`
**Contexte :** `team_creation` + `teams` — tests Playwright

## Objectif

Couvrir en E2E le comportement réel de la page restructurée : c'est
précisément le genre de bug (statut affiché incohérent avec le statut
domaine réel) qu'aucun test unitaire n'aurait détecté seul.

**Spec de référence :** `docs/specs/my-teams/mes-equipes/07-integration.md`.

---

## Scénarios

1. Coach avec : 2 brouillons (un avec roster choisi, un sans), 2 équipes
   actives avec `game_phase` différents, 1 équipe refusée, 1 équipe
   renvoyée → les 3 sections affichent le bon nombre de cartes et les bons
   libellés (dont "Phase d'amélioration"/"Phase de recrutement" etc., pas
   des libellés inventés).
2. Clic sur "Continuer →" d'un brouillon → navigation vers la page de build
   de ce brouillon.
3. Clic sur une carte active ou archivée → navigation vers le détail de
   l'équipe correspondante.
4. Coach sans aucune équipe dans le space → les 3 sections sont absentes du
   DOM (pas de titre "0").
5. Équipe fraîchement soumise (juste après `mark_submitted`) : disparaît de
   "En cours de création" — vérifie que le bug initial (statut "Inscrite"
   affiché avant tout traitement admin) est bien corrigé.

---

## Checklist

- [ ] Fixture : coach avec équipes dans les 4 `ParticipationStatus`
- [ ] Scénario 1 — répartition et libellés des 3 sections
- [ ] Scénario 2 — navigation brouillon → build
- [ ] Scénario 3 — navigation active/archivée → détail
- [ ] Scénario 4 — état vide, aucune section affichée
- [ ] Scénario 5 — non-régression du bug initial (statut affiché = statut réel)
- [ ] Carte ajoutée à la carte d'impact tests↔bounded-contexts (skill `test-impact`)
