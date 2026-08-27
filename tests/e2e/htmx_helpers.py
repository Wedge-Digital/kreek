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
