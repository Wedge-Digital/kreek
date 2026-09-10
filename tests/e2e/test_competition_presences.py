"""Le parcours complet de l'onglet Présences, dans un navigateur.

Aucun test unitaire ne voit ce que ce fichier couvre. Les deux défauts qui ont
motivé l'existence de cette suite — le widget coach-search et les pickers de
tiers — n'ont été trouvés que là.

# Les cinq journées, et pourquoi elles sont allouées

Les scénarios ne sont pas indépendants : poser une réponse suppose une campagne,
tirer suppose un sondage clos, réparer suppose un tirage. Plutôt que de relancer
huit campagnes, chaque journée porte les scénarios qui partagent son état :

| Journée | Scénarios |
|---|---|
| J1 | lancer et voir les trois colonnes (R3) · poser une présence et son badge (R6) |
| J2 | clore, puis poser encore une réponse (R21) |
| J3 | tirer avec un seul présent (R15) |
| J4 | la chaîne : tirer → valider → Calendrier → défection (R12) → réparer |
| J5 | valider, vider au Calendrier, revenir (R24) |

**Les scénarios d'une même journée sont ordonnés**, et chacun le dit dans sa
docstring. C'est la dépendance d'ordre qui a fait diagnostiquer à faux un échec en
tentant d'isoler un test de `test_team_detail_state_banner.py` : lancé seul, il
échouait pour une raison sans rapport.

# Le piège à ne pas rouvrir

`cliquer_quand_cable` sur **tout** ce qui vient d'être injecté. Le panneau est
remplacé à chaque action, et c'est exactement la fenêtre où un bouton est peint,
visible et **inerte** : six éléments non câblés à `t=0`, zéro à `t=50 ms`.

**Aucun `sleep`.** Une durée fixe n'a pas de marge sur une machine chargée — et
c'est là que la suite échoue — tout en coûtant son délai aux appels où tout est
déjà prêt.

Prérequis : serveur lancé en `make dev-e2e`, base au gabarit (`make e2e_db`).
"""

import re

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import attendre_que, query_db
from htmx_helpers import cliquer_quand_cable, cliquer_quand_cable_locator

ENTETES = {"HX-Request": "true"}
LOINTAIN = "2099-12-31"


@pytest.fixture(scope="module")
def competition(browser, space_id):
    """Quatre équipes, cinq journées, **sans appariements**.

    `with_pairings` reste à son défaut : la carte 508 a montré qu'un test qui n'en
    a pas besoin ne doit pas en créer, le tirage départageant au sort (R17).
    """
    return build_full_competition(browser, space_id, 4, 5)


# ── Les adresses des routes ───────────────────────────────────────────────────


def _base(space_id, comp):
    return (
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{comp['competition_id']}/{comp['season_id']}/admin"
    )


def _onglet(space_id, comp):
    return f"{_base(space_id, comp)}/presences"


def _action(space_id, comp, nom):
    return f"{_base(space_id, comp)}/presences/{nom}"


def _poster(space_id, comp, nom, corps):
    """Les actions sont appelées par HTTP quand le test éprouve leur **effet**, et
    par un clic quand il éprouve **l'écran**. Les deux comptent, et les confondre
    ferait un fichier deux fois plus long sans rien prouver de plus."""
    return requests.post(
        _action(space_id, comp, nom), json=corps, headers=ENTETES, timeout=30
    )


def _ouvrir_la_journee(page: Page, space_id, comp, round_id):
    """Ouvre l'onglet sur une journée donnée, comme le ferait un clic dans la barre
    latérale — en posant `activeRoundId`, que la page hôte lit."""
    page.goto(f"{_onglet(space_id, comp)}", wait_until="load")
    expect(page.locator("#presences-sidebar")).to_be_visible(timeout=15000)
    page.evaluate(f"document.body.dataset.activeRoundId = '{round_id}'")
    page.evaluate(
        "htmx.trigger(document.body, 'roundSelected', "
        f"{{round_id: '{round_id}'}})"
    )
    expect(page.locator(".presences-panel")).to_be_visible(timeout=15000)


def _presences_en_base(comp, round_id):
    lignes = query_db(
        "SELECT a.presence FROM competition_presence_answers a "
        "JOIN competition_presence_surveys s ON s.id = a.survey_id "
        f"WHERE s.round_id = '{round_id}'"
    )
    return [l.strip() for l in lignes if l.strip()]


# ══ J1 — lancer, et poser une présence ════════════════════════════════════════


def test_j1_lancer_une_campagne_met_toutes_les_equipes_en_sans_reponse(
    page: Page, space_id, competition
):
    """R3 — une équipe dont le coach n'a pas d'adresse **entre dans la campagne**.

    Refuser le lancement ferait dépendre une campagne de quatorze coachs de la
    fiche incomplète d'un seul.
    """
    round_id = competition["round_ids"][0]
    reponse = _poster(
        space_id,
        competition,
        "launch",
        {"round_id": round_id, "deadline": LOINTAIN, "auto_remind": True},
    )
    assert reponse.status_code == 200, reponse.text[:400]

    _ouvrir_la_journee(page, space_id, competition, round_id)

    expect(page.locator(".panel-title")).to_contain_text("Sondage en cours")
    expect(page.locator(".colonne--muets .team-card")).to_have_count(4)
    expect(page.locator(".colonne--presents .team-card")).to_have_count(0)
    expect(page.locator(".legend-item--total")).to_contain_text("sur 4 équipes")


def test_j1_poser_une_presence_a_la_main_laisse_son_badge(
    page: Page, space_id, competition
):
    """R6 — « saisi par vous » **survit au rechargement**.

    C'est le seul test qui prouve que `Repondant::Organisateur` a traversé la base :
    un badge calculé à l'affichage passerait sans que rien ne le vérifie.

    Dépend du scénario précédent, qui a lancé la campagne de J1.
    """
    round_id = competition["round_ids"][0]
    _ouvrir_la_journee(page, space_id, competition, round_id)

    cliquer_quand_cable(page, ".colonne--muets .team-card .team-act--present")

    # Le panneau se recharge sur `presenceChanged` : on attend que la colonne bouge.
    expect(page.locator(".colonne--presents .team-card")).to_have_count(1, timeout=15000)
    expect(page.locator(".colonne--presents .team-badge-admin")).to_be_visible()

    # Rechargement complet : le badge vient de la base, pas de l'état de la page.
    _ouvrir_la_journee(page, space_id, competition, round_id)
    expect(page.locator(".colonne--presents .team-badge-admin")).to_be_visible(
        timeout=15000
    )
    assert "presente" in _presences_en_base(competition, round_id)


# ══ J2 — clore n'arrête pas l'organisateur ════════════════════════════════════


def test_j2_clore_puis_poser_encore_une_reponse(page: Page, space_id, competition):
    """R21 — la clôture ferme les chemins du coach, **pas celui de l'organisateur**.

    C'est lui qui rattrape le coup de fil reçu après l'échéance. Le refus opposé au
    coach appartient à l'unité 2, qui n'existe pas encore.
    """
    round_id = competition["round_ids"][1]
    assert (
        _poster(
            space_id,
            competition,
            "launch",
            {"round_id": round_id, "deadline": LOINTAIN, "auto_remind": False},
        ).status_code
        == 200
    )
    assert (
        _poster(space_id, competition, "close", {"round_id": round_id}).status_code == 200
    )

    _ouvrir_la_journee(page, space_id, competition, round_id)
    expect(page.locator(".panel-title")).to_contain_text("Sondage clos")

    cliquer_quand_cable(page, ".colonne--muets .team-card .team-act--present")

    expect(page.locator(".colonne--presents .team-card")).to_have_count(1, timeout=15000)
    assert "presente" in _presences_en_base(competition, round_id)


# ══ J3 — R15 refuse, et le dit ════════════════════════════════════════════════


def test_j3_un_seul_present_desactive_le_bouton_et_affiche_son_motif(
    page: Page, space_id, competition
):
    """R15 — le motif compte autant que le bouton inactif.

    Un bouton mort sans explication passerait pour une panne : c'est précisément ce
    que cette règle existe pour éviter, et vérifier le seul `disabled` laisserait
    passer sa disparition.
    """
    round_id = competition["round_ids"][2]
    assert (
        _poster(
            space_id,
            competition,
            "launch",
            {"round_id": round_id, "deadline": LOINTAIN, "auto_remind": False},
        ).status_code
        == 200
    )
    # Une seule présente, les trois autres absentes.
    for rang, equipe in enumerate(competition["team_ids"]):
        _poster(
            space_id,
            competition,
            "answer",
            {
                "round_id": round_id,
                "team_id": equipe,
                "presence": "presente" if rang == 0 else "absente",
            },
        )
    assert _poster(space_id, competition, "close", {"round_id": round_id}).status_code == 200

    _ouvrir_la_journee(page, space_id, competition, round_id)

    bouton = page.locator(".panel-footer .btn-primary-sm")
    expect(bouton).to_contain_text("Apparier les présents")
    expect(bouton).to_be_disabled()
    expect(page.locator(".panel-blocage")).to_contain_text("au moins deux")


# ══ J4 — la chaîne : tirer, valider, défection, réparer ═══════════════════════


def _tous_presents(space_id, competition, round_id):
    for equipe in competition["team_ids"]:
        _poster(
            space_id,
            competition,
            "answer",
            {"round_id": round_id, "team_id": equipe, "presence": "presente"},
        )


def _appariements_en_base(comp, round_id):
    return [
        l.strip()
        for l in query_db(
            f"SELECT id FROM competition_match_day_pairings WHERE match_day_id = '{round_id}'"
        )
        if l.strip()
    ]


def test_j4_tirer_puis_valider_ecrit_les_rencontres_au_calendrier(
    page: Page, space_id, competition
):
    """Le seul test qui prouve que `confirm_draw` fait ce qu'il dit.

    Quatre présents donnent deux rencontres, et elles doivent se retrouver dans les
    tables du Calendrier — donc leur projection avec.
    """
    round_id = competition["round_ids"][3]
    assert (
        _poster(
            space_id,
            competition,
            "launch",
            {"round_id": round_id, "deadline": LOINTAIN, "auto_remind": False},
        ).status_code
        == 200
    )
    _tous_presents(space_id, competition, round_id)
    assert _poster(space_id, competition, "close", {"round_id": round_id}).status_code == 200

    _ouvrir_la_journee(page, space_id, competition, round_id)
    cliquer_quand_cable(page, ".panel-footer .btn-primary-sm")

    # L'aperçu : deux rencontres, et **des noms**, jamais un identifiant.
    expect(page.locator(".panel-title")).to_contain_text("Tirage proposé", timeout=15000)
    expect(page.locator(".draw-row")).to_have_count(2)
    # Le nom, pas l'identifiant — le défaut de la carte 506. La vérification porte
    # sur la **forme** d'un ULID nu : chercher « 01 » n'importe où échouait sur les
    # noms de la fixture, qui portent un horodatage.
    premier = page.locator(".draw-row .draw-home").first
    expect(premier).not_to_have_text(
        re.compile(r"^[0-9A-HJKMNP-TV-Z]{26}$"),
    )

    cliquer_quand_cable(page, ".panel-footer .btn-primary-sm")

    attendre_que(
        lambda: len(_appariements_en_base(competition, round_id)) == 2,
        quoi="les deux rencontres écrites au calendrier",
    )
    expect(page.locator(".panel-title")).to_contain_text("Journée appariée", timeout=15000)


def test_j4_une_defection_apres_le_tirage_propose_sans_agir(
    page: Page, space_id, competition
):
    """R12 — le panneau de défection **propose**, il n'agit pas.

    Et les autres rencontres ne bougent pas : leurs coachs ont déjà noté leur
    adversaire, et refaire le tirage entier ferait trois mécontents pour en soulager
    un.

    Dépend du scénario précédent, qui a apparié J4.
    """
    round_id = competition["round_ids"][3]
    avant = set(_appariements_en_base(competition, round_id))
    assert len(avant) == 2, "le scénario précédent doit avoir apparié la journée"

    # Une équipe se décommande — celle du premier appariement.
    desistante = query_db(
        f"SELECT home_team_id FROM competition_match_day_pairings WHERE match_day_id = '{round_id}' "
        "ORDER BY id LIMIT 1"
    )[0].strip()
    assert (
        _poster(
            space_id,
            competition,
            "answer",
            {"round_id": round_id, "team_id": desistante, "presence": "absente"},
        ).status_code
        == 200
    )

    _ouvrir_la_journee(page, space_id, competition, round_id)
    expect(page.locator(".panel-title")).to_contain_text("ne tient plus", timeout=15000)
    # Rien n'a été défait : le panneau signale, il ne répare pas.
    assert set(_appariements_en_base(competition, round_id)) == avant

    cliquer_quand_cable(page, ".panel-footer .btn-primary-sm")
    expect(page.locator(".panel-title")).to_contain_text(
        "Réparation proposée", timeout=15000
    )
    assert set(_appariements_en_base(competition, round_id)) == avant, (
        "proposer n'écrit rien"
    )


def test_j4_valider_la_reparation_ne_touche_que_la_rencontre_concernee(
    page: Page, space_id, competition
):
    """La rencontre touchée change, **les autres ne bougent pas**.

    Dépend des deux scénarios précédents.
    """
    round_id = competition["round_ids"][3]
    avant = set(_appariements_en_base(competition, round_id))

    _ouvrir_la_journee(page, space_id, competition, round_id)
    expect(page.locator(".panel-title")).to_contain_text("ne tient plus", timeout=15000)
    cliquer_quand_cable(page, ".panel-footer .btn-primary-sm")
    expect(page.locator(".panel-title")).to_contain_text(
        "Réparation proposée", timeout=15000
    )

    cliquer_quand_cable_locator(page, page.locator(".panel-footer .btn-primary-sm"))

    attendre_que(
        lambda: set(_appariements_en_base(competition, round_id)) != avant,
        quoi="la réparation écrite",
    )
    apres = set(_appariements_en_base(competition, round_id))
    intactes = avant & apres
    assert intactes, "au moins une rencontre devait rester intacte"


# ══ J5 — R24, la décision la plus structurante de l'épic ══════════════════════


def test_j5_vider_la_journee_au_calendrier_ramene_l_onglet_a_clos(
    page: Page, space_id, competition
):
    """R24 — l'appariement se lit **sur la journée**, jamais sur la campagne.

    C'est ce que la règle promet, et rien d'autre ne le vérifie : vider la journée
    depuis le Calendrier doit ramener l'onglet Présences à « clos », sans qu'aucune
    réconciliation n'ait à tourner. Avec la forme précédente — la campagne se
    souvenant de ses rencontres — l'onglet aurait continué à dire « appariée ».
    """
    round_id = competition["round_ids"][4]
    assert (
        _poster(
            space_id,
            competition,
            "launch",
            {"round_id": round_id, "deadline": LOINTAIN, "auto_remind": False},
        ).status_code
        == 200
    )
    _tous_presents(space_id, competition, round_id)
    assert _poster(space_id, competition, "close", {"round_id": round_id}).status_code == 200

    _ouvrir_la_journee(page, space_id, competition, round_id)
    cliquer_quand_cable(page, ".panel-footer .btn-primary-sm")
    expect(page.locator(".panel-title")).to_contain_text("Tirage proposé", timeout=15000)
    cliquer_quand_cable(page, ".panel-footer .btn-primary-sm")
    attendre_que(
        lambda: len(_appariements_en_base(competition, round_id)) == 2,
        quoi="la journée appariée",
    )

    # Le Calendrier vide la journée — un autre onglet, un autre chemin.
    vidage = requests.post(
        f"{_base(space_id, competition)}/schedule/clear-round",
        json={"round_id": round_id},
        headers=ENTETES,
        timeout=30,
    )
    assert vidage.status_code in (200, 204), vidage.text[:300]
    attendre_que(
        lambda: not _appariements_en_base(competition, round_id),
        quoi="la journée vidée",
    )

    _ouvrir_la_journee(page, space_id, competition, round_id)

    expect(page.locator(".panel-title")).to_contain_text("Sondage clos", timeout=15000)
    expect(page.locator(".panel-title")).not_to_contain_text("appariée")
