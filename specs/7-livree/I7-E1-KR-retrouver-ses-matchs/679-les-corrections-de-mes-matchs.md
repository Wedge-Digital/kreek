---
id: 679-les-corrections-de-mes-matchs
parent: I7-E1-KR
ticket: I7-E1-KR-T5
status: done
---

# Les corrections de « Mes matchs »

**Epic :** I7-E1-KR · **Dépend de :** 678

Deux défauts trouvés par les tests e2e de la carte 678.

## Le constat

1. **« Continuer » ne ramène pas à l'étape en cours.** « Continuer » passe par
   `from_pairing`, qui redirige vers `edit_match_report`. Dans
   `src/app/match_report/io/web/match_selection_controller.rs`, `edit_page`
   renvoie tout rapport en phase `PreMatch` sur `/step2`. Or cette phase couvre
   l'avant-match **et** le match lui-même : en « Je saisis tout », un coach dont
   le match a commencé retombe sur le formulaire du fan factor au lieu de
   `/live`. En « Chacun son équipe », `kick_off_redirect` ne rattrape que le cas
   inverse (`/live` avant le coup d'envoi).
2. **Le widget épuise le pool de connexions.** `entry_actions_for_pairings`
   (`src/app/match_report/use_cases/match_entry_action_service.rs`) lance un
   `join_all` sans limite : un contrôle d'accès par rapport, tous en même temps.
   En e2e, une ouverture de la page par un coach de 120 équipes a pris 98
   connexions sur 100, et Postgres a refusé toute connexion pendant
   l'`idle_timeout`. En production (pool à 20), la page monopoliserait le pool.

## Ce qui change

- **`edit_page`** : un rapport `PreMatch` dont le match a commencé
  (`MatchReportPreMatch::has_kicked_off`, la question du domaine déjà posée par
  `kick_off_service`) mène à `/live` ; avant le coup d'envoi, `/step2` comme
  aujourd'hui. Valable dans les deux modes de saisie.
- **`entry_actions_for_pairings`** : le nombre de contrôles d'accès simultanés
  est borné par une petite constante nommée, indépendante du nombre de matchs.
  Si une lecture groupée des données d'accès s'écrit simplement, elle est
  préférable ; sinon, une concurrence bornée.
- **`tests/e2e/test_my_matches.py`** : le marqueur `xfail` de
  `test_s2_continuer_ramene_a_l_etape_en_cours` est retiré.

## Règles et invariants

- La règle d'écriture reste `can_write_report` ; aucune règle d'accès ne change.
- Un contrôle d'accès échoue fermé, comme avant.

## Tests

- Unitaires : l'aiguillage de `edit_page` avant et après le coup d'envoi, dans
  les deux modes ; la borne de concurrence (un double de port qui compte les
  appels simultanés).
- E2E : S2 passe sans `xfail`.

## Conséquences sur les données de production

Aucune.

## Terminé quand

« Continuer » ramène à l'étape en cours, la page d'un coach qui a beaucoup
d'équipes n'occupe qu'un petit nombre de connexions, et `test_my_matches` passe
en entier.
