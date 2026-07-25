import os
import re
import urllib.error
import urllib.request

import pytest

BASE_URL = os.environ.get("E2E_BASE_URL", "http://localhost:3210")

# Roster du jeu de démonstration servant de sentinelle : sa présence atteste
# que le serveur charge bien `assets/references.example`.
DEMO_ROSTER_UID = "DEMO_GRANIT"

# Espace dédié à la suite : toutes les compétitions et équipes créées par les
# tests y atterrissent. Doit correspondre à SPACE_NAME dans
# `src/cli/seed_e2e.rs` — une divergence fait échouer la résolution ci-dessous
# avec un message explicite, jamais silencieusement.
E2E_SPACE_NAME = "Espace E2E"

# Dans /app/spaces, chaque espace est un bloc où l'identifiant précède son
# libellé. Le segment intermédiaire interdit un nouveau `hx-get=`, pour qu'un
# bloc sans title ne puisse pas déborder sur l'espace suivant.
_SPACE_ENTRY_RE = re.compile(
    r'hx-get="/app/([0-9A-Z]{26})/home"(?:(?!hx-get=)[\s\S])*?title="([^"]*)"'
)


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


@pytest.fixture(scope="session", autouse=True)
def _server_serves_demo_ruleset(_server_is_running):
    """Vérifie que le serveur sert bien `assets/references.example`.

    Toute la suite s'appuie sur les rosters de démonstration (Granitiers,
    Zéphyriens, Lanterniers). Un serveur servant un autre jeu de règles ne
    produit pas d'erreur explicite : les widgets se rendent vides, les
    sélections de roster n'ont aucun effet, et on récolte une cascade de
    timeouts illisibles après plusieurs minutes. Ce contrôle transforme ça en
    un message immédiat.
    """
    url = f"{BASE_URL}/references/roster-picker"
    try:
        with urllib.request.urlopen(url, timeout=5) as resp:
            final_url = resp.geturl()
            html = resp.read().decode("utf-8")
    except urllib.error.URLError as exc:
        pytest.exit(f"Impossible de charger {url} ({exc}).", returncode=2)

    # Sans authentification, la route redirige vers /auth/login : on récupère
    # la page de connexion, où aucun roster ne figure. Sans cette distinction,
    # un problème d'auth serait diagnostiqué à tort comme un mauvais jeu de
    # données — c'est arrivé.
    if "/auth/login" in final_url:
        pytest.exit(
            f"{url} redirige vers la page de connexion : le serveur n'a pas "
            "authentifié la requête.\n"
            "Vérifie BYPASS_AUTH=true, et que la base contient bien "
            "l'utilisateur legacy_id=1 (`cargo run -- seed-accounts`).",
            returncode=2,
        )

    if DEMO_ROSTER_UID not in html:
        pytest.exit(
            f"Le serveur ne sert pas le jeu de démonstration : le roster "
            f"{DEMO_ROSTER_UID} est absent de {url}.\n"
            "Relance le serveur avec `make dev-demo` (ou "
            "REFERENCES__DIR=assets/references.example).",
            returncode=2,
        )


@pytest.fixture(scope="session")
def space_id(_server_is_running):
    """Identifiant de l'espace « Espace E2E », créé par `make seed_e2e`.

    Résolu par son nom, et non en prenant le premier espace venu : sur une base
    contenant de vraies données, ce raccourci enverrait les compétitions et les
    équipes créées par les tests dans un espace de production. À défaut de
    trouver l'espace dédié, la suite s'arrête plutôt que d'écrire ailleurs.

    Résolu à chaque exécution plutôt que figé en constante : l'identifiant est
    régénéré à chaque `make reset_db`. Surchargeable via E2E_SPACE_ID.
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

    spaces = {name: uid for uid, name in _SPACE_ENTRY_RE.findall(html)}
    if E2E_SPACE_NAME not in spaces:
        pytest.exit(
            f"Espace « {E2E_SPACE_NAME} » introuvable dans {url}.\n"
            f"Espaces visibles par l'utilisateur bypass_auth : "
            f"{sorted(spaces) or 'aucun'}.\n"
            "Lance `make seed_e2e` pour le créer.",
            returncode=2,
        )
    return spaces[E2E_SPACE_NAME]


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
