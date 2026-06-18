import os
import re
import urllib.error
import urllib.request

import pytest

BASE_URL = os.environ.get("E2E_BASE_URL", "http://localhost:3210")
_ULID_IN_PATH_RE = re.compile(r"/app/([0-9A-Z]{26})/")


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


@pytest.fixture(scope="session")
def space_id(_server_is_running):
    """Espace du coach connecté automatiquement par bypass_auth (legacy_id=1).

    Résolu dynamiquement via /app/spaces plutôt que codé en dur : un
    `make init_db` régénère des ULIDs aléatoires à chaque réimport, ce qui
    invaliderait silencieusement un identifiant figé. Surchargeable via
    E2E_SPACE_ID pour cibler un espace précis.
    """
    override = os.environ.get("E2E_SPACE_ID")
    if override:
        return override

    url = f"{BASE_URL}/app/spaces"
    try:
        with urllib.request.urlopen(url, timeout=5) as resp:
            html = resp.read().decode("utf-8")
    except urllib.error.URLError as exc:
        pytest.exit(f"Impossible de charger {url} ({exc}).", returncode=2)

    match = _ULID_IN_PATH_RE.search(html)
    if not match:
        pytest.exit(
            f"Aucun space_id trouvé dans {url} — l'utilisateur bypass_auth "
            "(legacy_id=1) n'appartient à aucun space. "
            "Lance `make init_db WITH_SEED=1` pour en créer.",
            returncode=2,
        )
    return match.group(1)


@pytest.fixture
def competition_create_url(space_id):
    return f"{BASE_URL}/app/{space_id}/competitions/create"


@pytest.fixture
def competition_rules_url(space_id):
    competition_id = os.environ.get("E2E_COMPETITION_ID", space_id)
    season_id = os.environ.get("E2E_SEASON_ID", space_id)
    return f"{BASE_URL}/app/{space_id}/competitions/create/{competition_id}/{season_id}/rules"


# Chrome logge automatiquement un message de type "error" en console pour
# toute réponse HTTP non-2xx (fetch/XHR), y compris quand le code applicatif
# la gère correctement (ex: un test qui déclenche volontairement un 422).
# Ce n'est pas une erreur JS — on l'exclut pour ne garder que les vraies
# exceptions (TypeError, ReferenceError, throw non capturé, etc.).
_BENIGN_CONSOLE_PATTERNS = ("Failed to load resource", "Response Status Error Code")


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
