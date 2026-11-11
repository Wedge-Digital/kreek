"""Attendre qu'htmx ait **câblé** le contenu qu'il vient d'insérer.

# Le piège

htmx câble le contenu inséré quelques dizaines de millisecondes *après* l'avoir
rendu visible. Pendant cette fenêtre, un bouton est peint, visible, cliquable —
et **inerte**. Le clic s'y perd sans émettre la moindre requête, sans erreur de
console, sans rien.

Mesuré sur l'étape de validation du magicien de compétition, trois fois de
suite, identique :

    t=0ms    6 éléments htmx non câblés sur 31
    t=50ms   0
    t=500ms  0

Les six étaient tout le contenu fraîchement injecté — les quatre « Modifier »,
« ← Retour », et « 🏆 Créer la compétition ».

# Pourquoi aucune attente habituelle ne le voit

À t=0 la page est rendue : l'élément est visible, son texte est le bon, plus
aucune requête n'est en vol. **Tous les signaux qu'on attend naturellement sont
déjà verts.** C'est ce qui rend le défaut si trompeur — et le symptôme, un clic
qui ne produit strictement rien, ne ressemble pas à un problème d'attente.

Attendre `.htmx-request` à zéro ne suffit pas : la classe est retirée avant que
le contenu inséré soit câblé.

# Pourquoi pas un `sleep`

Une durée fixe n'a aucune marge : la fenêtre s'étire sur une machine chargée, et
c'est précisément là que la suite échouait. Elle coûterait en plus son délai à
chaque appel, y compris les milliers de fois où tout est prêt immédiatement.
La condition ci-dessous rend la main dès que c'est vrai.

# La limite, à connaître

`htmx-internal-data` est une propriété **interne** d'htmx, pas une API publique.
Si htmx la renomme, ces helpers expirent au bout de leur `timeout` — bruyamment,
jamais en silence. C'est le prix d'une condition précise ; l'alternative serait
d'écouter `htmx:afterSettle`, au prix d'un branchement avant chaque échange.
"""

from playwright.sync_api import Page

_EST_CABLE = "e => !!(e['htmx-internal-data'])"


def attendre_cablage_locator(page: Page, cible, timeout: int = 10000) -> None:
    """Même attente, pour une cible qu'un sélecteur seul ne désigne pas — une
    ligne de tableau retrouvée par filtrage, par exemple.

    L'élément est passé à `wait_for_function` **par sa poignée**. Une première
    version interrogeait `document.querySelector` avec le sélecteur : les moteurs
    propres à Playwright (`text=`, `has-text`) n'y sont pas des sélecteurs CSS,
    la condition n'était jamais vraie, et les tests échouaient à tous les coups
    au lieu d'une fois sur deux.
    """
    poignee = cible.first.element_handle(timeout=timeout)
    page.wait_for_function(_EST_CABLE, arg=poignee, timeout=timeout)


def attendre_cablage(page: Page, selecteur: str, timeout: int = 10000) -> None:
    """Rend la main quand htmx a câblé l'élément désigné."""
    attendre_cablage_locator(page, page.locator(selecteur), timeout)


def cliquer_quand_cable(page: Page, selecteur: str, timeout: int = 10000) -> None:
    """Clique une fois l'élément câblé — jamais avant."""
    attendre_cablage(page, selecteur, timeout)
    page.locator(selecteur).first.click()


def cliquer_quand_cable_locator(page: Page, cible, timeout: int = 10000) -> None:
    """Même clic, pour une cible qu'un sélecteur seul ne désigne pas.

    Une ligne de tableau retrouvée par `.filter(has=…)`, par exemple : les
    moteurs propres à Playwright ne sont pas des sélecteurs CSS, et
    `cliquer_quand_cable` ne peut pas les retrouver depuis le navigateur.

    L'attente avait déjà sa variante par poignée ; le clic ne l'avait pas, et
    l'appelant se rabattait sur un `.click()` nu — c'est-à-dire sur le piège
    même que ces helpers existent pour fermer.
    """
    attendre_cablage_locator(page, cible, timeout)
    cible.first.click()


# ── Attendre que des widgets soient au repos ──────────────────────────────────
#
# Cartes 662 et 667. Sur une page à widgets, une mutation remplace la page
# **plusieurs fois** : la réponse du POST remplace sa cible, puis l'événement
# qu'elle émet (`basketChanged` sur les phases de recrutement et de renvois)
# relance un GET de **chaque** widget. Attendre le premier signe visible — une
# ligne de plus au panier — rend la main entre deux remplacements : le bouton
# suivant est alors peint mais pas encore câblé, ou câblé puis aussitôt remplacé
# par un frère qui ne l'est pas. Le clic s'y perd sans requête. Une sonde l'a
# mesuré deux fois sur dix sur la phase de renvois.
#
# On attend donc que les rendus attendus soient **tous** arrivés, puis que plus
# rien ne soit en vol et que tout élément htmx soit câblé. Ce sont des
# conditions, pas des durées : elles rendent la main dès qu'elles sont vraies.

# Compte les rendus terminés de chaque conteneur. `htmx:afterSettle` est émis
# sur la cible de l'échange, c'est-à-dire sur le conteneur du widget.
_COMPTER_LES_RENDUS = """(classes) => {
    window.__rendus = Object.fromEntries(classes.map(c => [c, 0]));
    window.__rendus_classes = classes;
    if (window.__rendus_branches) return;
    window.__rendus_branches = true;
    document.body.addEventListener('htmx:afterSettle', e => {
        for (const c of window.__rendus_classes) {
            if (e.target.classList.contains(c)) window.__rendus[c]++;
        }
    });
}"""

_RENDUS_ARRIVES = (
    "(attendus) => Object.entries(attendus).every(([c, n]) => window.__rendus[c] >= n)"
)

_AU_REPOS = """(classes) => {
    const widgets = classes.map(c => document.querySelector('.' + c));
    if (widgets.some(w => !w)) return false;
    return widgets.every(w =>
        !w.matches('.htmx-request, .htmx-settling, .htmx-swapping')
        && !w.querySelector('.htmx-request, .htmx-settling, .htmx-swapping')
        && [...w.querySelectorAll('[hx-post], [hx-get]')].every(e => e['htmx-internal-data']));
}"""


def attendre_le_repos(page: Page, conteneurs: list[str], timeout: int = 10000) -> None:
    """Rend la main quand aucun des conteneurs (désignés par leur classe CSS)
    n'a de requête ni d'échange en cours, et que tout élément htmx qu'ils
    portent est câblé."""
    page.wait_for_function(_AU_REPOS, arg=conteneurs, timeout=timeout)


def muter(page: Page, cible, rendus: dict[str, int], timeout: int = 10000) -> None:
    """Clique une cible câblée, attend les rendus que la mutation déclenche —
    `{"dis-main": 2, "dis-side": 1}` : deux rendus de la liste, un du panier —,
    puis le repos de ces conteneurs."""
    conteneurs = list(rendus)
    attendre_le_repos(page, conteneurs, timeout)
    page.evaluate(_COMPTER_LES_RENDUS, conteneurs)
    cliquer_quand_cable_locator(page, cible, timeout)
    page.wait_for_function(_RENDUS_ARRIVES, arg=rendus, timeout=timeout)
    attendre_le_repos(page, conteneurs, timeout)
