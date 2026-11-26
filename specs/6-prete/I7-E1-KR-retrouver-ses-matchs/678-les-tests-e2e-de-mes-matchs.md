---
id: 678-les-tests-e2e-de-mes-matchs
parent: I7-E1-KR
ticket: I7-E1-KR-T4
status: approved
---

# Les tests e2e de « Mes matchs »

**Epic :** I7-E1-KR · **Dépend de :** 677

## Le constat

- `tests/e2e/test_competition_hors_calendrier.py` (carte 550) :
  `test_l_entree_de_menu_disparait_dans_les_deux_tailles`, volet mobile, cherche
  `.mobile-tab` avec le texte « Match » et attend qu'il disparaisse quand le hors
  calendrier est interdit. Après 677, l'onglet « Matchs » reste — et `has_text`
  attrape les sous-chaînes.

## Ce qui change

- **`tests/e2e/test_my_matches.py`** (nouveau) : les scénarios S1 à S6 de I7-E1, une
  compétition dédiée par scénario (piège connu : un match en cours entre les mêmes
  équipes bloque le suivant), dont le match lointain qui reste le prochain (S1),
  « Suivre » vers la lecture seule (S3) et « Continuer » en « Chacun son équipe »
  vers l'avant-match de son camp (S4), avec deux contextes de navigateur.
- **Le volet mobile de `test_competition_hors_calendrier`** : le hors calendrier
  interdit fait disparaître « Nouveau match » de « Mes matchs », et l'onglet
  « Matchs » reste.
- `tests/impact-map.toml` : l'entrée du nouveau fichier.

## Règles et invariants

- Aucune attente fixe ; `cliquer_quand_cable` sur le contenu injecté.

## Tests

Cette carte est le test.

## Conséquences sur les données de production

Aucune.

## Terminé quand

Les six scénarios passent contre le serveur e2e, et la suite complète reste verte.
