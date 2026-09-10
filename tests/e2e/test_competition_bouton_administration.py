"""Le bouton `⚙️ Administration` s'offre à qui a le droit d'entrer (carte 544).

Deux défauts distincts le faisaient disparaître, et ce module en tient un test
chacun :

- **le droit à moitié lu** — `competition_detail` ne consultait que les
  administrateurs *de la compétition*. Un administrateur *d'espace* avait le
  droit d'entrer, et ne le voyait nulle part ;
- **la valeur inventée** — six des sept entrées de la page passaient `false` en
  dur à `full_page`. Arriver par un onglet effaçait le bouton pour **tout le
  monde**, y compris l'administrateur de la compétition.

Les deux se croisent dans `test_admin_espace_voit_le_bouton_par_l_onglet` :
corriger l'un sans l'autre le laisse rouge.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import pytest
import requests

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import execute_db

# `DevCoach`, l'utilisateur de `bypass_auth`, est le **seul** administrateur de
# l'espace E2E (`seed_e2e.rs`). Sans cet en-tête, c'est lui qui répond.
MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}

BOUTON = "Administration"


@pytest.fixture(scope="module")
def competition_sans_son_admin(browser, space_id):
    """Une compétition dont `DevCoach` **n'est plus** administrateur.

    Le magicien de création l'inscrit forcément comme administrateur de la
    compétition — `build_full_competition` le documente, `/recap` l'exige. Or
    c'est précisément la combinaison inverse qu'il faut ici : administrateur de
    l'**espace**, étranger à la **compétition**. Il faut donc le retirer après
    coup.

    Le retrait vise `competitions_members`, une projection : un rebuild de
    l'event store le réinstallerait. C'est sans conséquence — la compétition
    n'est utilisée que par ce module, et l'espace n'a pas d'autre
    administrateur à qui ce retrait pourrait nuire.
    """
    ctx = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    execute_db(
        "DELETE FROM competitions_members "
        f"WHERE competition_id = '{ctx['competition_id']}' "
        "AND competition_profile = 'CompetitionAdmin'"
    )
    return {"space_id": space_id, **ctx}


def _page(ctx: dict, onglet: str = "", entetes: dict | None = None) -> str:
    """Le HTML d'une entrée de la page compétition, en chargement complet.

    Sans en-tête `HX-Request` : c'est le chemin de la page entière, le seul où
    le bandeau — et donc le bouton — est rendu. Le fragment htmx d'un changement
    d'onglet ne le porte pas.
    """
    url = (
        f"{BASE_URL}/app/{ctx['space_id']}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}{onglet}"
    )
    reponse = requests.get(url, headers=entetes or {}, timeout=15)
    assert reponse.status_code == 200, f"{url} → {reponse.status_code}"
    return reponse.text


# ── 1 · Le droit ──────────────────────────────────────────────────────────────


def test_admin_espace_voit_le_bouton(competition_sans_son_admin):
    """Le cas de la carte : administrateur de l'espace, étranger à la
    compétition. La garde le laisse entrer — l'écran doit le lui dire."""
    assert BOUTON in _page(competition_sans_son_admin)


def test_membre_simple_ne_voit_pas_le_bouton(competition_sans_son_admin):
    """La contre-épreuve, sans laquelle le test ci-dessus passerait aussi bien
    si le bouton était offert à tout le monde."""
    assert BOUTON not in _page(competition_sans_son_admin, entetes=MEMBRE_SIMPLE)


def test_le_membre_simple_reste_refuse_a_l_entree(competition_sans_son_admin):
    """Masquer un bouton n'est pas un contrôle d'accès : `require_admin_access`
    garde l'entrée, et cette carte n'y a pas touché."""
    ctx = competition_sans_son_admin
    url = (
        f"{BASE_URL}/app/{ctx['space_id']}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/admin"
    )
    reponse = requests.get(url, headers=MEMBRE_SIMPLE, timeout=15)
    assert reponse.status_code == 403, f"obtenu {reponse.status_code}"


# ── 2 · Toutes les entrées de la page ─────────────────────────────────────────


@pytest.mark.parametrize(
    "onglet",
    ["", "/calendrier", "/resultats", "/detailed-standings", "/teams", "/stats"],
)
def test_admin_espace_voit_le_bouton_par_l_onglet(competition_sans_son_admin, onglet):
    """Chacune des sept entrées de la page rend le bouton.

    C'est le test qui compte : il croise les deux défauts. Six de ces entrées
    passaient `false` en dur, et la septième ne lisait que la moitié du droit —
    corriger l'un sans l'autre laisse ce test rouge.
    """
    assert BOUTON in _page(competition_sans_son_admin, onglet)
