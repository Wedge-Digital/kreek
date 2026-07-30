"""Publication de rapports de match par API — helpers partagés.

Extraits de `test_ranking_bonus.py`, qui les portait en privé, et généralisés
sur un seul point : le score des **deux** équipes est paramétrable, là où la
version d'origine ne savait faire marquer que l'équipe home.

Publier un rapport impose de traverser tout le parcours (brouillon → pré-match
→ inducements → actions → gains → publication). Ces fonctions l'automatisent en
HTTP direct plutôt qu'au navigateur : le parcours lui-même est couvert par les
tests `test_match_report_*`, le refaire au clic ici ne testerait rien de plus et
multiplierait le temps d'exécution.

Les équipes sont désignées par leur id et non par un index dans le contexte :
les fixtures des différents fichiers de test ne nomment pas leur liste
d'équipes de la même façon (`teams` ici, `team_ids` dans
`competition_lifecycle.build_full_competition`).
"""

import re
import time

import requests

from competition_lifecycle import BASE_URL
from db_helpers import query_db

_ULID_RE = re.compile(r"/app/[0-9A-Z]{26}/match-report/([0-9A-Z]{26})")


def create_draft(space_id, ctx, round_id, home_team_id, away_team_id):
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/new",
        data={
            "competition_id": ctx["competition_id"],
            "season_id": ctx["season_id"],
            "round_id": round_id,
            "home_team_id": home_team_id,
            "away_team_id": away_team_id,
        },
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"create: {resp.status_code}\n{resp.text[:200]}"
    m = _ULID_RE.search(resp.headers.get("Location", ""))
    assert m, f"match_report_id introuvable dans Location: {resp.headers.get('Location')!r}"
    return m.group(1)


def ensure_pre_match(space_id, mr_id, ctx, round_id, home_team_id, away_team_id):
    check = requests.get(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2", allow_redirects=False)
    if check.status_code == 200:
        return
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}",
        data={
            "competition_id": ctx["competition_id"],
            "season_id": ctx["season_id"],
            "round_id": round_id,
            "home_team_id": home_team_id,
            "away_team_id": away_team_id,
        },
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"confirm: {resp.status_code}\n{resp.text[:200]}"


def ensure_inducements(space_id, mr_id):
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "2", "away_fan_roll": "3"},
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"fan factor: {resp.status_code}"
    location = resp.headers.get("Location", "")
    for _ in range(3):
        if not location or "/inducements/" not in location:
            break
        resp = requests.post(f"{BASE_URL}{location}", data={"selection": ""}, allow_redirects=False)
        if resp.status_code not in (302, 303):
            break
        location = resp.headers.get("Location", "")


def record_action_api(space_id, mr_id, side, player_id, turn, action_type):
    endpoint = "step3" if side == "home" else "step4"
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{endpoint}/actions",
        data={"turn": str(turn), "player_id": player_id, "player_type": "regular", "action_type": action_type},
    )
    assert resp.status_code == 200, f"record_action {action_type}: {resp.status_code}\n{resp.text[:200]}"


def first_player_id(mr_id, side):
    rows = _query_side_team(mr_id, side)
    assert rows, f"match report {mr_id} introuvable en DB"
    player_rows = query_db(f"SELECT player_id FROM players_proj WHERE team_id = '{rows[0]}' LIMIT 1")
    assert player_rows, f"aucun joueur trouvé pour l'équipe {rows[0]}"
    return player_rows[0]


def _query_side_team(mr_id, side):
    return query_db(f"SELECT {side}_team_id FROM match_report_proj WHERE match_report_id = '{mr_id}'")


def post_step5(space_id, mr_id, *, home_gain=50000, away_gain=40000):
    """Les gains sont en **kPo** (cf. l'unité affichée par step5.html).

    Les valeurs par défaut sont celles d'origine, conservées telles quelles pour
    ne rien changer aux tests existants — mais elles sont absurdes à l'échelle du
    jeu : 50 000 kPo, soit cinquante fois le budget de création d'une équipe.
    Sans incidence tant qu'on regarde le classement ; à paramétrer dès qu'un test
    observe la trésorerie, qu'elles rendraient sinon inépuisable.
    """
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step5",
        data={
            "home_gain": str(home_gain),
            "away_gain": str(away_gain),
            "home_fan_mod": "1",
            "away_fan_mod": "-1",
            "summary_title": "Match E2E",
            "summary_body": "Compte-rendu automatique.",
        },
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"step5: {resp.status_code}\n{resp.text[:200]}"


def publish(space_id, mr_id):
    resp = requests.post(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap/publish", allow_redirects=False)
    assert resp.status_code in (302, 303), f"publish: {resp.status_code}\n{resp.text[:200]}"


def play_match(
    space_id,
    ctx,
    round_id,
    home_team_id,
    away_team_id,
    *,
    home_td=1,
    away_td=0,
    home_sorties=0,
    home_gain=50000,
    away_gain=40000,
):
    """Joue et publie un match au score voulu.

    Les tours sont numérotés en continu par équipe ; leur valeur n'a aucune
    incidence sur le classement, seuls comptent le nombre et le type d'actions.
    """
    mr_id = create_draft(space_id, ctx, round_id, home_team_id, away_team_id)
    ensure_pre_match(space_id, mr_id, ctx, round_id, home_team_id, away_team_id)
    ensure_inducements(space_id, mr_id)

    home_player = first_player_id(mr_id, "home")
    for turn in range(home_td):
        record_action_api(space_id, mr_id, "home", home_player, turn=turn + 1, action_type="TOUCHDOWN")
    for i in range(home_sorties):
        record_action_api(space_id, mr_id, "home", home_player, turn=home_td + i + 1, action_type="SORTIE")

    if away_td:
        away_player = first_player_id(mr_id, "away")
        for turn in range(away_td):
            record_action_api(space_id, mr_id, "away", away_player, turn=turn + 1, action_type="TOUCHDOWN")

    post_step5(space_id, mr_id, home_gain=home_gain, away_gain=away_gain)
    publish(space_id, mr_id)
    return mr_id


def wait_ranking_points(season_id, team_id, timeout_s=20):
    """Attend que la projection ranking (asynchrone via app event) soit peuplée."""
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        rows = query_db(
            f"SELECT ranking_points FROM ranking_lines "
            f"WHERE season_id = '{season_id}' AND team_id = '{team_id}' "
            f"ORDER BY sequence DESC LIMIT 1"
        )
        if rows:
            return int(rows[0])
        time.sleep(0.3)
    raise AssertionError(f"aucune ligne de classement pour team={team_id} après {timeout_s}s")


def wait_ranking_lines(season_id, expected_lines, timeout_s=30):
    """Attend que **tous** les matchs publiés soient projetés — deux lignes par
    match. Compter les équipes distinctes ne suffirait pas : une équipe qui joue
    deux matchs apparaîtrait dès le premier, et le classement serait lu à
    mi-chemin.
    """
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        rows = query_db(f"SELECT COUNT(*) FROM ranking_lines WHERE season_id = '{season_id}'")
        if rows and int(rows[0]) >= expected_lines:
            return
        time.sleep(0.3)
    raise AssertionError(
        f"seulement {rows[0] if rows else 0} lignes de classement sur {expected_lines} attendues "
        f"après {timeout_s}s (saison {season_id})"
    )
