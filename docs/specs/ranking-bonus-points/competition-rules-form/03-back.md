# Phase 3 — Architecture back (competition-rules-form)

## Mapping BC

Tout est **intra-`competitions`** : définir et persister les règles de classement
est une opération interne au BC competitions. Pas de widget, pas de communication
cross-BC, pas de port, pas de domain service pour cette unité.

(La propagation vers `ranking` et le calcul sont l'affaire de l'unité
`post-match-bonus-calc`.)

## Fichiers impactés

| Fichier | Nature | Détail |
|---|---|---|
| `competitions/domain/competition_rules.rs` | Struct domaine (conçu en Phase 6) | VO `MaxTdConceded` + champ `max_td_conceded` sur `DefensiveBonus` ; VO `MinCasualties` + struct `AggressiveBonus` ; champ `aggressive_bonus` sur `RankingRules` ; `#[serde(default)]` pour rétro-compat JSONB |
| `competitions/io/web/templates/new-competition-phase-2.html` | Template + JS inline | Markup (Phase 1) + `buildJSON()` + `initFromExistingRules()` |
| `competitions/io/web/rules_labels.rs` | **Nouveau** helper présentation | `format_bonus_label(&RankingRules) -> Option<String>` — factorise le label bonus |
| `competitions/io/web/new_competition_phase_5.rs` | Affichage récap | Remplace le bloc inline par un appel à `format_bonus_label` |
| `competitions/io/web/admin/summary_tab.rs` | Affichage récap | Idem |

## Décision d'architecture — extraction du helper de label

Le formatage du label bonus est aujourd'hui **copié-collé** dans
`new_competition_phase_5.rs` (~135-155) et `admin/summary_tab.rs` (~192-209), avec
le seuil défensif « ≤ 1 » codé en dur. L'ajout du bonus agressif en ferait une
**triple** duplication.

→ Extraction dans un helper de **présentation** `competitions/io/web/rules_labels.rs` :

```
pub fn format_bonus_label(rr: &RankingRules) -> Option<String>
```

- Produit le libellé FR combiné (Offensif / Défensif / Agressif) selon les flags
  `activated` et les seuils configurés.
- Vit en **couche IO/web** (texte FR = présentation), jamais dans le domaine.
- Consommé par les deux fichiers récap ; respecte la règle des 20 lignes (chaque
  sous-formatage extrait en fonction courte si besoin).

## Ce qui NE change PAS

- **Routes** : aucune nouvelle. Le POST existant (path courant, `submitRules()`) est
  réutilisé.
- **Handler POST** `post_competition_rules` / `SaveRulesPayload`
  (`new_competition.rs:411-462`) : **aucun changement** — les nouveaux champs
  transitent via `#[serde(flatten)] rules: CompetitionRules` sans toucher au handler.
- **Use case** `save_competition_rules::execute` : **aucun changement** (valide
  l'unicité roster/tier, n'inspecte pas les bonus).
- **Ports / domain services / adapters** : aucun pour cette unité.

## Règle métier à cette étape

Aucune nouvelle.