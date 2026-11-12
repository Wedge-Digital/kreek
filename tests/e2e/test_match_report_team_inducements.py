"""Le favori a fini ses achats (carte 655, épic E22).

En « Chacun son équipe », le favori achète d'abord ; l'outsider attend sur sa
propre page, qui s'ouvre d'elle-même sur ses achats quand le favori déclare
« J'ai fini mes achats » — panier vide compris. Chacun n'achète que pour son
équipe : `/inducements/{team_id}` et ses mercenaires sont sous `garde_camp`.

Scénarios :
- B1 — Deux navigateurs : l'outsider voit l'attente ; le favori, sans
       « Passer », déclare « J'ai fini mes achats » sans rien acheter et attend
       à son tour ; la page de l'outsider s'ouvre sur ses achats sans
       rechargement, et il reçoit le toast. Quand l'outsider a fini, la page
       du favori passe au récapitulatif.
- B2 — L'outsider qui poste avant le favori reçoit 409 ; le membre simple qui
       poste, ou ouvre les mercenaires, sur l'équipe de DevCoach reçoit 403.

Le mode B se démarre par un POST direct (`entry_mode=TeamEntry`) : l'option
reste grisée jusqu'à la carte 659. DevCoach, admin de l'espace, coache la
première équipe que monte `build_full_competition` (le domicile) ; le membre
simple (`X-Bypass-Auth-Profile: simple`) la deuxième. **Le favori ne se
présume pas** : il se lit dans les valeurs d'équipe figées au premier jet.

**Sans rechargement** : un témoin posé sur `window` survit au remplacement de
`#app-content`, pas à un chargement de page.

Prérequis : `make e2e-server` (port 3211, base kreek_e2e), `make init_e2e_db`.
"""

from dataclasses import dataclass

import pytest
import requests
from playwright.sync_api import expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import PROFIL_SIMPLE, create_draft, record_team_fan_factor
from realtime_helpers import (
    DIRECT_TIMEOUT_MS,
    goto_declared,
    presence_declared,
    recording_context,
    toasts_of_match,
    wait_for_match_toasts,
)

WAIT = ".match-report-inducements .inducements-wait"
FINISHED = "J'ai fini mes achats"
WITNESS = "() => { window.__sansRechargement = true; }"
STILL_THERE = "() => window.__sansRechargement === true"


@dataclass
class Side:
    team_id: str
    headers: dict


@pytest.fixture(scope="module")
def competition(browser, space_id):
    return build_full_competition(browser, space_id, num_teams=2, num_rounds=3)


def _match_b(space_id, ctx, round_index):
    """Un match en « Chacun son équipe », les deux fans factors lancés."""
    domicile, exterieur = ctx["team_ids"][0], ctx["team_ids"][1]
    mr_id = create_draft(
        space_id, ctx, ctx["round_ids"][round_index], domicile, exterieur, entry_mode="TeamEntry"
    )
    for side, roll, headers in (("home", 2, None), ("away", 3, PROFIL_SIMPLE)):
        resp = record_team_fan_factor(space_id, mr_id, side, roll, headers)
        assert resp.status_code == 303, f"fan factor {side}: {resp.status_code}"
    return mr_id


def _topdog_and_underdog(mr_id, ctx):
    """Le favori puis l'outsider : la plus forte valeur figée, le domicile à
    égalité (R10)."""
    row = query_db(
        "SELECT (payload->>'home_team_value') || '|' || (payload->>'away_team_value') "
        "FROM match_report_event_store "
        f"WHERE match_report_id = '{mr_id}' AND event_type = 'TeamValuesRecorded'"
    )[0]
    home_tv, away_tv = (int(v) for v in row.split("|"))
    home = Side(ctx["team_ids"][0], {})
    away = Side(ctx["team_ids"][1], PROFIL_SIMPLE)
    return (away, home) if away_tv > home_tv else (home, away)


def _inducements_url(space_id, mr_id, team_id):
    return f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/inducements/{team_id}"


def _post_finished(space_id, mr_id, side):
    return requests.post(
        _inducements_url(space_id, mr_id, side.team_id),
        data={"intent": "validate", "selection": "[]", "mercenaries": "[]"},
        headers=side.headers,
        allow_redirects=False,
        timeout=30,
    )


def _appended(mr_id, event_type):
    return int(query_db(
        "SELECT count(*) FROM match_report_event_store "
        f"WHERE match_report_id = '{mr_id}' AND event_type = '{event_type}'"
    )[0])


def _finish_in_browser(page):
    """« J'ai fini mes achats », panier vide ; la page suivante — l'attente —
    déclare sa présence avant qu'on lui envoie un signal."""
    with page.expect_response(presence_declared):
        _click_finished(page)


def _click_finished(page):
    """La seconde validation mène au récapitulatif, qui ne déclare pas sa
    présence (décision 9 de la carte 655) : on n'attend que la navigation."""
    with page.expect_navigation(wait_until="load"):
        page.get_by_role("button", name=FINISHED).click()


def test_l_outsider_attend_le_favori_puis_achete_sans_rechargement(browser, space_id, competition):
    mr_id = _match_b(space_id, competition, 0)
    topdog, underdog = _topdog_and_underdog(mr_id, competition)
    outsider = recording_context(browser, underdog.headers).new_page()
    favori = browser.new_context(extra_http_headers=topdog.headers).new_page()

    goto_declared(outsider, _inducements_url(space_id, mr_id, underdog.team_id))
    expect(outsider.locator(WAIT)).to_contain_text("achète ses coups de pouce")
    expect(outsider.get_by_role("button", name=FINISHED)).to_have_count(0)
    outsider.evaluate(WITNESS)

    favori.goto(_inducements_url(space_id, mr_id, topdog.team_id), wait_until="load")
    expect(favori.locator("button[name='intent'][value='pass']")).to_have_count(0)
    _finish_in_browser(favori)
    expect(favori.locator(WAIT)).to_contain_text("Vos coups de pouce sont validés")

    expect(outsider.get_by_role("button", name=FINISHED)).to_be_visible(timeout=DIRECT_TIMEOUT_MS)
    assert outsider.evaluate(STILL_THERE), "la page de l'outsider s'est rechargée"
    wait_for_match_toasts(outsider, mr_id, 1)
    assert "aucun coup de pouce" in toasts_of_match(outsider, mr_id)[-1]["text"]

    _click_finished(outsider)
    expect(favori.locator(".match-report-inducements .mr-header-sub")).to_contain_text(
        "Achats terminés pour les deux équipes", timeout=DIRECT_TIMEOUT_MS
    )
    assert _appended(mr_id, "InducementsRecorded") == 2


def test_chacun_n_achete_que_pour_son_equipe(space_id, competition):
    mr_id = _match_b(space_id, competition, 1)
    topdog, underdog = _topdog_and_underdog(mr_id, competition)

    avant = _post_finished(space_id, mr_id, underdog)
    assert avant.status_code == 409, f"obtenu {avant.status_code}"

    devcoach_team = competition["team_ids"][0]
    chez_l_autre = _post_finished(space_id, mr_id, Side(devcoach_team, PROFIL_SIMPLE))
    assert chez_l_autre.status_code == 403, f"obtenu {chez_l_autre.status_code}"

    mercenaires = requests.get(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2/{devcoach_team}/mercenaires",
        headers={**PROFIL_SIMPLE, "HX-Request": "true"},
        allow_redirects=False,
        timeout=30,
    )
    assert mercenaires.status_code == 403, f"obtenu {mercenaires.status_code}"

    assert _appended(mr_id, "InducementsRecorded") == 0
    assert topdog.team_id != underdog.team_id


def _cancel_as(space_id, mr_id, headers):
    """Le bouton d'annulation est un `hx-post` : on envoie ce qu'htmx enverrait."""
    return requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/cancel-inducements",
        headers={**(headers or {}), "HX-Request": "true"},
        allow_redirects=False,
        timeout=30,
    )


def _page_as(space_id, mr_id, team_id, headers):
    return requests.get(_inducements_url(space_id, mr_id, team_id), headers=headers or {}, timeout=30)


def _check_admin_switch_on_waiting_page(space_id, mr_id, waiting_team, exterieur):
    """La page de l'équipe qui attend : DevCoach, admin d'espace, y a le lien
    vers l'autre camp ; le membre simple, sur sa propre attente, ne l'a pas."""
    admin_page = _page_as(space_id, mr_id, waiting_team, None)
    assert "inducements-wait" in admin_page.text, "la page de l'équipe qui attend"
    assert "Saisir pour" in admin_page.text, "DevCoach est admin : il peut passer à l'autre camp"
    if waiting_team == exterieur:
        membre_page = _page_as(space_id, mr_id, exterieur, PROFIL_SIMPLE)
        assert "inducements-wait" in membre_page.text
        assert "Saisir pour" not in membre_page.text


def test_l_annulation_ramene_chacun_chez_lui_et_l_admin_peut_changer_de_camp(space_id, competition):
    """Décisions 2 et 6 de la carte 655. Une seule équipe attend à la fois :
    l'outsider avant que le favori ait fini, le favori après. Une annulation
    ramène chacun sur la page de son équipe, pas chez le favori."""
    mr_id = _match_b(space_id, competition, 2)
    topdog, underdog = _topdog_and_underdog(mr_id, competition)
    domicile, exterieur = competition["team_ids"][0], competition["team_ids"][1]

    _check_admin_switch_on_waiting_page(space_id, mr_id, underdog.team_id, exterieur)
    assert _post_finished(space_id, mr_id, topdog).status_code == 303
    _check_admin_switch_on_waiting_page(space_id, mr_id, topdog.team_id, exterieur)

    par_le_membre = _cancel_as(space_id, mr_id, PROFIL_SIMPLE)
    assert par_le_membre.status_code == 200, f"obtenu {par_le_membre.status_code}"
    assert par_le_membre.headers["HX-Redirect"].endswith(f"/inducements/{exterieur}")

    assert _post_finished(space_id, mr_id, topdog).status_code == 303
    par_devcoach = _cancel_as(space_id, mr_id, None)
    assert par_devcoach.status_code == 200, f"obtenu {par_devcoach.status_code}"
    assert par_devcoach.headers["HX-Redirect"].endswith(f"/inducements/{domicile}")
