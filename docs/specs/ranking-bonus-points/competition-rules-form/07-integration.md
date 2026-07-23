# Phase 7 — Effets de bord (competition-rules-form)

## 1. Persistance

**Aucune nouvelle méthode repository, aucune migration SQL.**

- La colonne `competitions.rules JSONB` (migration `20260520000002`) stocke
  l'intégralité de `CompetitionRules` sérialisé.
- `season_repository::save_rules` (`serde_json::to_string`) et `find_rules`
  (`serde_json::from_str`) sont **génériques** : les nouveaux champs bonus sont
  persistés/relus sans changement de code ni de requête SQL
  (`sql/seasons/update_rules.sql`, `select_rules.sql` inchangés).
- Rétro-compat des lignes existantes assurée par les `#[serde(default = "…")]`
  (cf. Phase 6).

## 2. Événements

**Aucun.** Sauvegarder les règles de compétition n'émet pas d'événement domaine ni
d'app event pour cette feature. Les règles sont consultées par le BC `ranking` en
**lecture synchrone via port ACL** (unité `post-match-bonus-calc`), pas par
propagation d'événement — conforme au critère « consultation → port » du CLAUDE.md.

## 3. Handlers

**Aucun handler nouveau ni modifié.**

| Handler | Rôle | Changement |
|---|---|---|
| `post_competition_rules` (`new_competition.rs`) | Reçoit le POST des règles, deser `SaveRulesPayload` | Aucun (deser via `#[serde(flatten)]`) |
| GET phase-2 (rendu formulaire) | Passe `existing_rules_json` au template | Aucun (sérialise toute la struct) |
| GET phase-5 (récap) | Construit le label bonus | Remplace le bloc inline par `format_bonus_label` |
| GET summary_tab (admin) | Construit le label bonus | Idem |

## 4. Templates & helper

| Fichier | Type | Changement |
|---|---|---|
| `templates/new-competition-phase-2.html` | Template + JS inline | Markup Phase 1 ; `buildJSON()` écrit `defensive_bonus.max_td_conceded` + bloc `aggressive_bonus` ; `initFromExistingRules()` ré-hydrate ces champs (défauts si absents) |
| `io/web/rules_labels.rs` | **Nouveau** helper présentation | `format_bonus_label(&RankingRules) -> Option<String>` : combine Offensif/Défensif/Agressif selon `activated` + seuils ; sous-formatages en fonctions courtes (règle 20 lignes) |
| `new_competition_phase_5.rs` | Contrôleur | Appelle `format_bonus_label` au lieu du bloc inline |
| `admin/summary_tab.rs` | Contrôleur | Idem |

Formats de label attendus :
- Offensif : `+{points} si ≥ {min_td} TDs`
- Défensif : `+{points} si ≤ {max_td_conceded} TD encaissé(s)` (seuil désormais dynamique)
- Agressif : `+{points} si > {min_casualties} sorties`

## 5. Tests

### Unitaires (Phase 6)
Bornes VO, désérialisation legacy → défauts, compat clé `diff_td` → champ `min_td`,
round-trip serde.

Ajouter un test unitaire du helper `format_bonus_label` : combinaisons
activé/désactivé des 3 bonus → chaîne attendue (dont seuils dynamiques).

### E2E (Playwright, `tests/e2e/`)
Scénario « saisie des bonus » :
1. Créer une compétition, atteindre l'étape 2 (Règles).
2. Activer le **bonus agressif**, saisir points X et seuil Y ; modifier le **seuil
   défensif** (≠ 1).
3. Enregistrer & continuer → étape 5 : vérifier que le récap affiche les 3 bonus
   avec les bons seuils.
4. Revenir éditer l'étape 2 : vérifier la **ré-hydratation** (agressif coché,
   valeurs X/Y et seuil défensif restaurés).
5. Cas rétro-compat (optionnel, fixture) : une compétition dont les règles JSONB ne
   contiennent pas les nouveaux champs → l'étape 2 affiche les défauts (agressif
   décoché, seuil défensif = 1) sans erreur.

## Règle métier à cette étape

Aucune nouvelle (phase de conception des effets de bord).
