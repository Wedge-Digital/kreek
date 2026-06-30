# BC match_report — E2E tests step2 mercenaires

**Priorité : haute**
**Dépend de :** 128
**Contexte :** `docs/specs/match-report/step2-mercenaires/07-integration.md`

## Objectif

Écrire les tests E2E Playwright pour le flow mercenaires du step 2.

## Conception

### Fichier

`tests/e2e/test_match_report_step2_mercenaires.py`

### Prérequis données

Même setup que `test_match_report_step2_inducements.py` :
- Deux équipes en `ReadyToPlay`, fan factors enregistrés, TV enregistrées
- Match report en phase inducements (fan factor + TV saisis)
- Équipe dont le roster a au moins 2 positions de joueurs non-journaliers
- Idéalement : une position à max_qty (pour tester le cas disabled)

### Scénarios

| ID | Test | Description |
|----|------|-------------|
| TC-MERC-01 | `test_mercenaires_tab_visible_and_widget_loads` | Tab Mercenaires présent, clic → widget se charge |
| TC-MERC-02 | `test_journaliers_excluded_from_grid` | Positions journalier absentes de la grille |
| TC-MERC-03 | `test_position_click_shows_hire_panel` | Clic carte → hire panel + prix corrects |
| TC-MERC-04 | `test_recruit_base_adds_to_cart` | Clic "Recruter" → compteur 1/3, résumé panier |
| TC-MERC-05 | `test_max_3_mercenaires_frontend` | 3 mercos → bouton disabled ou ajout bloqué |
| TC-MERC-06 | `test_remove_mercenaire_from_cart` | Clic ✕ → compteur réduit, carte redevient active |
| TC-MERC-07 | `test_full_position_card_disabled` | Position à max_qty → carte disabled, hire panel inaccessible |
| TC-MERC-08 | `test_submit_with_mercenaire_creates_temp_player` | Formulaire soumis → step 3, temp player mercenaire visible |
| TC-MERC-09 | `test_submit_without_mercenaires_regression` | 0 mercenaires + inducements classiques → OK (non-régression) |

### Notes d'implémentation

- TC-MERC-01 : vérifier que `hx-trigger="mercenairesActivated from:body once"` charge le widget au clic de l'onglet
- TC-MERC-03 : vérifier les prix (`base_cost + 30 kPo` et `base_cost + 80 kPo`)
- TC-MERC-07 : nécessite un setup fixture avec une équipe ayant une position pleine
- TC-MERC-08 : naviguer jusqu'au sélecteur de joueurs en step 3 et chercher l'entrée mercenaire

## Checklist

- [ ] TC-MERC-01 : tab visible + widget chargé
- [ ] TC-MERC-02 : journaliers exclus
- [ ] TC-MERC-03 : hire panel + prix
- [ ] TC-MERC-04 : ajout au panier
- [ ] TC-MERC-05 : max 3 enforced
- [ ] TC-MERC-06 : suppression ✕
- [ ] TC-MERC-07 : position full = disabled
- [ ] TC-MERC-08 : soumission + temp player step 3
- [ ] TC-MERC-09 : non-régression sans mercenaires
- [ ] `make e2e` passe (avec serveur dev lancé)
