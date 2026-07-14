# Player match impact — Test d'intégration bout-en-bout

**Priorité : moyenne**
**Dépend de :** `157-match-report-publisher-player-events.md`, `158-players-match-impact-listeners.md`
**Contexte :** test d'intégration cross-BC, pas de test E2E Playwright

## Objectif

Vérifier le pipeline complet `match_report` (publication) → app event bus →
`players` (agrégat final), sans passer par une interaction navigateur — cette
feature n'a **aucune surface HTML/HTMX/Alpine**, la couverture E2E Playwright
habituelle (CLAUDE.md) est donc sans objet ici. Ce test la remplace. Voir
`docs/specs/player-match-impact/player-report-events/07-integration.md` §7 pour la
justification complète de cette déviation assumée.

---

## Conception

Nouveau fichier de test d'intégration (convention du projet pour les tests
cross-composants contre une vraie base — pas de mock sqlx).

Scénario :

1. Construire un `MatchReportPublished` factice (agrégat `match_report` complet,
   avec `home_actions`/`away_actions` couvrant chaque type d'action : touchdown,
   passe, interception, sortie, mvp, agression, et une blessure de chaque type y
   compris `Sequel{stat}`), pour des joueurs `Regular` connus.
2. Publier le domain event correspondant sur le bus interne `match_report`, avec
   les deux listeners/publishers réels démarrés (`match_report_app_event_publisher`,
   `player_match_impact_listener`, `team_match_concluded_listener`) contre une
   vraie `PgPool`.
3. Attendre la propagation (les listeners tournent en tâches async — polling ou
   petit délai, cohérent avec les autres tests d'intégration événementiels du
   projet si un pattern existe déjà).
4. Charger l'agrégat `Player` final (`find_by_id`) pour chaque joueur impliqué et
   vérifier : SPP crédité au total attendu, chaque compteur de carrière incrémenté
   correctement, `injuries` contient toutes les blessures, `stat_adjustments`
   contient l'ajustement de la séquelle, `participation_status` cohérent avec la
   dernière blessure appliquée.
5. Publier un second rapport (même équipe) ne concernant pas le joueur blessé,
   vérifier que `TeamMatchConcluded` restaure `participation_status = Available`.

---

## Checklist

- [ ] Fixture `MatchReportPublished` couvrant tous les types d'action et de blessure
- [ ] Démarrage réel des listeners/publisher contre une vraie PgPool (pas de mock)
- [ ] Assertions sur l'agrégat `Player` final (SPP, compteurs, injuries, stat_adjustments, statut)
- [ ] Scénario de restauration `MissingNextGame` → `Available` via un second `TeamMatchConcluded`
