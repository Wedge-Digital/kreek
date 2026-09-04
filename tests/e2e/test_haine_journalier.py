"""La Haine d'un journalier ne rejoint aucun agrégat joueur (carte 404, R9).

C'est le seul scénario de la Haine qui vérifie une **absence d'écriture
ailleurs** : le journalier n'existe que le temps du match, sa Haine reste dans
le rapport — visible au récapitulatif — et n'atteint jamais `players`.

Le filtre qui l'assure existait avant la fonctionnalité (`ActionPlayer::Regular`,
BR1) et un test unitaire le couvre déjà. Celui-ci vérifie la chaîne entière,
publication comprise.

**Obtenir un journalier demande deux matchs.** Il n'en apparaît que si l'équipe
compte moins de onze joueurs disponibles : le premier match inflige une Blessure
Sérieuse, qui rend son porteur indisponible au suivant ; le second voit donc
l'équipe à dix, et le serveur lui adjoint un journalier.

Fichier séparé, et compétition à deux équipes : celle de
`test_match_report_recap` a ses douze équipes déjà engagées, et publier une
blessure sur l'une d'elles ferait dépendre ses tests de l'ordre d'exécution.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import json
import re

import pytest
import requests

from db_helpers import attendre_que, query_db

BASE_URL = "http://localhost:3210"


@pytest.fixture(scope="module")
def contexte(browser, space_id):
    from competition_lifecycle import build_full_competition

    full = build_full_competition(browser, space_id, num_teams=2, num_rounds=3)
    assert len(full["round_ids"]) >= 3, (
        "trois journées : deux pour obtenir un journalier, une pour le pendant positif"
    )
    return full


_ULID_RE = re.compile(r"/app/[0-9A-Z]{26}/match-report/([0-9A-Z]{26})")


def _creer_rapport(space_id, ctx, round_id, home, away):
    """Crée le rapport puis confirme la sélection — `/new` dédoublonne, la
    confirmation est un no-op si le rapport est déjà en PreMatch."""
    champs = {
        "competition_id": ctx["competition_id"],
        "season_id": ctx["season_id"],
        "round_id": round_id,
        "home_team_id": home,
        "away_team_id": away,
    }
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/new", data=champs, allow_redirects=False
    )
    assert resp.status_code in (302, 303), f"création : {resp.status_code}\n{resp.text[:300]}"
    m = _ULID_RE.search(resp.headers.get("Location", ""))
    assert m, f"identifiant introuvable dans Location : {resp.headers.get('Location')!r}"
    mr_id = m.group(1)

    if requests.get(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2", allow_redirects=False
    ).status_code != 200:
        resp = requests.post(
            f"{BASE_URL}/app/{space_id}/match-report/{mr_id}", data=champs, allow_redirects=False
        )
        assert resp.status_code in (302, 303), f"confirmation : {resp.status_code}"
    return mr_id


def _passer_en_pre_match(space_id, mr_id):
    """Facteur fans puis coups de pouce vides — c'est ici que les journaliers
    sont adjoints à une équipe incomplète."""
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "2", "away_fan_roll": "3"},
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"facteur fans : {resp.status_code}"
    location = resp.headers.get("Location", "")
    for _ in range(3):
        if not location or "/inducements/" not in location:
            break
        resp = requests.post(f"{BASE_URL}{location}", data={"selection": ""}, allow_redirects=False)
        if resp.status_code not in (302, 303):
            break
        location = resp.headers.get("Location", "")


def _enregistrer(space_id, mr_id, side, player_id, turn, **champs):
    data = {"turn": str(turn), "player_id": player_id, "player_type": "regular"}
    data.update({k: str(v) for k, v in champs.items()})
    endpoint = "step3" if side == "home" else "step4"
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{endpoint}/actions", data=data
    )
    assert resp.status_code == 200, f"action : {resp.status_code}\n{resp.text[:300]}"


def _publier(space_id, mr_id):
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step5",
        data={
            "home_gain": "50000",
            "away_gain": "40000",
            "home_fan_mod": "1",
            "away_fan_mod": "-1",
            "summary_title": "Match capté par les tests E2E",
            "summary_body": "Compte-rendu généré automatiquement.",
        },
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"step5 : {resp.status_code}\n{resp.text[:300]}"
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap/publish", allow_redirects=False
    )
    assert resp.status_code in (302, 303), f"publication : {resp.status_code}\n{resp.text[:300]}"


def _un_joueur(team_id):
    rows = query_db(f"SELECT player_id FROM players_proj WHERE team_id = '{team_id}' LIMIT 1")
    assert rows, f"aucun joueur dans l'équipe {team_id}"
    return rows[0]


def _journalier(mr_id, side):
    rows = query_db(
        f"SELECT {side}_temp_players FROM match_report_proj WHERE match_report_id = '{mr_id}'"
    )
    temps = json.loads(rows[0]) if rows and rows[0] else []
    for t in temps:
        if "Journeyman" in t.get("kind", {}):
            return t["id"]
    return None


def _competences_de_blessure(team_id):
    """Les compétences en mode `Injury` de tout l'effectif — celles que seule
    une Haine peut produire."""
    rows = query_db(
        f"SELECT jsonb_array_length(coalesce(acquired_skills, '[]'::jsonb)) "
        f"FROM players_proj WHERE team_id = '{team_id}'"
    )
    _ = rows
    lignes = query_db(
        "SELECT count(*) FROM players_proj p, "
        "jsonb_array_elements(coalesce(p.acquired_skills, '[]'::jsonb)) s "
        f"WHERE p.team_id = '{team_id}' AND s->>'mode' = 'Injury'"
    )
    return int(lignes[0])


def _version_de_l_effectif(team_id):
    """Somme des versions de la projection — un marqueur de progression.

    Chaque événement appliqué à un joueur incrémente sa version. Attendre que
    cette somme bouge prouve que le pipeline d'app events a tourné, ce qu'une
    assertion négative ne peut pas déduire de son propre résultat.
    """
    lignes = query_db(
        f"SELECT coalesce(sum(version), 0) FROM players_proj WHERE team_id = '{team_id}'"
    )
    return int(lignes[0])


def test_la_haine_d_un_journalier_n_atteint_aucun_joueur(space_id, contexte):
    domicile, exterieur = contexte["team_ids"][0], contexte["team_ids"][1]
    journees = contexte["round_ids"]

    # ── Match 1 : une Blessure Sérieuse rend un joueur indisponible ──────────
    mr1 = _creer_rapport(space_id, contexte, journees[0], domicile, exterieur)
    _passer_en_pre_match(space_id, mr1)
    _enregistrer(
        space_id, mr1, "home", _un_joueur(domicile), turn=1,
        action_type="BLESSE", injury_type="BLESSURE_SERIEUSE",
    )
    version_avant_mr1 = _version_de_l_effectif(domicile)
    _publier(space_id, mr1)
    # La blessure doit être **projetée** avant qu'on demande le second rapport :
    # tant qu'elle ne l'est pas, l'équipe est encore à onze et aucun journalier
    # n'est adjoint — le test échouerait en accusant la règle des journaliers.
    attendre_que(
        lambda: _version_de_l_effectif(domicile) > version_avant_mr1,
        quoi="la projection de la blessure du premier match",
    )

    avant = _competences_de_blessure(domicile)
    version_avant_mr2 = _version_de_l_effectif(domicile)

    # ── Match 2 : l'équipe est à dix, le serveur lui adjoint un journalier ───
    mr2 = _creer_rapport(space_id, contexte, journees[1], domicile, exterieur)
    _passer_en_pre_match(space_id, mr2)
    journalier = _journalier(mr2, "home")
    assert journalier, (
        "aucun journalier au second match : la Blessure Sérieuse du premier "
        "devait rendre un joueur indisponible et faire passer l'équipe à dix"
    )

    # ── Le journalier est blessé et gagne une Haine ──────────────────────────
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr2}/step3/actions",
        data={
            "turn": "1",
            "player_id": journalier,
            "player_type": "temp",
            "action_type": "BLESSE",
            "injury_type": "AMOCHE",
            "hate_gained": "true",
            "hate_keyword": "DARK_ELF",
        },
    )
    assert resp.status_code == 200, f"action du journalier : {resp.status_code}\n{resp.text[:300]}"
    _publier(space_id, mr2)

    # **Sans ce marqueur, l'assertion ci-dessous est creuse.** « Rien n'a
    # changé » est vrai aussi quand rien n'est jamais arrivé : le test passerait
    # sur une chaîne entièrement cassée. On attend donc la preuve que le
    # pipeline a tourné — la version de l'effectif bouge — avant de vérifier
    # qu'il n'a rien écrit dans les compétences.
    attendre_que(
        lambda: _version_de_l_effectif(domicile) > version_avant_mr2,
        quoi="la projection du second match",
    )

    apres = _competences_de_blessure(domicile)
    assert apres == avant, (
        f"la Haine d'un journalier a atteint l'effectif : {avant} → {apres} "
        "compétences en mode Injury"
    )


def test_la_haine_d_un_joueur_permanent_atteint_l_effectif(page, space_id, contexte):
    """Le pendant positif — et ce qui rend le test précédent concluant.

    Sans lui, « aucune compétence en mode Injury » passerait aussi bien si la
    chaîne entière était cassée. Il vérifie en outre la couleur du badge, que
    seul le navigateur peut constater : la projection écrivait la Haine avec une
    classe **vide** avant la carte 405, et trois feuilles sur quatre ne
    portaient pas celle des traits.
    """
    domicile, exterieur = contexte["team_ids"][1], contexte["team_ids"][0]
    mr = _creer_rapport(space_id, contexte, contexte["round_ids"][2], domicile, exterieur)
    _passer_en_pre_match(space_id, mr)

    avant = _competences_de_blessure(domicile)
    _enregistrer(
        space_id, mr, "home", _un_joueur(domicile), turn=1,
        action_type="BLESSE", injury_type="AMOCHE",
        hate_gained="true", hate_keyword="DARK_ELF",
    )
    _publier(space_id, mr)

    # La condition **est** l'assertion : la Haine transite par le bus d'app
    # events, elle n'est pas projetée au retour de la publication. L'échec au
    # bout du délai porte le bon message — il accuse la chaîne, pas la règle.
    attendre_que(
        lambda: _competences_de_blessure(domicile) == avant + 1,
        quoi="la Haine d'un joueur permanent rejoignant ses compétences acquises",
    )

    page.goto(f"{BASE_URL}/app/{space_id}/teams/{domicile}", wait_until="load")
    badge = page.locator(".skill-tag.type-traits", has_text="Haine").first
    badge.wait_for(timeout=10000)

