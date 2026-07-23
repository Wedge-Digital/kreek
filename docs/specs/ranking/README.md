# Ranking — Spec index

Nouveau bounded context `ranking` : calcule les points de classement, points bonus et scores de départage des équipes d'une compétition à partir des rapports de match publiés (BC `match_report`). Fournit le widget **Classement**, affiché en premier onglet (actif par défaut) de la page détail compétition.

## Contexte

L'onglet "Classement" existe déjà dans `competition_detail.rs` (BC `competitions`) mais sert des données 100% mockées (`mock_standings()`, cf. carte `13-mock-data-competition-detail.md`). Cette fonctionnalité remplace le mock par un vrai calcul, porté par un BC dédié.

Les règles de calcul (points victoire/nul/défaite, bonus offensif/défensif) sont définies par `competitions` (`CompetitionRules.ranking_rules`) — `ranking` les consulte via un port, il ne les possède pas.

## Périmètre feature 1

- Mise en place du bounded context `ranking`
- Calcul des points de classement (victoire/nul/défaite) — **pas** les points bonus ni les scores de départage, hors scope
- Affichage du widget Classement avec 2 états vides distincts (aucune équipe / aucun match joué)

## Progression

| Page / onglet | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| classement | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
