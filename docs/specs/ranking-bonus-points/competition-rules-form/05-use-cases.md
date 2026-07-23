# Phase 5 — Use cases (competition-rules-form)

## Conclusion : aucun changement de use case

La seule mutation de cette unité est la sauvegarde des règles de compétition, portée
par le use case **existant** `save_competition_rules::execute`
(`competitions/use_cases/save_competition_rules.rs`). Il reste **inchangé**.

### Pourquoi aucun changement

- **Persistance générique** : les nouveaux champs bonus font partie de
  `CompetitionRules`, sérialisé **en bloc** (`serde_json`) vers la colonne
  `rules JSONB`. Le use case et le repository n'énumèrent pas les champs — ajouter
  des champs au domaine ne les touche pas.
- **Validation en amont** : les bornes des nouvelles valeurs (`MaxTdConceded`,
  `MinCasualties`) sont portées par les **value objects**, donc validées à la
  **désérialisation** du POST, avant même d'entrer dans le use case (conforme
  « Responsabilités des couches » — le handler construit la commande via smart
  constructors, le use case orchestre).
- **Pas de règle inter-champs** : le use case valide l'unicité roster/tier ; aucune
  règle transverse sur les bonus n'existe (chaque bonus est autonome — flag
  `activated` + ses seuils/points, sans dépendance à un autre bonus ni au barème
  V/N/D).

### Bilan

| Élément | Statut |
|---|---|
| Nouveau use case | Aucun |
| Modification de `save_competition_rules::execute` | Aucune |
| Nouvelle erreur applicative | Aucune |
| Orchestration | Inchangée |

## Règle métier à cette étape

Aucune nouvelle. Confirmation : chaque bonus est **autonome** (pas de contrainte
inter-champs ni de dépendance au barème V/N/D). Le calcul effectif des bonus n'est
pas dans cette unité — il est traité dans `post-match-bonus-calc`.
