"""La chaîne entière du sondage, sans qu'un organisateur saisisse une présence.

C'est la traduction littérale du `Terminé quand` de l'épic E16 :

    Un organisateur ouvre une campagne sur une journée, reçoit des réponses,
    tire au sort et retrouve ses rencontres au Calendrier — **sans avoir saisi
    une seule présence à la main**.

# Pourquoi ce fichier existe alors que trente-sept tests couvrent le sondage

Ils le couvrent en **deux moitiés qui ne se touchent pas** :

| Fichier | Prouve | S'arrête |
|---|---|---|
| `test_presence_reponse_publique.py` | le coach répond par son lien, l'organisateur le voit | avant le tirage |
| `test_competition_presences.py` | le tirage écrit les rencontres au Calendrier | ses présences viennent de l'endpoint `answer` de l'administration |

Celle qui va jusqu'au Calendrier saisit les présences à la main — précisément ce
que le critère exclut, et précisément ce que l'épic dit ne rien résoudre : « un
onglet où l'organisateur coche quatorze cases lui-même ne résout rien qu'il ne
sache déjà faire ».

# Aucune intervention humaine n'est nécessaire

Le jeton se lit en base, et **visiter son URL est le geste du coach** — il n'y a
pas de différence entre cliquer dans un e-mail et charger `/presence/{jeton}/oui`.
La chaîne est donc automatisable de bout en bout.

# Ce que ce test ne prouve pas

Qu'un e-mail soit **réellement parti et arrivé**. Il part du jeton, pas d'une
boîte mail. L'expédition relève de la carte 527 ; le rendu des trois gabarits
dans un vrai client mail demande un œil humain, et reste en attente au bas de la
carte 526.

Le dire ici plutôt que de le laisser deviner : un test nommé « chaîne complète »
qui tairait sa limite ferait croire l'e-mail couvert.

# Le garde-fou, et pourquoi il fait la valeur du test

Ne pas appeler `answer` ne suffit pas : il faut que **rien** ne l'appelle. Le
test vérifie donc en base qu'aucune réponse ne porte de `saisi_par_admin` — la
colonne que R6 réserve à la saisie de l'organisateur, et que le chemin du jeton
laisse à `NULL`.

Sans elle, un remaniement du fixture pourrait réintroduire une saisie
d'organisateur sans que rien ne bronche, et ce fichier recommencerait à prouver
la moitié que les autres prouvent déjà.

## Ce que le garde-fou attrape, et ce qu'il laisse passer

**R28 : le canal ne vit que le temps de l'écriture.** Une réponse saisie par
l'organisateur puis re-posée par le coach voit son `saisi_par_admin` **remis à
`NULL`** — c'est voulu, le canal n'est pas une propriété de la réponse.

Conséquence, vérifiée en falsifiant deux fois :

| Ce qu'on injecte | Le garde-fou |
|---|---|
| une saisie d'organisateur **en plus** des visites de jetons | ne voit rien — la visite l'a effacée |
| le chemin du jeton **remplacé** par la saisie | tombe, en nommant les quatre équipes |

C'est la seconde forme qui est la régression réelle : on ne rajoute pas une
saisie par mégarde, on remplace une boucle jugée compliquée par un appel qui
« fait la même chose ». Le garde-fou est taillé pour celle-là.
"""

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import attendre_que, query_db
from htmx_helpers import cliquer_quand_cable

ENTETES = {"HX-Request": "true"}
LOINTAIN = "2099-12-31"


@pytest.fixture(scope="module")
def competition(browser, space_id):
    """Quatre équipes, une journée — deux rencontres attendues au bout.

    Quatre et non deux : deux équipes donneraient une seule rencontre, et un
    tirage à une paire ne peut pas se tromper d'appariement.
    """
    return build_full_competition(browser, space_id, 4, 1)


# ── Les adresses ──────────────────────────────────────────────────────────────


def _base(space_id, comp):
    return (
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{comp['competition_id']}/{comp['season_id']}/admin"
    )


def _poster(space_id, comp, nom, corps):
    return requests.post(
        f"{_base(space_id, comp)}/presences/{nom}",
        json=corps,
        headers=ENTETES,
        timeout=30,
    )


def _lien_du_coach(jeton):
    """L'URL que porte le bouton « Je serai là » de l'e-mail."""
    return f"{BASE_URL}/presence/{jeton}/oui"


def _ouvrir_la_journee(page: Page, space_id, comp, round_id):
    page.goto(f"{_base(space_id, comp)}/presences", wait_until="load")
    expect(page.locator("#presences-sidebar")).to_be_visible(timeout=15000)
    page.evaluate(f"document.body.dataset.activeRoundId = '{round_id}'")
    page.evaluate(
        "htmx.trigger(document.body, 'roundSelected', " f"{{round_id: '{round_id}'}})"
    )
    expect(page.locator(".presences-panel")).to_be_visible(timeout=15000)


# ── Ce que la base sait ───────────────────────────────────────────────────────


def _jetons(round_id):
    return query_db(
        "SELECT a.token FROM competition_presence_answers a "
        "JOIN competition_presence_surveys s ON s.id = a.survey_id "
        f"WHERE s.round_id = '{round_id}' ORDER BY a.team_id"
    )


def _saisies_par_admin(round_id):
    """Les réponses que l'organisateur a posées lui-même. Doit rester vide."""
    return [
        l.strip()
        for l in query_db(
            "SELECT a.team_id FROM competition_presence_answers a "
            "JOIN competition_presence_surveys s ON s.id = a.survey_id "
            f"WHERE s.round_id = '{round_id}' AND a.saisi_par_admin IS NOT NULL"
        )
        if l.strip()
    ]


def _appariements(round_id):
    return [
        l.strip()
        for l in query_db(
            "SELECT id FROM competition_match_day_pairings "
            f"WHERE match_day_id = '{round_id}'"
        )
        if l.strip()
    ]


# ══ Le parcours ════════════════════════════════════════════════════════════════


def test_du_lien_du_coach_jusqu_aux_rencontres_au_calendrier(
    page: Page, space_id, competition
):
    """Lancer, répondre par les liens, clore, tirer, valider — et les rencontres.

    Un seul test, et non cinq : c'est l'enchaînement qui est en cause, pas
    chacune de ses étapes. Elles sont déjà couvertes séparément par les quatre
    autres fichiers du sondage.
    """
    round_id = competition["round_ids"][0]

    # 1 — L'organisateur ouvre la campagne. C'est son seul geste de saisie.
    reponse = _poster(
        space_id,
        competition,
        "launch",
        {"round_id": round_id, "deadline": LOINTAIN, "auto_remind": False},
    )
    assert reponse.status_code == 200, reponse.text[:400]

    # 2 — Les quatre coachs répondent, chacun par son propre lien.
    jetons = _jetons(round_id)
    assert len(jetons) == 4, f"quatre campagnes attendues, {len(jetons)} trouvées"
    for jeton in jetons:
        page.goto(_lien_du_coach(jeton.strip()), wait_until="load")
        expect(page.locator(".presence-card--ok")).to_be_visible(timeout=15000)

    # Le garde-fou : personne n'a saisi à la place de personne.
    saisies = _saisies_par_admin(round_id)
    assert saisies == [], f"des présences ont été saisies par l'organisateur : {saisies}"

    # 3 — L'organisateur clôt, tire, et valide.
    assert (
        _poster(space_id, competition, "close", {"round_id": round_id}).status_code
        == 200
    )

    _ouvrir_la_journee(page, space_id, competition, round_id)
    cliquer_quand_cable(page, ".panel-footer .btn-primary-sm")
    expect(page.locator(".panel-title")).to_contain_text("Tirage proposé", timeout=15000)
    expect(page.locator(".draw-row")).to_have_count(2)

    cliquer_quand_cable(page, ".panel-footer .btn-primary-sm")

    # 4 — Les rencontres sont au Calendrier.
    attendre_que(
        lambda: len(_appariements(round_id)) == 2,
        quoi="les deux rencontres écrites au calendrier",
    )
    expect(page.locator(".panel-title")).to_contain_text(
        "Journée appariée", timeout=15000
    )

    # Et elles le sont toujours sans qu'une présence ait été posée à la main —
    # revérifié après le tirage, que `confirm_draw` écrit aussi dans cette table.
    assert _saisies_par_admin(round_id) == []
