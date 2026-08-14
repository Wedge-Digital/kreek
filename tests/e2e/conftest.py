import os
import uuid
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
    """Page de règles d'une compétition **qui existe**.

    Elle repliait auparavant `competition_id` et `season_id` sur le `space_id`,
    faute de mieux : l'URL désignait donc une compétition qui n'existait nulle
    part, et la page se rendait quand même. Les tests vérifiaient le rendu des
    pickers sur une compétition fantôme.

    Le contrôle d'appartenance (carte 324) a mis fin à cette tolérance — servir
    une page de configuration pour une compétition inexistante n'a pas de sens.
    La fixture crée donc un vrai brouillon.

    Écrit en base plutôt que joué au clic : le parcours du magicien est déjà
    couvert par `test_full_competition_creation_flow`, et le rejouer ici
    passerait par la page même qu'on veut tester.
    """
    from db_helpers import execute_db

    competition_id = f"01TEST{uuid.uuid4().hex[:20].upper()}"
    season_id = f"01TEST{uuid.uuid4().hex[:20].upper()}"
    execute_db(
        f"INSERT INTO competitions (id, space_id, name, logo) "
        f"VALUES ('{competition_id}', '{space_id}', 'Compétition de test pickers', '')"
    )
    execute_db(
        f"INSERT INTO competition_seasons (id, competition_id, name, status) "
        f"VALUES ('{season_id}', '{competition_id}', 'Saison 1', 'draft')"
    )
    yield f"{BASE_URL}/app/{space_id}/competitions/create/{competition_id}/{season_id}/rules"
    execute_db(f"DELETE FROM competition_seasons WHERE id = '{season_id}'")
    execute_db(f"DELETE FROM competitions WHERE id = '{competition_id}'")


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


# ── Garde-fou : aucune transaction fantôme ne survit à un test ────────────────
#
# Carte 317. Une requête annulée en vol laisse une connexion `idle in
# transaction` : `sqlx::Transaction::drop` ne peut pas `await` son `ROLLBACK`,
# et la connexion retourne au pool encore dans sa transaction. Ses verrous
# bloquent alors tout ce qui les demande — jusqu'à trois minutes observées.
#
# Le dégât est *décalé* : la fuite d'un test fait tomber un test **suivant**,
# souvent dans un autre fichier, en `Timeout 30000ms`. C'est ce décalage qui
# rendait la flakiness illisible et la faisait attribuer à la charge.
#
# Ce garde-fou supprime le décalage. Il échoue sur le test qui a fui, pas sur
# celui qui en paie le prix.

_FUITE_TOLEREE_S = 5


def _transactions_fantomes() -> list[str]:
    from db_helpers import query_db

    return query_db(
        "SELECT pid || ' — ouverte depuis ' || (now() - xact_start)::text "
        "    || ' — ' || left(regexp_replace(query, '\\s+', ' ', 'g'), 90) "
        "FROM pg_stat_activity "
        "WHERE datname = current_database() "
        "  AND state = 'idle in transaction' "
        f" AND now() - xact_start > interval '{_FUITE_TOLEREE_S} seconds'"
    )


@pytest.fixture(autouse=True)
def _fail_on_leaked_transactions():
    yield
    fuites = _transactions_fantomes()
    assert not fuites, (
        "Transaction laissée ouverte par ce test — elle bloquera les suivants "
        "(carte 317) :\n  " + "\n  ".join(fuites)
    )
