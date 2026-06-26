# Step 2 — Avant-match — Cartes kanban

## Ordre d'implémentation

| # | Carte | Dépendance |
|---|---|---|
| 1 | Value object D3Roll + erreur domaine | — |
| 2 | Événement FanFactorRecorded + rehydratation + projection | Carte 1 |
| 3 | Méthode agrégat PreMatch::record_fan_factor | Carte 1, 2 |
| 4 | Use case record_fan_factor | Carte 3 |
| 5 | Endpoint JSON team-match-context (BC Teams) | — |
| 6 | Page step 2 : handler GET + template + routes | Carte 5 |
| 7 | Page step 2 : handler POST + wiring | Carte 4, 6 |
| 8 | Tests E2E step 2 | Carte 7 |

## Cartes

### Carte 1 — Value object D3Roll

**Objectif** : créer le value object D3Roll avec smart constructor.

**Fichiers** :
- `src/app/match_report/domain/value_objects.rs` — ajouter `D3Roll`
- `src/app/match_report/domain/error.rs` — ajouter `InvalidD3Roll(u8)`

**Tests** :
- `d3roll_accepte_1_2_3`
- `d3roll_rejette_0_et_4`

---

### Carte 2 — Événement FanFactorRecorded + rehydratation

**Objectif** : ajouter l'événement domaine et sa prise en charge dans la machine d'états.

**Fichiers** :
- `src/app/match_report/domain/events.rs` — ajouter variante `FanFactorRecorded`
- `src/app/match_report/domain/match_report_pre_match.rs` — ajouter champs `home_fan_roll`, `away_fan_roll`
- `src/app/match_report/domain/match_report_state.rs` — ajouter bras rehydratation
- `src/app/match_report/io/repository/match_report_repository.rs` — ajouter bras vide dans `update_projection_in_tx`

**Tests** :
- `rehydratation_fan_factor`
- `rehydratation_double_fan_factor`

---

### Carte 3 — Méthode agrégat PreMatch::record_fan_factor

**Objectif** : implémenter la méthode métier sur l'agrégat.

**Fichier** :
- `src/app/match_report/domain/match_report_pre_match.rs`

**Tests** :
- `record_fan_factor_emet_evenement`
- `record_fan_factor_met_a_jour_les_champs`

---

### Carte 4 — Use case record_fan_factor

**Objectif** : orchestration charge agrégat → appel méthode domaine → persistance.

**Fichiers** :
- `src/app/match_report/use_cases/record_fan_factor_use_case.rs` (nouveau)
- `src/app/match_report/use_cases/mod.rs` — ajouter module

---

### Carte 5 — Endpoint JSON team-match-context (BC Teams)

**Objectif** : fournir les données d'équipe nécessaires à la page step 2 (dedicated fans, player count, CTV, treasury, journeyman type).

**Fichiers** :
- `src/app/teams/io/web/widgets/team_match_context_widget.rs` (nouveau)
- `src/app/teams/io/web/widgets/mod.rs` — ajouter module
- `src/app/teams/routes.rs` — ajouter route
- `src/app/teams/router.rs` — ajouter handler

**Dépendance données** : vérifier que le repository/projection Teams expose les champs nécessaires. Si manquants, étendre la projection.

---

### Carte 6 — Page step 2 : handler GET + template

**Objectif** : afficher la page d'avant-match avec les données du match report + chargement client des données d'équipe.

**Fichiers** :
- `src/app/match_report/io/web/pre_match_controller.rs` (nouveau) — handler GET
- `src/app/match_report/io/web/templates/pre-match.html` (nouveau)
- `src/app/match_report/io/web/mod.rs` — ajouter module
- `src/app/match_report/routes.rs` — ajouter route MATCH_REPORT_STEP2
- `src/app/match_report/router.rs` — ajouter route

**Template** : Alpine x-data avec fetch des données d'équipe, calcul temps réel fan factor/TV/inducements.

---

### Carte 7 — Page step 2 : handler POST + wiring

**Objectif** : gérer la soumission du formulaire fan factor.

**Fichiers** :
- `src/app/match_report/io/web/pre_match_controller.rs` — ajouter handler POST
- `src/app/match_report/router.rs` — ajouter POST sur la route

---

### Carte 8 — Tests E2E step 2

**Objectif** : valider le parcours complet en navigateur.

**Scénarios** :
- Accès step 2 sur un match report PreMatch
- Saisie fan factor + soumission
- Soumission invalide (D3 hors range)
- Redirect si état Draft
- Affichage données équipe (journaliers, TV, inducements)
