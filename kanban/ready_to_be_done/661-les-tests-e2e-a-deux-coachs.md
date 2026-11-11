# Les tests e2e à deux coachs

**Épic :** E22 — Le mode « Chacun son équipe » · **Ordre :** 10 · **Dépend de :** 652 à 660
**Conception :** `docs/specs/saisie-mobile/E22-cadrage.md` (§ 7)

## Le constat

Chaque carte de l'épic porte ses e2e. Aucune ne rejoue le « Terminé quand » de
bout en bout, ni n'éprouve d'un seul geste toutes les familles de routes contre
une écriture dans le camp adverse.

## Ce qui change

Un fichier `tests/e2e/test_match_report_team_entry.py`, sur le modèle de
`test_realtime_three_matches.py` :

- **deux contextes** : DevCoach (domicile) et un coach **réellement connecté**
  (`realtime_helpers.login`) ; saisies par `http_session_of(context)` pour que
  l'auteur soit la vraie identité ; `wait_for_stream` avant d'attendre un
  signal, `assert_none_after` pour une absence, `cliquer_quand_cable` après
  tout swap ; pas de `networkidle` ;
- **le parcours** : démarrage en « Chacun son équipe » depuis la carte
  « Qui saisit ? » ; chacun son fan factor ; l'outsider attend puis achète après
  « J'ai fini mes achats » du favori ; chacun ses actions, son MVP, ses gains,
  son paragraphe ; la seconde validation publie ; le récapitulatif montre les
  deux paragraphes ;
- **les refus** : un POST direct dans le camp adverse sur chaque famille de
  routes (`step2`, coups de pouce, mercenaires, actions, suppression, MVP,
  tirage, `step5`, validation) rend 403 ;
- **le démarrage concurrent** : les deux coachs cliquent « Commencer » ; un seul
  mode est retenu, l'autre arrive sur l'état du match.

`match_report_helpers.create_draft` gagne `entry_mode=None` (absent =
`SoloEntry`) ; un helper `start_team_entry` démarre un match en mode B.

## Fichiers touchés

`tests/e2e/test_match_report_team_entry.py` (nouveau),
`tests/e2e/match_report_helpers.py`, `tests/impact-map.toml`.

## Règles et invariants

- Aucune attente fixe : une condition qui rend la main dès qu'elle est vraie.
- Le mode A reste éprouvé par `test_match_report_solo_entry.py`, inchangé.

## Tests

Cette carte est le test.

## Conséquences sur les données de production

Aucune : la carte n'ajoute que des tests, sur la base `kreek_e2e`.

## Terminé quand

Le scénario passe en CI : deux coachs, chacun dans son navigateur, saisissent
le même match en « Chacun son équipe », toute écriture dans le camp de l'autre
est refusée, et le compte rendu se publie à la seconde validation avec les deux
paragraphes du résumé.
