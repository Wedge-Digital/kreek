"""Tests E2E — les parcours des coups de pouce (épic E20, carte 630).

Le « Terminé quand » de l'épic, éprouvé dans un navigateur :

1. le panier retire un article commun **sans que son onglet soit ouvert** ;
2. des achats validés ne se modifient plus — un second POST répond 409 ;
3. les deux équipes passées, la page montre le récapitulatif des deux camps ;
4. « Annuler les coups de pouce » puis « Garder » : rien ne bouge ;
5. « Oui, tout annuler » : retour chez le favori, achats effacés des deux
   côtés, journaliers retirés de l'effectif ;
6. après l'annulation, l'outsider rachète avec un budget recalculé ;
7. après la première action, l'annulation n'est plus proposée.

**Chaque test ouvre son propre rapport**, sur sa propre journée. Un état
partagé entre tests a cassé trois fois pendant E20 : une équipe qui a validé ne
rachète plus (R2), le favori achète d'abord (R9), et une équipe ne joue qu'un
match par journée (carte 551) — deux tests sur le même rapport, ou sur la même
journée, dépendraient de leur ordre d'exécution.

Les équipes 0 et 1 servent aux scénarios 1 à 4, 6 et 7, sur les journées 0 à 5.
Les équipes 2 et 3 sont réservées au scénario 5 (journées 6 et 7) : il leur
faut un blessé pour obtenir un journalier, et ce blessé ne doit gêner personne.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), base seedée.
"""

import json
import re

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import attendre_que, query_db
from htmx_helpers import cliquer_quand_cable
from match_report_helpers import (
    create_draft,
    ensure_inducements,
    ensure_pre_match,
    first_player_id,
    post_step5,
    publish,
    record_action_api,
)

# Corpus de démonstration (`assets/references.example/inducements_fr.json`) :
# deux coups de pouce sans tarif réduit ni restriction de roster, pour que leur
# prix ne dépende pas de l'équipe qui achète.
ARBITRE = "DEMO_ARBITRE_COMPLAISANT"  # 50
CHARIOT = "DEMO_CHARIOT_DE_VIVRES"  # 15, max 5
PRIX = {ARBITRE: 50, CHARIOT: 15}

_TEAM_ID_RE = re.compile(r"/inducements/([0-9A-Z]{26})")


# ── Fixture ──────────────────────────────────────────────────────────────────


@pytest.fixture(scope="module")
def flows_ctx(browser, space_id):
    """Une compétition dédiée : quatre équipes, huit journées — une journée
    par rapport, aucune partagée entre deux tests."""
    full = build_full_competition(browser, space_id, num_teams=4, num_rounds=8)
    assert len(full["round_ids"]) >= 8, "une journée par rapport : huit au moins"
    return {"space_id": space_id, "full": full, "teams": full["team_ids"]}


# ── Helpers ──────────────────────────────────────────────────────────────────


def _inducements_url(space_id: str, mr_id: str, team_id: str) -> str:
    return f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/inducements/{team_id}"


def _cancel_url(space_id: str, mr_id: str) -> str:
    return f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/cancel-inducements"


def _ouvrir_un_rapport(ctx, round_idx: int, home_idx: int = 0, away_idx: int = 1) -> dict:
    """Un rapport neuf, jusqu'aux coups de pouce. Le facteur fans renvoie chez
    le favori : c'est lui qui le désigne, sans recalculer la règle ici."""
    space_id, full = ctx["space_id"], ctx["full"]
    round_id = full["round_ids"][round_idx]
    home, away = ctx["teams"][home_idx], ctx["teams"][away_idx]
    mr_id = create_draft(space_id, full, round_id, home, away)
    ensure_pre_match(space_id, mr_id, full, round_id, home, away)
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "2", "away_fan_roll": "3"},
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"facteur fans : {resp.status_code}"
    m = _TEAM_ID_RE.search(resp.headers.get("Location", ""))
    assert m, f"le facteur fans doit mener aux coups de pouce : {resp.headers.get('Location')!r}"
    topdog = m.group(1)
    underdog = away if topdog == home else home
    return {"space_id": space_id, "mr_id": mr_id, "topdog": topdog, "underdog": underdog}


def _poster(rapport: dict, team_id: str, intent: str, achats=()) -> requests.Response:
    selection = json.dumps([{"uid": uid, "qty": qty} for uid, qty in achats])
    return requests.post(
        _inducements_url(rapport["space_id"], rapport["mr_id"], team_id),
        data={"intent": intent, "selection": selection, "mercenaries": "[]"},
        allow_redirects=False,
    )


def _acheter(rapport: dict, team_id: str, achats) -> None:
    resp = _poster(rapport, team_id, "validate", achats)
    assert resp.status_code in (302, 303), f"achat {achats} : {resp.status_code}\n{resp.text[:200]}"


def _passer(rapport: dict, team_id: str) -> None:
    resp = _poster(rapport, team_id, "pass")
    assert resp.status_code in (302, 303), f"« Passer » : {resp.status_code}\n{resp.text[:200]}"


def _cout(achats) -> int:
    return sum(PRIX[uid] * qty for uid, qty in achats)


def _annuler_par_http(rapport: dict) -> requests.Response:
    """Le bouton est un `hx-post` : on envoie ce qu'htmx enverrait."""
    return requests.post(
        _cancel_url(rapport["space_id"], rapport["mr_id"]),
        headers={"HX-Request": "true"},
        allow_redirects=False,
    )


def _evenements(mr_id: str, event_type: str) -> int:
    rows = query_db(
        "SELECT count(*) FROM match_report_event_store "
        f"WHERE match_report_id = '{mr_id}' AND event_type = '{event_type}'"
    )
    return int(rows[0])


def _achats_en_projection(mr_id: str, team_id: str) -> str:
    """Les achats de l'équipe dans `match_report_proj` — `NULL` en toutes
    lettres quand elle n'a rien validé (psql rendrait une ligne vide)."""
    rows = query_db(
        "SELECT CASE WHEN home_team_id = '" + team_id + "' "
        "THEN coalesce(home_inducements::text, 'NULL') "
        "ELSE coalesce(away_inducements::text, 'NULL') END "
        f"FROM match_report_proj WHERE match_report_id = '{mr_id}'"
    )
    assert rows, f"aucune projection pour {mr_id}"
    return rows[0]


def _budget_affiche(page: Page) -> int:
    return int(page.locator(".mr-budget-value").inner_text().strip())


def _ouvrir_le_recapitulatif(page: Page, rapport: dict, team_id: str) -> None:
    page.goto(_inducements_url(rapport["space_id"], rapport["mr_id"], team_id), wait_until="load")
    expect(page.locator(".mr-header-sub")).to_have_text("Achats terminés pour les deux équipes")
    expect(page.locator(".mr-recap-side")).to_have_count(2)


# ── 1 · Le panier retire un article commun sans ouvrir son onglet ────────────


def test_cart_removes_a_common_item_without_opening_its_tab(page: Page, flows_ctx):
    """Le panier est le seul endroit qui voie l'article : on quitte l'onglet
    « Communs » avant de le retirer. Le sélecteur, qui garde les quantités,
    doit pourtant revenir à 0 — on ne le vérifie qu'en rouvrant l'onglet."""
    rapport = _ouvrir_un_rapport(flows_ctx, round_idx=0)
    page.goto(_inducements_url(rapport["space_id"], rapport["mr_id"], rapport["topdog"]), wait_until="load")

    name = "Chariot de Vivres"
    card = page.locator(".mr-inducement-card", has=page.locator(".mr-inducement-name", has_text=name))
    expect(card).to_be_visible()
    card.locator(".mr-qty-btn").nth(1).click()

    expect(page.locator(".mr-cart-items")).to_contain_text("1 coup de pouce")
    expect(page.locator(".mr-cart-total-value")).to_have_text(str(PRIX[CHARIOT]))

    page.locator(".mr-tab", has_text="Spéciaux").click()
    expect(card).to_be_hidden()

    page.locator(".mr-cart-toggle").click()
    line = page.locator(".mr-cart-merco-item", has_text=name)
    expect(line).to_be_visible()
    line.locator(".mr-cart-merco-remove").click()

    expect(page.locator(".mr-cart-items")).to_contain_text("Aucun coup de pouce sélectionné")
    expect(page.locator(".mr-cart-total-value")).to_have_text("0")
    expect(page.locator(".mr-cart-toggle")).to_be_disabled()

    page.locator(".mr-tab", has_text="Communs").click()
    expect(card).to_be_visible()
    expect(card.locator(".mr-qty-value")).to_have_text("0")


# ── 2 · Des achats validés ne se modifient plus ──────────────────────────────


def test_validated_purchases_refuse_a_second_post(flows_ctx):
    """Le second POST porte un achat qui serait valable : seul l'état du
    rapport le refuse. L'événement reste unique, la projection inchangée."""
    rapport = _ouvrir_un_rapport(flows_ctx, round_idx=1)
    topdog, mr_id = rapport["topdog"], rapport["mr_id"]
    _acheter(rapport, topdog, [(CHARIOT, 1)])

    second = _poster(rapport, topdog, "validate", [(ARBITRE, 1)])

    assert second.status_code == 409, f"second POST : {second.status_code}\n{second.text[:200]}"
    assert "annulez" in second.text, f"le refus doit renvoyer à l'annulation : {second.text!r}"
    assert _evenements(mr_id, "InducementsRecorded") == 1
    achats = _achats_en_projection(mr_id, topdog)
    assert CHARIOT in achats and ARBITRE not in achats, f"projection modifiée : {achats}"


# ── 3 · Les deux équipes passées : le récapitulatif ──────────────────────────


def test_recap_shows_both_sides_and_their_lines(page: Page, flows_ctx):
    """Ouvert par l'adresse de l'outsider (R6 : quel que soit `team_id`), le
    récapitulatif montre le favori d'abord, et les achats de chacun."""
    rapport = _ouvrir_un_rapport(flows_ctx, round_idx=2)
    achats_favori = [(ARBITRE, 1), (CHARIOT, 1)]
    achats_outsider = [(CHARIOT, 2)]
    _acheter(rapport, rapport["topdog"], achats_favori)
    _acheter(rapport, rapport["underdog"], achats_outsider)

    _ouvrir_le_recapitulatif(page, rapport, rapport["underdog"])

    favori, outsider = page.locator(".mr-recap-side").nth(0), page.locator(".mr-recap-side").nth(1)
    expect(favori.locator(".mr-banner-label")).to_have_text("Favori · achète en premier")
    expect(outsider.locator(".mr-banner-label")).to_have_text("Outsider · achète en second")
    expect(favori.locator(".mr-cart-merco-item")).to_have_count(2)
    expect(favori.locator(".mr-cart-merco-item", has_text="Arbitre Complaisant")).to_contain_text("50 kPo")
    expect(favori.locator(".mr-cart-merco-item", has_text="Chariot de Vivres")).to_contain_text("15 kPo")
    expect(outsider.locator(".mr-cart-merco-item")).to_have_count(1)
    expect(outsider.locator(".mr-cart-merco-item")).to_contain_text("Chariot de Vivres ×2")
    expect(favori.locator(".mr-recap-total b")).to_have_text(str(_cout(achats_favori)))
    expect(outsider.locator(".mr-recap-total b")).to_have_text(str(_cout(achats_outsider)))
    expect(page.get_by_role("link", name="Coup d'envoi → Match")).to_have_attribute("href", re.compile(r"/step3$"))
    expect(page.locator(".mr-recap-cancel")).to_be_visible()


# ── 4 · « Annuler » puis « Garder » : rien ne change ─────────────────────────


def test_keep_after_cancel_prompt_changes_nothing(page: Page, flows_ctx):
    rapport = _ouvrir_un_rapport(flows_ctx, round_idx=3)
    _acheter(rapport, rapport["topdog"], [(CHARIOT, 1)])
    _passer(rapport, rapport["underdog"])
    _ouvrir_le_recapitulatif(page, rapport, rapport["topdog"])

    page.locator(".mr-recap-cancel").click()
    expect(page.locator(".mr-recap-confirm")).to_be_visible()
    expect(page.locator(".mr-recap-cancel")).to_be_hidden()
    page.get_by_role("button", name="Garder").click()

    expect(page.locator(".mr-recap-confirm")).to_be_hidden()
    expect(page.locator(".mr-recap-cancel")).to_be_visible()
    assert "/inducements/" in page.url
    mr_id = rapport["mr_id"]
    assert _evenements(mr_id, "InducementsCancelled") == 0
    assert CHARIOT in _achats_en_projection(mr_id, rapport["topdog"])
    assert _achats_en_projection(mr_id, rapport["underdog"]) == "[]"


# ── 5 · « Oui, tout annuler » ────────────────────────────────────────────────


def _blesser(space_id: str, mr_id: str, victime: str) -> None:
    """Repris de `test_journeyman_recruitment` : une blessure sérieuse rend le
    joueur indisponible au match suivant — seul chemin vers un journalier."""
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step4/actions",
        data={
            "turn": "3",
            "player_id": victime,
            "player_type": "regular",
            "action_type": "BLESSE",
            "injury_type": "BLESSURE_SERIEUSE",
        },
    )
    assert resp.status_code == 200, f"blessure : {resp.status_code}\n{resp.text[:200]}"


def _version_de_l_effectif(team_id: str) -> int:
    rows = query_db(f"SELECT coalesce(sum(version), 0) FROM players_proj WHERE team_id = '{team_id}'")
    return int(rows[0])


def _equipe_a_dix(ctx) -> None:
    """Journée 6 : l'équipe 2 perd un titulaire. Publié et projeté avant de
    rendre la main — sinon le rapport suivant la verrait encore à onze."""
    space_id, full = ctx["space_id"], ctx["full"]
    round_id = full["round_ids"][6]
    home, away = ctx["teams"][2], ctx["teams"][3]
    mr_id = create_draft(space_id, full, round_id, home, away)
    ensure_pre_match(space_id, mr_id, full, round_id, home, away)
    ensure_inducements(space_id, mr_id)
    _blesser(space_id, mr_id, first_player_id(mr_id, "home"))
    version_avant = _version_de_l_effectif(home)
    post_step5(space_id, mr_id, home_gain=5, away_gain=5)
    publish(space_id, mr_id)
    attendre_que(lambda: _version_de_l_effectif(home) > version_avant, quoi="la blessure projetée")


def _journaliers_du_rapport(mr_id: str) -> list[str]:
    rows = query_db(
        "SELECT home_temp_players::text, away_temp_players::text "
        f"FROM match_report_proj WHERE match_report_id = '{mr_id}'"
    )
    home, away = rows[0].split("|", 1)
    temps = json.loads(home or "[]") + json.loads(away or "[]")
    return [t["id"] for t in temps if "Journeyman" in t.get("kind", {})]


def _appartenances(player_ids: list[str]) -> set[str]:
    ids = ",".join(f"'{p}'" for p in player_ids)
    return set(query_db(f"SELECT membership FROM players_proj WHERE player_id IN ({ids})"))


def test_confirm_cancel_reopens_topdog_choice_and_withdraws_journeymen(page: Page, flows_ctx):
    _equipe_a_dix(flows_ctx)
    rapport = _ouvrir_un_rapport(flows_ctx, round_idx=7, home_idx=2, away_idx=3)
    _passer(rapport, rapport["topdog"])
    _passer(rapport, rapport["underdog"])
    mr_id = rapport["mr_id"]
    journaliers = _journaliers_du_rapport(mr_id)
    assert journaliers, "l'équipe à dix doit aligner un journalier"
    attendre_que(lambda: _appartenances(journaliers) == {"Journeyman"}, quoi="les journaliers dans l'effectif")

    _ouvrir_le_recapitulatif(page, rapport, rapport["underdog"])
    page.locator(".mr-recap-cancel").click()
    cliquer_quand_cable(page, ".mr-recap-confirm-cancel")

    page.wait_for_url(re.compile(rf"/inducements/{rapport['topdog']}$"))
    expect(page.locator(".mr-budget-label")).to_contain_text("TopDog")
    expect(page.locator("button[name='intent'][value='pass']")).to_be_visible()
    assert _achats_en_projection(mr_id, rapport["topdog"]) == "NULL"
    assert _achats_en_projection(mr_id, rapport["underdog"]) == "NULL"
    assert _evenements(mr_id, "InducementsCancelled") == 1
    assert _journaliers_du_rapport(mr_id) == []
    attendre_que(lambda: _appartenances(journaliers) == {"Dismissed"}, quoi="les journaliers retirés")


# ── 6 · L'outsider rachète avec un budget recalculé ──────────────────────────


def test_underdog_budget_is_recomputed_after_cancel(page: Page, flows_ctx):
    """Trois valeurs distinctes, comme `test_inducement_treasury` :
      — le budget d'avant, qui comptait l'ancienne dépense du favori ;
      — le budget naïf, qui ajouterait la nouvelle à l'ancienne ;
      — le budget juste, qui ne compte que la nouvelle.
    Le budget d'avant est **lu** à l'écran : l'écart de valeur et l'appoint
    de trésorerie y sont déjà, et ne bougent pas entre les deux lectures."""
    rapport = _ouvrir_un_rapport(flows_ctx, round_idx=4)
    avant_achats, apres_achats = [(ARBITRE, 1), (CHARIOT, 1)], [(CHARIOT, 1)]
    underdog_url = _inducements_url(rapport["space_id"], rapport["mr_id"], rapport["underdog"])

    _acheter(rapport, rapport["topdog"], avant_achats)
    page.goto(underdog_url, wait_until="load")
    avant = _budget_affiche(page)
    _passer(rapport, rapport["underdog"])

    annulation = _annuler_par_http(rapport)
    assert annulation.status_code == 200, f"annulation : {annulation.status_code}\n{annulation.text[:200]}"
    assert annulation.headers.get("HX-Redirect", "").endswith(f"/inducements/{rapport['topdog']}")
    _acheter(rapport, rapport["topdog"], apres_achats)

    juste = avant - _cout(avant_achats) + _cout(apres_achats)
    naif = avant + _cout(apres_achats)
    assert len({avant, naif, juste}) == 3, "le test doit discriminer les trois issues"
    page.goto(underdog_url, wait_until="load")
    assert page.url == underdog_url, f"l'outsider doit pouvoir acheter : {page.url!r}"
    assert _budget_affiche(page) == juste, f"budget juste {juste} (avant {avant}, naïf {naif})"


# ── 7 · Après la première action, plus d'annulation ──────────────────────────


def test_no_cancel_once_an_action_is_recorded(page: Page, flows_ctx):
    rapport = _ouvrir_un_rapport(flows_ctx, round_idx=5)
    _passer(rapport, rapport["topdog"])
    _passer(rapport, rapport["underdog"])
    mr_id = rapport["mr_id"]
    record_action_api(rapport["space_id"], mr_id, "home", first_player_id(mr_id, "home"), turn=1, action_type="TOUCHDOWN")

    _ouvrir_le_recapitulatif(page, rapport, rapport["topdog"])
    expect(page.get_by_role("link", name="Coup d'envoi → Match")).to_be_visible()
    expect(page.locator(".mr-recap-cancel")).to_have_count(0)
    expect(page.locator(".mr-recap-confirm")).to_have_count(0)

    refus = _annuler_par_http(rapport)
    assert refus.status_code == 409, f"annulation après une action : {refus.status_code}\n{refus.text[:200]}"
    assert _evenements(mr_id, "InducementsCancelled") == 0
    assert _achats_en_projection(mr_id, rapport["topdog"]) == "[]"
