# BC `teams` — Tests E2E du bandeau d'état contextuel

**Priorité : moyenne**
**Dépend de :** `171-teams-team-detail-state-banner.md`, `172-teams-match-report-published-listener.md`
**Contexte :** `tests/e2e` — Playwright

## Objectif

Couvrir en navigateur réel le rendu et les transitions du bandeau d'état sur
la page de détail d'équipe, pour les scénarios qu'aucun test unitaire ne peut
détecter (rendu HTML/HTMX réel, disparition/apparition du bandeau après
rechargement). Spec complète :
`docs/specs/team-state-management/team-detail/02-07-conception.md` (Phase 7).

---

## Conception

Utiliser `override_phase()` (admin, déjà existant) ou des fixtures SQL
d'événements pour placer une équipe de test dans chaque phase avant de charger
la page — pas de dépendance à un vrai enchaînement de matchs.

### Scénarios

1. Équipe `PendingEnrollment` → bandeau informatif visible, aucun bouton.
2. Équipe `Enrolled` + `ReadyToPlay` → bouton "Imprimer en PDF" visible.
3. Équipe `Enrolled` + `MatchReporting` → clic sur "Reprendre le rapport →" navigue vers la bonne étape du rapport en cours.
4. Équipe `Enrolled` + `PlayerImprovement` → clic "Évolutions terminées" → page rechargée, badge "Phase de recrutement".
5. Équipe `Enrolled` + `Recruitment` → clic "Terminer les achats" → badge "Phase de renvois".
6. Équipe `Enrolled` + `Dismissals` → clic "Valider les renvois" → badge "Prête à jouer".
7. Équipe `Enrolled` + `TemporaryRetirement` (ou `OffSeason`) → aucun bandeau affiché, seul le badge d'en-tête visible.

---

## Checklist

- [ ] Fixture / helper pour placer une équipe dans une phase donnée (via `override_phase` ou insertion directe dans `team_event_store`)
- [ ] 7 scénarios Playwright listés ci-dessus
- [ ] Exécution documentée dans `tests/e2e/README.md` si nouvelle commande nécessaire
