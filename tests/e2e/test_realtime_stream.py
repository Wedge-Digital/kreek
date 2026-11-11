"""Tests E2E : le flux temps réel d'un onglet (carte 642, épic E23).

Chaque page de l'application ouvre un flux SSE et reçoit `hello`, qui porte
l'identifiant de la connexion. Le flux est ouvert **une fois par chargement
complet** : une navigation par le menu (htmx, `hx-select="#app-content"`) ne
remplace que le contenu, et ne doit pas en ouvrir un second.

Ce que seul un navigateur montre : que le script du layout s'exécute, que
l'`EventSource` traduit `hello`, et que la navigation htmx ne rejoue pas le
script. Le harnais Rust vérifie les en-têtes ; il ne voit rien de cela.

**Pas de `networkidle`** : le flux reste ouvert, l'état n'arrive jamais. En
développement, `tower-livereload` tient en plus son propre `EventSource` : les
requêtes sont donc filtrées par chemin, jamais comptées en bloc.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import re

from playwright.sync_api import Page, expect

from e2e_env import BASE_URL
from realtime_helpers import HELLO_RECEIVED

STREAM_PATH = "/app/realtime/stream"
# Un ULID fait 26 caractères en Crockford base32.
ULID_LENGTH = 26


def _is_stream(url: str) -> bool:
    return url.split("?")[0].endswith(STREAM_PATH)


def _count_stream_requests(page: Page) -> list[str]:
    """Enregistre chaque ouverture du flux, avant toute navigation."""
    seen: list[str] = []
    page.on("request", lambda req: seen.append(req.url) if _is_stream(req.url) else None)
    return seen


def _wait_for_hello(page: Page) -> str:
    page.wait_for_function(HELLO_RECEIVED, timeout=15000)
    return page.evaluate("() => window.kreekRealtime.connectionId")


def test_a_page_opens_the_stream_and_receives_hello(page: Page, space_id):
    with page.expect_response(lambda r: _is_stream(r.url), timeout=15000) as info:
        page.goto(f"{BASE_URL}/app/{space_id}/competitions", wait_until="load")

    response = info.value
    assert response.status == 200
    assert response.headers.get("content-type", "").startswith("text/event-stream")
    assert response.headers.get("x-accel-buffering") == "no"

    connection_id = _wait_for_hello(page)
    assert len(connection_id) == ULID_LENGTH, f"identifiant inattendu : {connection_id!r}"


def test_menu_navigation_does_not_reopen_the_stream(page: Page, space_id):
    opened = _count_stream_requests(page)
    page.goto(f"{BASE_URL}/app/{space_id}/competitions", wait_until="load")
    connection_id = _wait_for_hello(page)

    entry = page.locator(".sub-menu-btn", has_text="Mes équipes")
    expect(entry).to_be_visible(timeout=15000)
    entry.click()
    expect(page).to_have_url(re.compile(r"/team/list$"), timeout=15000)

    assert len(opened) == 1, f"le flux a été ouvert {len(opened)} fois : {opened}"
    assert page.evaluate("() => window.kreekRealtime.connectionId") == connection_id
