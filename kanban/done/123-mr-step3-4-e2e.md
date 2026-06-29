# BC match_report — E2E tests step3-4-actions

**Priorité : normale**
**Dépend de :** 122
**Contexte :** match_report step3-4-actions — tests end-to-end

## Objectif

Couvrir les parcours utilisateur des étapes 3 et 4 par des tests Playwright contre le serveur dev.

## Conception

Cf. `docs/specs/match-report/step3-4-actions/02-front.md`

### Fichier

`tests/e2e/test_match_report_step3_4_actions.py`

### Scénarios

| ID | Scénario | Étapes clés |
|---|---|---|
| S1 | Chargement page step3 | Navigate → vérifier les 5 widgets présents |
| S2 | Sélectionner un tour | Clic tour 3 → turn-selector indique tour 3 sélectionné |
| S3 | Sélectionner un joueur régulier | Clic joueur → action-panel visible |
| S4 | Enregistrer un TD | Clic TD → action-log affiche 1 action (tour 3, joueur, TD) |
| S5 | Enregistrer un Blessé · Amoché | Clic Blessé → sélecteur blessure → Amoché → action-log affiche "Blessé · Amoché" |
| S6 | Enregistrer un Blessé · Séquelle −AV | Flux complet 3 étapes → action-log affiche "Blessé · Séquelle −AV" |
| S7 | Supprimer une action | Clic supprimer → action disparaît du log |
| S8 | Plusieurs actions même tour | Enregistrer TD + MVP tour 3 → log affiche 2 lignes |
| S9 | Joueur temporaire visible si inducements soumis | Star player visible dans temp-player-selector |
| S10 | Journalier ajouté automatiquement | Équipe < 11 joueurs → N journaliers dans temp-player-selector |
| S11 | Page step4 — équipe away | Même parcours que step3, équipe away |

### Préconditions fixtures

- Un match report en état `PreMatch` avec TV enregistrées et inducements soumis pour les deux équipes
- Au moins un joueur régulier pour chaque équipe
- Star player engagée pour l'équipe domicile (pour S9)

## Checklist

- [ ] Fixture SQL qui crée le match report + events jusqu'à `InducementsRecorded`
- [ ] S1 — chargement page step3
- [ ] S2 — sélection tour
- [ ] S3 — sélection joueur régulier → action-panel visible
- [ ] S4 — enregistrer TD
- [ ] S5 — Blessé · Amoché
- [ ] S6 — Blessé · Séquelle −AV
- [ ] S7 — supprimer une action
- [ ] S8 — plusieurs actions même tour
- [ ] S9 — temp player visible
- [ ] S10 — journaliers automatiques
- [ ] S11 — step4 (away)
