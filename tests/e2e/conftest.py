import os
import urllib.error
import urllib.request

import pytest

BASE_URL = os.environ.get("E2E_BASE_URL", "http://localhost:3210")
# Space appartenant à l'utilisateur legacy_id=1 ("Bagouze_2"), celui que
# bypass_auth connecte automatiquement en dev (BYPASS_AUTH=true).
SPACE_ID = os.environ.get("E2E_SPACE_ID", "01KV65QPBK151AJTSAMB8BE6SS")
# La page phase 2 ("rules") n'exige pas que competition_id/season_id existent
# réellement en base — le contrôleur retombe sur des valeurs par défaut. Un
# ULID au bon format suffit donc à charger la page sans créer de compétition.
COMPETITION_ID = os.environ.get("E2E_COMPETITION_ID", SPACE_ID)
SEASON_ID = os.environ.get("E2E_SEASON_ID", SPACE_ID)


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
def competition_rules_url(space_id):
    return f"{BASE_URL}/app/{space_id}/competitions/create/{COMPETITION_ID}/{SEASON_ID}/rules"


# Chrome logge automatiquement un message de type "error" en console pour
# toute réponse HTTP non-2xx (fetch/XHR), y compris quand le code applicatif
# la gère correctement (ex: un test qui déclenche volontairement un 422).
# Ce n'est pas une erreur JS — on l'exclut pour ne garder que les vraies
# exceptions (TypeError, ReferenceError, throw non capturé, etc.).
_BENIGN_CONSOLE_PATTERNS = ("Failed to load resource",)


@pytest.fixture
def console_errors(page):
    errors = []

    def on_console(msg):
        if msg.type == "error" and not any(p in msg.text for p in _BENIGN_CONSOLE_PATTERNS):
            errors.append(msg.text)

    page.on("console", on_console)
    page.on("pageerror", lambda exc: errors.append(f"pageerror: {exc}"))
    return errors


@pytest.fixture(autouse=True)
def _fail_on_console_errors(console_errors):
    yield
    assert not console_errors, f"Erreurs JS détectées dans la console : {console_errors}"
