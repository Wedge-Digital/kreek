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

    # ── TV initiale (carte 252) ──────────────────────────────────────────────
    # Ces équipes sont auto-enrôlées : `teams` atteint ReadyToPlay pendant que
    # `players` insère encore ses joueurs. Sans l'app event de fin de roster, la
    # TV se figerait sur un effectif vide — zéro, ou la seule valeur des
    # journaliers, plausible et fausse.
    #
    # L'assertion ne rejoue pas la formule (couverte par les tests unitaires de
    # `compute_team_value`) : elle vérifie que les joueurs existent bien et que
    # la TV les intègre. Journaliers, staff et relances ne peuvent qu'ajouter,
    # d'où la comparaison par supériorité.
    for team_id in team_ids:
        tv = int(query_db(f"SELECT team_value FROM team_proj WHERE team_id = '{team_id}';")[0])
        somme_joueurs = int(
            query_db(
                f"SELECT COALESCE(SUM(value_kpo), 0) FROM players_proj "
                f"WHERE team_id = '{team_id}' AND participation_status = 'Available';"
            )[0]
        )

        assert somme_joueurs > 0, f"équipe {team_id} : aucun joueur valorisé, le roster n'a pas été créé"
        assert tv > 0, f"équipe {team_id} : TV initiale nulle, le recalcul n'a pas eu lieu"
        assert tv >= somme_joueurs, (
            f"équipe {team_id} : TV {tv} inférieure à la somme de ses joueurs {somme_joueurs} — "
            "la TV a été calculée sur un effectif incomplet"
        )
