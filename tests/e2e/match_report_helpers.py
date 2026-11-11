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

from competition_lifecycle import liberer_les_equipes, BASE_URL
from db_helpers import query_db

_ULID_RE = re.compile(r"/app/[0-9A-Z]{26}/match-report/([0-9A-Z]{26})")


def create_draft(space_id, ctx, round_id, home_team_id, away_team_id, via=None):
    """Malgré son nom, **démarre** le rapport : `POST /match-report/new` le
    confirme au nom de qui l'envoie, et `ensure_pre_match` n'a plus rien à
    faire ensuite.

    `via` : une session `requests` portant une autre identité que DevCoach
    (`realtime_helpers.http_session_of`) — c'est elle qui devient alors le
    démarreur (carte 647). `liberer_les_equipes` reste sous DevCoach : retirer
    un appariement est un geste d'admin.
    """
    liberer_les_equipes(space_id, ctx["competition_id"], ctx["season_id"], round_id, home_team_id, away_team_id)
    resp = (via or requests).post(
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


def draft_of_round(round_id: str) -> str:
    """Le rapport qu'une journée porte dès son appariement (carte 633).

    Ramené de `test_match_report_entry_mode.py` et de
    `test_competition_hors_calendrier.py`, qui en portaient chacun une copie
    identique (carte 639).
    """
    lignes = query_db(
        "SELECT match_report_id FROM match_report_proj "
        f"WHERE round_id = '{round_id}' LIMIT 1"
    )
    assert lignes, "la journée doit porter un rapport, créé avec son appariement"
    return lignes[0].strip()


def entry_mode_of(mr_id: str) -> tuple[str, str, str]:
    """La phase, le mode de saisie et le démarreur, lus sur la projection
    (cartes 631 et 632). Le démarreur est vide tant que le match n'est pas
    démarré.

    Ramené de `test_match_report_entry_mode.py` (carte 639).
    """
    ligne = query_db(
        "SELECT phase || '|' || entry_mode || '|' || COALESCE(started_by, '') "
        f"FROM match_report_proj WHERE match_report_id = '{mr_id}'"
    )[0]
    phase, mode, demarreur = ligne.split("|")
    return phase, mode, demarreur


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


def ensure_inducements(space_id, mr_id, via=None):
    http = via or requests
    resp = http.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "2", "away_fan_roll": "3"},
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"fan factor: {resp.status_code}"
    location = resp.headers.get("Location", "")
    for _ in range(3):
        if not location or "/inducements/" not in location:
            break
        resp = http.post(f"{BASE_URL}{location}", data={"selection": ""}, allow_redirects=False)
        if resp.status_code not in (302, 303):
            break
        location = resp.headers.get("Location", "")


def start_match(space_id, ctx, round_id, home, away, via=None) -> str:
    """Un rapport démarré, coups de pouce passés — ramené de
    `test_realtime_direct.py` (carte 647). `via` : qui le démarre — par
    `create_draft` —, puis passe les coups de pouce ; DevCoach par défaut."""
    mr_id = create_draft(space_id, ctx, round_id, home, away, via=via)
    ensure_pre_match(space_id, mr_id, ctx, round_id, home, away)
    ensure_inducements(space_id, mr_id, via=via)
    return mr_id


def pass_topdog_first(space_id, mr_id, team_id):
    """Fait passer le favori si `team_id` est l'outsider — carte 627, R9.

    Le favori choisit ses coups de pouce en premier : tant qu'il n'a pas
    validé, l'outsider est refusé en 409. Le favori se lit par la page de
    l'outsider, qui redirige vers la sienne ; une page servie sans
    redirection dit que `team_id` peut acheter, et rien n'est fait.

    À appeler **avant** l'achat de `team_id`, sur un rapport où elle n'a pas
    encore validé : sinon la redirection mène à l'autre équipe (R7).
    """
    url = f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/inducements/{team_id}"
    resp = requests.get(url, allow_redirects=False)
    if resp.status_code not in (302, 303):
        return
    topdog_url = f"{BASE_URL}{resp.headers['Location']}"
    passed = requests.post(
        topdog_url,
        data={"intent": "pass", "selection": "[]", "mercenaries": "[]"},
        allow_redirects=False,
    )
    assert passed.status_code in (302, 303), (
        f"le favori n'a pas pu passer : {passed.status_code}\n{passed.text[:200]}"
    )


def pass_inducements_in_browser(page) -> None:
    """Le favori puis l'outsider passent ; le récapitulatif offre le coup
    d'envoi. Rien à faire si le tier n'a pas de coups de pouce.

    Le seul geste **au navigateur** de ce module : ramené de
    `test_match_report_live.py`, pour que le parcours complet de la carte 639
    traverse les coups de pouce de la même façon, sans en porter une copie.
    """
    passer = page.locator("button[name='intent'][value='pass']")
    for _ in range(2):
        if "/inducements/" not in page.url or passer.count() == 0:
            return
        with page.expect_navigation(wait_until="load"):
            passer.click()
    coup_d_envoi = page.get_by_role("link", name="Coup d'envoi → Match")
    if coup_d_envoi.count():
        with page.expect_navigation(wait_until="load"):
            coup_d_envoi.click()


def record_action_api(space_id, mr_id, side, player_id, turn, action_type, via=None):
    """`via` : comme pour `create_draft`, l'auteur de la saisie."""
    endpoint = "step3" if side == "home" else "step4"
    resp = (via or requests).post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{endpoint}/actions",
        data={"turn": str(turn), "player_id": player_id, "player_type": "regular", "action_type": action_type},
    )
    assert resp.status_code == 200, f"record_action {action_type}: {resp.status_code}\n{resp.text[:200]}"


def record_mvp_api(space_id, mr_id, side, player_id, player_type="regular"):
    """Désigne le MVP d'un camp par son étape (carte 637).

    Le POST d'action refuse `MVP` : le MVP n'entre que par `/mvp/{side}`, au
    tour 16, et un seul par camp — un second est refusé en 422.
    """
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/mvp/{side}",
        data={"player_id": player_id, "player_type": player_type},
        allow_redirects=False,
    )
    assert resp.status_code == 200, f"record_mvp {side}: {resp.status_code}\n{resp.text[:200]}"
    return resp


def ensure_mvps(space_id, mr_id):
    """Donne son MVP à chaque camp qui n'en a pas (carte 638).

    Le MVP est obligatoire : l'après-match ne se clôt pas sans. Les tests qui
    ne s'intéressent pas au MVP traversent l'étape par ici, sans toucher à
    ceux qui l'ont déjà désigné eux-mêmes.

    Le MVP rapporte des SPP : il est donné au **dernier** joueur actif de
    l'équipe qui n'a aucune action dans ce rapport, pour ne pas se mêler des
    joueurs qu'un test observe — ceux-là ont marqué, sorti ou été blessés.
    """
    for side in ("home", "away"):
        if not _has_mvp(mr_id, side):
            record_mvp_api(space_id, mr_id, side, _mvp_candidate(mr_id, side))


def _has_mvp(mr_id, side):
    return bool(query_db(
        f"SELECT 1 FROM match_report_actions WHERE match_report_id = '{mr_id}' "
        f"AND team_side = '{side}' AND NOT is_deleted AND action_json = '\"Mvp\"'::jsonb"
    ))


def _mvp_candidate(mr_id, side):
    rows = _query_side_team(mr_id, side)
    assert rows, f"match report {mr_id} introuvable en DB"
    candidates = query_db(
        f"SELECT player_id FROM players_proj WHERE team_id = '{rows[0]}' "
        f"AND membership = 'Active' AND player_id NOT IN ("
        f"SELECT player_id FROM match_report_actions WHERE match_report_id = '{mr_id}' "
        f"AND NOT is_deleted) ORDER BY player_id DESC LIMIT 1"
    )
    assert candidates, f"aucun joueur libre pour le MVP de l'équipe {rows[0]}"
    return candidates[0]


def first_player_id(mr_id, side):
    rows = _query_side_team(mr_id, side)
    assert rows, f"match report {mr_id} introuvable en DB"
    player_rows = query_db(f"SELECT player_id FROM players_proj WHERE team_id = '{rows[0]}' LIMIT 1")
    assert player_rows, f"aucun joueur trouvé pour l'équipe {rows[0]}"
    return player_rows[0]


def _query_side_team(mr_id, side):
    return query_db(f"SELECT {side}_team_id FROM match_report_proj WHERE match_report_id = '{mr_id}'")


def post_step5(
    space_id,
    mr_id,
    *,
    home_gain=50000,
    away_gain=40000,
    summary_title="Match E2E",
    summary_body="Compte-rendu automatique.",
):
    """Les gains sont en **kPo** (cf. l'unité affichée par step5.html).

    Les valeurs par défaut sont celles d'origine, conservées telles quelles pour
    ne rien changer aux tests existants — mais elles sont absurdes à l'échelle du
    jeu : 50 000 kPo, soit cinquante fois le budget de création d'une équipe.
    Sans incidence tant qu'on regarde le classement ; à paramétrer dès qu'un test
    observe la trésorerie, qu'elles rendraient sinon inépuisable.

    Chaque camp reçoit d'abord son MVP s'il n'en a pas : sans, l'après-match est
    refusé (carte 638).

    Le titre et le corps du résumé sont paramétrables depuis la carte 639 : les
    copies privées de quatre fichiers, qui ne différaient plus que par eux
    depuis qu'elles appelaient `ensure_mvps`, sont ramenées ici.
    """
    ensure_mvps(space_id, mr_id)
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step5",
        data={
            "home_gain": str(home_gain),
            "away_gain": str(away_gain),
            "home_fan_mod": "1",
            "away_fan_mod": "-1",
            "summary_title": summary_title,
            "summary_body": summary_body,
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
