"""Test E2E de couverture : le pipeline complet "créer une compétition,
construire et soumettre des équipes, générer le calendrier" fonctionne
réellement de bout en bout.

Avant ce test, rien ne vérifiait ce pipeline — les fichiers e2e qui ont
besoin d'une compétition avec des équipes inscrites et des journées
programmées supposaient simplement que cette donnée existait déjà en base
(accumulée à la main au fil du temps, jamais reproduite par un script). La
fixture `full_competition` (conftest.py) réutilise ce même pipeline pour
fournir cette donnée à tous les autres fichiers e2e.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

from db_helpers import query_db

from competition_lifecycle import build_and_submit_team, create_full_competition, sync_and_generate_schedule

NUM_TEAMS = 12
NUM_ROUNDS = 7


def test_full_competition_lifecycle_produces_enrolled_teams_and_schedule(page, space_id, competition_create_url):
    competition = create_full_competition(page, competition_create_url, num_rounds=NUM_ROUNDS)

    team_ids = [
        build_and_submit_team(page, space_id, competition["name"], coach_option_index=i, roster_index=i)
        for i in range(NUM_TEAMS)
    ]

    sync_and_generate_schedule(page, space_id, competition["competition_id"], competition["season_id"])

    enrolled = query_db(
        f"SELECT team_id FROM team_proj WHERE season_id = '{competition['season_id']}' AND status = 'Enrolled';"
    )
    assert len(enrolled) == NUM_TEAMS, f"attendu {NUM_TEAMS} équipes Enrolled, trouvé {len(enrolled)}"
    assert set(enrolled) == set(team_ids), "les équipes Enrolled ne correspondent pas à celles construites par le test"

    rounds = query_db(
        f"SELECT id FROM competition_match_days WHERE season_id = '{competition['season_id']}';"
    )
    assert len(rounds) == NUM_ROUNDS, f"attendu {NUM_ROUNDS} journées, trouvé {len(rounds)}"

    pairings = query_db(
        f"SELECT match_day_id FROM competition_match_day_pairings "
        f"WHERE match_day_id IN (SELECT id FROM competition_match_days WHERE season_id = '{competition['season_id']}');"
    )
    assert len(pairings) > 0, "aucun pairing généré pour cette saison"
