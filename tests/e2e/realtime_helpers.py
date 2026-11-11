"""Aides partagées des e2e du temps réel (épic E23, carte 647).

Ramenées des fichiers des cartes 642 à 646, qui en portaient chacun une
copie : la connexion d'un coach seedé, l'attente du flux, le journal des
toasts à lien, l'ouverture de `/live` présence inscrite.

**Pas de `networkidle`** : le flux SSE reste ouvert, l'état n'arrive jamais.
"""

import requests
from playwright.sync_api import Browser, Page, expect

from e2e_env import BASE_URL

# `src/cli/seed_e2e.rs` — le mot de passe de tous les comptes semés.
SEED_PASSWORD = "changeme-dev-only"
HELLO_RECEIVED = "() => window.kreekRealtime && window.kreekRealtime.connectionId"
DESKTOP = {"width": 1280, "height": 900}
LOG_ENTRIES = ".match-report-live .mr-log-entry"
SCORES = ".match-report-live .live-score-value"
# Le direct doit battre l'ancienne interrogation : moins de 15 s.
DIRECT_TIMEOUT_MS = 14000

# Note chaque toast à lien inséré, dans un journal qui survit aux navigations.
# Le lien est noté avec le texte (carte 647) : il porte l'identifiant du match,
# seul repère qui distingue deux matchs dont les toasts se ressemblent.
RECORD_LINKED_TOASTS = """
new MutationObserver(function (mutations) {
  mutations.forEach(function (m) {
    m.addedNodes.forEach(function (node) {
      if (!node.classList || !node.classList.contains('toast--linked')) return;
      var seen = JSON.parse(sessionStorage.getItem('linkedToasts') || '[]');
      var link = node.querySelector('a.toast-link');
      seen.push({ text: node.textContent, href: link ? link.getAttribute('href') : null });
      sessionStorage.setItem('linkedToasts', JSON.stringify(seen));
    });
  });
}).observe(document, { childList: true, subtree: true });
"""


# ── Connexion d'un coach seedé (cartes 644 et 645) ────────────────────────────

def log_in(page: Page, coach_name: str) -> None:
    """Connexion réelle par `/auth/login` : `bypass_auth` ne s'applique qu'à une
    session vide, l'identité ouverte est donc conservée."""
    page.goto(f"{BASE_URL}/auth/login", wait_until="load")
    page.fill("input[name='coach_name']", coach_name)
    page.fill("input[name='password']", SEED_PASSWORD)
    page.click("button[type='submit']")
    page.wait_for_url("**/auth/login/success", timeout=10000)


def login(browser: Browser, coach_name: str) -> Page:
    page = browser.new_context().new_page()
    log_in(page, coach_name)
    return page


# ── Le flux et les toasts (cartes 642 et 645) ─────────────────────────────────

def recording_context(browser, headers=None):
    context = browser.new_context(viewport=DESKTOP, extra_http_headers=headers or {})
    context.add_init_script(RECORD_LINKED_TOASTS)
    return context


def wait_for_stream(page: Page) -> None:
    page.wait_for_function(HELLO_RECEIVED, timeout=15000)


def linked_toasts_seen(page: Page) -> list[dict]:
    return page.evaluate("() => JSON.parse(sessionStorage.getItem('linkedToasts') || '[]')")


def toast(page: Page, text: str):
    return page.locator(".toast-zone .toast--linked", has_text=text)


def toasts_of_match(page: Page, mr_id: str) -> list[dict]:
    """Les toasts notés dont le lien mène à ce match."""
    return [t for t in linked_toasts_seen(page) if mr_id in (t.get("href") or "")]


def wait_for_match_toasts(page: Page, mr_id: str, count: int, timeout_ms: int = 10000) -> None:
    """Attend que le journal de l'onglet compte `count` toasts liés à ce match :
    lu dans `sessionStorage`, il ne dépend ni de la vie du toast (6 s), ni des
    navigations."""
    page.wait_for_function(
        "([id, n]) => JSON.parse(sessionStorage.getItem('linkedToasts') || '[]')"
        ".filter(t => (t.href || '').includes(id)).length >= n",
        arg=[mr_id, count],
        timeout=timeout_ms,
    )


def assert_none_after(witness, pages: list[Page], mr_id: str) -> None:
    """Le constat d'absence **après** son témoin positif.

    `witness` attend qu'un destinataire légitime ait reçu le toast : le routeur
    livre tous les destinataires d'un fait dans la même boucle, un toast mal
    adressé serait donc arrivé aussi. Sans ce témoin, une livraison en panne
    ferait passer le constat pour une étanchéité.
    """
    witness()
    for page in pages:
        assert toasts_of_match(page, mr_id) == [], (page.url, linked_toasts_seen(page))


def http_session_of(context) -> requests.Session:
    """Une session `requests` qui porte les cookies du contexte : les requêtes
    partent sous l'identité ouverte dans ce navigateur, et non sous celle que
    `bypass_auth` donnerait à une session vide.

    L'en-tête `Cookie` est posé à la main : `http.cookiejar` ne renvoie pas un
    cookie de domaine `localhost` (il compare à `localhost.local`).
    """
    session = requests.Session()
    cookies = context.cookies(BASE_URL)
    session.headers["Cookie"] = "; ".join(f"{c['name']}={c['value']}" for c in cookies)
    return session


# ── La page d'un match, présence inscrite (cartes 644 et 646) ─────────────────

def match_url(space_id, mr_id, suffix):
    return f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{suffix}"


def presence_declared(response) -> bool:
    return response.url.endswith("/app/realtime/presence") and response.request.method == "POST"


def open_live(browser: Browser, space_id, mr_id, headers=None) -> Page:
    """`/live`, ses deux widgets chargés et sa présence inscrite."""
    page = browser.new_context(extra_http_headers=headers or {}).new_page()
    goto_live(page, space_id, mr_id)
    return page


def goto_live(page: Page, space_id, mr_id) -> None:
    """Mène un onglet existant sur `/live`, comme `open_live`."""
    with page.expect_response(presence_declared):
        with page.expect_response(lambda r: r.url.endswith(f"/{mr_id}/live/score")):
            with page.expect_response(lambda r: r.url.endswith(f"/{mr_id}/action-log")):
                page.goto(match_url(space_id, mr_id, "live"), wait_until="load")
    assert page.url.endswith(f"/{mr_id}/live"), page.url


def home_score(page: Page) -> int:
    first = page.locator(SCORES).first
    expect(first).to_be_visible(timeout=10000)
    return int(first.inner_text())


def count_navigations(page: Page) -> list:
    navigations: list = []
    page.on("framenavigated", lambda f: f.parent_frame is None and navigations.append(f.url))
    return navigations


def count_widget_requests(page: Page, mr_id: str) -> list:
    requests_seen: list = []
    page.on(
        "request",
        lambda r: (f"/{mr_id}/action-log" in r.url or f"/{mr_id}/live/score" in r.url)
        and requests_seen.append(r.url),
    )
    return requests_seen


def goto_declared(page: Page, url: str) -> None:
    """Une page de match, présence inscrite : un onglet ne reçoit le signal, et
    n'apparaît chez les autres, qu'après la réponse de son `POST` (carte 646)."""
    with page.expect_response(presence_declared):
        page.goto(url, wait_until="load")
