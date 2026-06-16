import os
import urllib.error
import urllib.request

import pytest

BASE_URL = os.environ.get("E2E_BASE_URL", "http://localhost:3210")
# Space appartenant à l'utilisateur legacy_id=1 ("Bagouze_2"), celui que
# bypass_auth connecte automatiquement en dev (BYPASS_AUTH=true).
SPACE_ID = os.environ.get("E2E_SPACE_ID", "01KV65QPBK151AJTSAMB8BE6SS")


@pytest.fixture(scope="session", autouse=True)
def _server_is_running():
    try:
        urllib.request.urlopen(BASE_URL, timeout=2)
    except urllib.error.URLError as exc:
        pytest.exit(
            f"Serveur kreek inaccessible sur {BASE_URL} ({exc}).\n"
            "Lance-le d'abord en dev (BYPASS_AUTH=true), ex : `cargo run`.",
            returncode=2,
        )


@pytest.fixture
def space_id():
    return SPACE_ID


@pytest.fixture
def competition_create_url(space_id):
    return f"{BASE_URL}/app/{space_id}/competitions/create"


@pytest.fixture
def console_errors(page):
    errors = []
    page.on(
        "console", lambda msg: errors.append(msg.text) if msg.type == "error" else None
    )
    page.on("pageerror", lambda exc: errors.append(f"pageerror: {exc}"))
    return errors


@pytest.fixture(autouse=True)
def _fail_on_console_errors(console_errors):
    yield
    assert not console_errors, f"Erreurs JS détectées dans la console : {console_errors}"
