# 138 — Tests E2E : onglets Résultats et Calendrier

## Objectif

Couvrir les deux onglets avec des tests Playwright end-to-end contre le serveur dev.

## Dépendances

- 137 (intégration complète)

## Scénarios

### Onglet Résultats

| # | Scénario | Vérification |
|---|---|---|
| R1 | Clic sur onglet Résultats | Fragment chargé, au moins une journée visible |
| R2 | Match `completed` | Score affiché (ex: "2 – 1"), pas de badge |
| R3 | Match `in_progress` | Badge "En cours de saisie" visible + lien "Accéder au rapport" |
| R4 | Logo présent | `<img>` rendu dans `.team-logo` |
| R5 | Logo absent | Initiales rendues dans `.team-logo-initials` |
| R6 | Scroll jusqu'au sentinel | 3 nouvelles journées ajoutées au DOM |
| R7 | Fin de scroll | Sentinel disparu après dernière page |
| R8 | Navigation directe sur URL `/resultats` | Full page rendue, onglet Résultats actif |

### Onglet Calendrier

| # | Scénario | Vérification |
|---|---|---|
| C1 | Clic sur onglet Calendrier | Fragment chargé, journées à venir visibles |
| C2 | Header journée `time_frame` | `date_range` affiché (ex: "25 – 26 mai") |
| C3 | Header journée `fixed_date` | Date unique affichée |
| C4 | Équipes visiteuses | Alignées à droite |
| C5 | Scroll Calendrier | 3 nouvelles journées futures ajoutées |
| C6 | Navigation directe sur URL `/calendrier` | Full page rendue, onglet Calendrier actif |

### Comportement partagé

| # | Scénario | Vérification |
|---|---|---|
| S1 | Onglet Classement par défaut | Onglet Classement actif au chargement initial |
| S2 | Switch onglet | Un seul onglet visible à la fois |

## Fichier de test

Créer `tests/e2e/test_competition_matchs.py` (ou ajouter dans un fichier compétition existant).

Voir `tests/e2e/README.md` pour le setup (`make e2e`, serveur dev requis).

## Checklist

- [ ] Fixtures de données : au moins 2 saisons avec pairings et statuts variés
- [ ] Scénarios R1–R8 implémentés
- [ ] Scénarios C1–C6 implémentés
- [ ] Scénarios S1–S2 implémentés
- [ ] `make e2e` passe (serveur dev lancé)
