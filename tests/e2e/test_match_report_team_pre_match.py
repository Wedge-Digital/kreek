"""L'avant-match de son camp (carte 654, épic E22).

En « Chacun son équipe », chaque coach lance son seul fan factor, sur
`/step2/{side}` ; celui de l'adversaire s'affiche en lecture seule dans un
widget rechargé sur le signal du match — sans recharger la page. Le premier jet
fige les valeurs des deux équipes (R4).

Scénarios :
- A1 — Deux navigateurs : le membre simple attend sur l'avant-match de
       l'extérieur ; DevCoach lance le jet du domicile ; chez le membre simple,
       « ✓ … lancé par DevCoach » apparaît sans rechargement. Puis il lance le
       sien.
- A2 — Le membre simple ne touche pas au jet du domicile : `POST
       /step2/home` est refusé par la garde de camp (403), `POST /step2` aux
       deux jets par le domaine (422) ; un second jet du domicile est refusé.

Le mode B se démarre par un POST direct (`entry_mode=TeamEntry`) : l'option
reste grisée jusqu'à la carte 659. DevCoach, admin de l'espace, coache la
première équipe que monte `build_full_competition` ; le membre simple
(`X-Bypass-Auth-Profile: simple`, « E2E Coach 01 ») la deuxième.

**La présence avant le geste** : l'onglet qui attend ne reçoit le signal
qu'une fois sa présence inscrite (`goto_declared`), comme sur `/live`.

**Pas de `networkidle`** : le flux SSE reste ouvert, l'état n'arrive jamais.

Prérequis : `make e2e-server` (port 3211, base kreek_e2e), `make init_e2e_db`.
"""

import pytest
import requests
from playwright.sync_api import expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import PROFIL_SIMPLE, create_draft, record_team_fan_factor
from realtime_helpers import DIRECT_TIMEOUT_MS, count_navigations, goto_declared, match_url

OPPONENT = ".match-report-pre-match .opponent-fan-factor"


@pytest.fixture(scope="module")
def competition(browser, space_id):
    return build_full_competition(browser, space_id, num_teams=2, num_rounds=2)


def _match_b(space_id, ctx, round_index):
    """Un match démarré en « Chacun son équipe » sur la journée demandée."""
    domicile, exterieur = ctx["team_ids"][0], ctx["team_ids"][1]
    return create_draft(
        space_id, ctx, ctx["round_ids"][round_index], domicile, exterieur, entry_mode="TeamEntry"
    )


def _team_rolls(mr_id):
    """Les jets de camp appendés, `(camp, jet)`, dans l'ordre."""
    rows = query_db(
        "SELECT (payload->>'team_side') || '|' || (payload->>'fan_roll') "
        "FROM match_report_event_store "
        f"WHERE match_report_id = '{mr_id}' AND event_type = 'TeamFanFactorRecorded' "
        "ORDER BY version"
    )
    return [tuple(row.split("|")) for row in rows]


def _appended(mr_id, event_type):
    return int(query_db(
        "SELECT count(*) FROM match_report_event_store "
        f"WHERE match_report_id = '{mr_id}' AND event_type = '{event_type}'"
    )[0])


def _submit_roll(page, roll):
    page.fill("input[name='fan_roll']", str(roll))
    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")


def test_le_jet_du_domicile_apparait_chez_l_exterieur_sans_rechargement(browser, space_id, competition):
    mr_id = _match_b(space_id, competition, 0)
    exterieur = browser.new_context(extra_http_headers=PROFIL_SIMPLE).new_page()
    domicile = browser.new_context().new_page()

    goto_declared(exterieur, match_url(space_id, mr_id, "step2"))
    assert exterieur.url.endswith(f"/{mr_id}/step2/away"), exterieur.url
    expect(exterieur.locator(OPPONENT)).to_contain_text("en attente")
    navigations = count_navigations(exterieur)

    domicile.goto(match_url(space_id, mr_id, "step2"), wait_until="load")
    assert domicile.url.endswith(f"/{mr_id}/step2/home"), domicile.url
    _submit_roll(domicile, 2)
    assert "/step2" not in domicile.url, domicile.url

    attendu = exterieur.locator(OPPONENT)
    expect(attendu).to_contain_text("+ 2 =", timeout=DIRECT_TIMEOUT_MS)
    expect(attendu).to_contain_text("lancé par DevCoach")
    assert navigations == [], f"la page de l'extérieur s'est rechargée : {navigations}"

    _submit_roll(exterieur, 3)
    assert "/step2" not in exterieur.url, exterieur.url
    assert _team_rolls(mr_id) == [("Home", "2"), ("Away", "3")]
    assert _appended(mr_id, "TeamValuesRecorded") == 1, "les valeurs se figent au premier jet"


def test_l_exterieur_ne_touche_pas_au_jet_du_domicile(space_id, competition):
    mr_id = _match_b(space_id, competition, 1)
    lance = record_team_fan_factor(space_id, mr_id, "home", 2)
    assert lance.status_code == 303, f"obtenu {lance.status_code}"

    chez_l_autre = record_team_fan_factor(space_id, mr_id, "home", 1, PROFIL_SIMPLE)
    assert chez_l_autre.status_code == 403, f"obtenu {chez_l_autre.status_code}"

    les_deux = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "1", "away_fan_roll": "1"},
        headers=PROFIL_SIMPLE,
        allow_redirects=False,
        timeout=30,
    )
    assert les_deux.status_code == 422, f"obtenu {les_deux.status_code}"

    second = record_team_fan_factor(space_id, mr_id, "home", 3)
    assert second.status_code == 422, f"obtenu {second.status_code}"

    assert _team_rolls(mr_id) == [("Home", "2")]
    assert _appended(mr_id, "FanFactorRecorded") == 0
