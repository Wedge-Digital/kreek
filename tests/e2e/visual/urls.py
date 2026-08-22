"""Les pages capturées par le harnais visuel, et de quoi les atteindre.

Les identifiants viennent de la base de dev : la suite e2e y a laissé des
équipes, des compétitions et des rapports de match dans tous les états. Les
recréer serait plus long, plus fragile, et n'apporterait rien — le harnais
compare **deux captures de la même base**, prises à quelques minutes d'écart.
C'est ce qui permet la comparaison octet à octet : entre l'avant et l'après,
seule la feuille de style a changé.

Une URL dont l'entité est introuvable n'est pas silencieusement omise : elle est
rendue avec `url=None`, et le harnais la compte comme non couverte. Une page
qu'on croit vérifiée alors qu'elle ne l'est pas est pire qu'une page qu'on sait
manquante.
"""

import os
import subprocess
from pathlib import Path

BASE = os.environ.get("E2E_BASE_URL", "http://localhost:3210")
ESPACE_E2E = "Espace E2E"


def _url_base_de_donnees() -> str:
    env = Path(__file__).resolve().parents[3] / ".env.dev"
    for ligne in env.read_text().splitlines():
        if ligne.startswith("DATABASE__URL="):
            return ligne.split("=", 1)[1].strip()
    raise RuntimeError(f"DATABASE__URL introuvable dans {env}")


def _un(sql: str) -> str | None:
    r = subprocess.run(["psql", _url_base_de_donnees(), "-t", "-A", "-c", sql],
                       capture_output=True, text=True, timeout=15)
    if r.returncode != 0:
        return None
    lignes = [l.strip() for l in r.stdout.strip().splitlines() if l.strip()]
    return lignes[0] if lignes else None


# La **classe de portée** que chaque page doit porter dans son DOM.
#
# Sans ce contrôle, une URL qui redirige capture une autre page et compte quand
# même : c'est arrivé sur `step2` et `step5`, qui ont rendu la liste des équipes.
# Une couverture qui monte sans rien couvrir est le pire des états.
#
# Le contrôle portait sur la **feuille chargée** jusqu'à la carte 342, qui
# réunit toutes les feuilles en un fichier unique : il n'y aurait plus qu'un
# `<link>`, et le contrôle dirait « aucune page n'a chargé sa feuille » sur les
# 43 pages. Porter le contrôle sur la classe est plus juste de toute façon — il
# vérifie que la page a rendu **son** contenu, indépendamment de la façon dont
# le CSS est livré.
#
# La correspondance est mécanique depuis la carte 341 : le nom du fichier **est**
# la classe de portée.
CLASSE_ATTENDUE = {
    "auth-login": ".auth-layout",
    "espaces-tous": ".allspace-home-grid",
    "espace-nouveau": ".new-space",
    "accueil": ".app-news-feed",
    "article-nouveau": ".editor-container",
    "article-detail": ".article-container",
    "equipes-mes": ".my-teams-container",
    "equipe-brouillon": ".app-new-team",
    "equipe-detail": ".team-page",
    "equipe-recrutement": ".rec-page",
    "equipe-renvois": ".dis-page",
    "equipe-construction": ".team-build",
    "equipe-finalisation": ".finalize-page",
    "joueur-detail": ".player-page",
    "joueur-debug": ".player-debug",
    "competitions-liste": ".competition-container",
    "competition-nouvelle": ".new-competition",
    "competition-detail": ".competition-detail",
    "admin-dashboard": ".competition-admin-dashboard",
    "admin-inscriptions": ".competition-admin-enrollments",
    "admin-groupes": ".competition-admin-groups",
    "admin-calendrier": ".competition-admin-schedule",
    "admin-resume": ".new-competition-phase-5",
    "rapport-selection": ".match-report-step1",
    "rapport-prematch": ".match-report-pre-match",
    "rapport-actions": ".match-report-actions",
    "rapport-step5": ".match-report-step5",
    "rapport-recap": ".ms-page",
    "competition-structure": ".new-competition-phase-3",
    "competition-regles": ".new-competition-phase-2",
    "competition-invitations": ".new-competition-phase-4",
    "classement-detaille": ".ranking-detailed-standings-widget",
    "widget-roster-picker": ".roster-picker",
    "widget-skill-picker": ".skill-picker",
    "widget-inducement-picker": ".inducement-grid",
    "widget-inducement-selector": ".inducement-selector",
    "widget-customisation": ".pd-right",
    "rapport-inducements": ".match-report-inducements",
    "rapport-mercenaires": ".merco-selector",
}


def collecter() -> tuple[dict[str, str], list[str]]:
    """Rend (pages atteignables, noms des pages non atteignables)."""
    espace = _un(f"SELECT id FROM spaces WHERE space_name = '{ESPACE_E2E}' LIMIT 1")
    if not espace:
        raise SystemExit(
            f"Espace « {ESPACE_E2E} » introuvable — lance `make seed_e2e` d'abord."
        )

    equipe = _un(f"SELECT team_id FROM team_proj WHERE space_id = '{espace}' LIMIT 1")
    # `team_drafts.id`, pas `team_id` : la première version cherchait une colonne
    # inexistante et rendait `None` en silence — deux pages disparaissaient du
    # harnais sans que le compte de couverture s'en émeuve.
    brouillon = _un(f"SELECT id FROM team_drafts WHERE space_id = '{espace}' LIMIT 1")
    joueur = _un(
        f"SELECT p.player_id FROM players_proj p "
        f"JOIN team_proj t ON t.team_id = p.team_id "
        f"WHERE t.space_id = '{espace}' LIMIT 1"
    )
    # Aucun article dans l'espace E2E : on en prend un là où il y en a, et on
    # capture sa page dans **son** espace. Le harnais mesure du rendu, pas une
    # règle d'appartenance.
    art = _un("SELECT space_id, id FROM articles LIMIT 1")
    espace_article, article = (art.split("|") + [None])[:2] if art else (None, None)
    comp = _un(
        f"SELECT c.id, s.id FROM competitions c "
        f"JOIN competition_seasons s ON s.competition_id = c.id "
        f"WHERE c.space_id = '{espace}' LIMIT 1"
    )
    competition, saison = (comp.split("|") + [None])[:2] if comp else (None, None)
    # Les pages de création ne se rendent que pour une saison encore en
    # `draft` : une saison créée redirige. La première version tirait une
    # saison au hasard et capturait la redirection.
    brouillon_comp = _un(
        f"SELECT c.id || '|' || s.id FROM competition_seasons s "
        f"JOIN competitions c ON c.id = s.competition_id "
        f"WHERE c.space_id = '{espace}' AND s.status = 'draft' LIMIT 1"
    )
    comp_draft, saison_draft = (
        (brouillon_comp.split("|") + [None])[:2] if brouillon_comp else (None, None)
    )
    # Un rapport par phase : les étapes ne se rendent que dans l'état qui leur
    # correspond, et une URL servie hors phase **redirige**. La première version
    # prenait un rapport au hasard : `step2` et `step5` ont capturé la liste des
    # équipes, en chargeant huit feuilles — une couverture qui comptait juste et
    # ne montrait rien.
    def rapport_en(phase: str) -> str | None:
        return _un(f"SELECT match_report_id FROM match_report_proj "
                   f"WHERE space_id = '{espace}' AND phase = '{phase}' LIMIT 1")

    rapport_prematch = rapport_en("PreMatch")
    rapport_publie = rapport_en("Published")
    rapport_pret = rapport_en("ReadyToPublish")
    rapport_brouillon = rapport_en("Draft")
    # Le skill-picker exige un `roster_line_id` : sans lui il rend un 400, et la
    # capture ramenait une page d'erreur qui chargeait pourtant `common.css` —
    # d'où l'utilité du contrôle de feuille attendue.
    ligne_roster = _un(
        "SELECT roster_line_id FROM players_proj WHERE roster_line_id IS NOT NULL LIMIT 1"
    )
    equipe_prematch = _un(
        f"SELECT home_team_id FROM match_report_proj WHERE match_report_id = '{rapport_prematch}'"
    ) if rapport_prematch else None

    a = f"{BASE}/app/{espace}"
    candidates: dict[str, str | None] = {
        # ── hors espace ──────────────────────────────────────────────────
        "auth-login":            f"{BASE}/auth/login",
        "auth-register":         f"{BASE}/auth/register",
        "auth-forgot-password":  f"{BASE}/auth/forgot-password",
        "espaces-tous":          f"{BASE}/app/space/all",
        "espace-nouveau":        f"{BASE}/app/space/create",
        # ── espace ───────────────────────────────────────────────────────
        "accueil":               f"{a}/home",
        "article-nouveau":       f"{a}/home/articles/new",
        "article-detail":        f"{BASE}/app/{espace_article}/home/articles/{article}" if article else None,
        # ── équipes ──────────────────────────────────────────────────────
        "equipes-mes":           f"{a}/team/list",
        "equipe-brouillon":      f"{a}/team/create",
        "equipe-detail":         f"{a}/teams/{equipe}" if equipe else None,
        "equipe-recrutement":    f"{a}/teams/{equipe}/recruitment" if equipe else None,
        "equipe-renvois":        f"{a}/teams/{equipe}/dismissals" if equipe else None,
        "equipe-construction":   f"{a}/team/{brouillon}/build" if brouillon else None,
        "equipe-finalisation":   f"{a}/team/{brouillon}/finalize" if brouillon else None,
        # ── joueurs ──────────────────────────────────────────────────────
        "joueur-detail":         f"{a}/players/{joueur}/detail" if joueur else None,
        "joueur-debug":          f"{a}/players/{joueur}/debug" if joueur else None,
        # ── compétitions ─────────────────────────────────────────────────
        "competitions-liste":    f"{a}/competitions",
        "competition-nouvelle":  f"{a}/competitions/create",
        "competition-detail":    f"{a}/competitions/{competition}/{saison}" if saison else None,
        "competition-calendrier": f"{a}/competitions/{competition}/{saison}/calendrier" if saison else None,
        "admin-dashboard":       f"{a}/competitions/{competition}/{saison}/admin/dashboard" if saison else None,
        "admin-inscriptions":    f"{a}/competitions/{competition}/{saison}/admin/enrollments" if saison else None,
        "admin-groupes":         f"{a}/competitions/{competition}/{saison}/admin/groups" if saison else None,
        "admin-calendrier":      f"{a}/competitions/{competition}/{saison}/admin/schedule" if saison else None,
        "admin-resultats":       f"{a}/competitions/{competition}/{saison}/admin/results" if saison else None,
        "admin-resume":          f"{a}/competitions/{competition}/{saison}/admin/summary" if saison else None,
        # ── phases de création de compétition ────────────────────────────
        #
        # Les libellés d'URL et les feuilles ne se correspondent pas :
        # `/structure` charge `phase-3.css` et `/rules` charge `phase-2.css`.
        # C'est vérifié, pas supposé — le contrôle de feuille attendue l'aurait
        # signalé sinon.
        "competition-structure": f"{a}/competitions/create/{comp_draft}/{saison_draft}/structure" if saison_draft else None,
        "competition-regles":    f"{a}/competitions/create/{comp_draft}/{saison_draft}/rules" if saison_draft else None,
        "competition-invitations": f"{a}/competitions/create/{comp_draft}/{saison_draft}/invitations" if saison_draft else None,
        "classement-detaille":   f"{a}/competitions/{competition}/{saison}/detailed-standings" if saison else None,
        # ── widgets, atteints par leur propre endpoint ───────────────────
        #
        # Un widget est un fragment autonome exposé par un GET dédié
        # (cf. CLAUDE.md, « Conventions widgets HTMX ») : il embarque son
        # `<link>` et se rend seul. Le capturer directement évite de simuler
        # l'interaction qui le fait apparaître dans sa page hôte.
        "widget-roster-picker":  f"{BASE}/references/roster-picker",
        "widget-skill-picker":   (f"{BASE}/references/roster-lines/skill-picker"
                                  f"?roster_line_id={ligne_roster}&spp_remaining=6"
                                  if ligne_roster else None),
        "widget-inducement-picker":   f"{BASE}/references/inducement-picker",
        "widget-inducement-selector": f"{BASE}/references/inducement-selector",
        "widget-customisation":  f"{a}/players/{joueur}/widgets/customisation" if joueur else None,
        # ── rapports de match ────────────────────────────────────────────
        "rapport-selection":     f"{a}/match-report/{rapport_brouillon}" if rapport_brouillon else None,
        "rapport-prematch":      f"{a}/match-report/{rapport_prematch}/step2" if rapport_prematch else None,
        "rapport-actions":       f"{a}/match-report/{rapport_prematch}/step3" if rapport_prematch else None,
        "rapport-step5":         f"{a}/match-report/{rapport_pret}/step5" if rapport_pret else None,
        "rapport-recap":         f"{a}/match-report/{rapport_publie}/recap" if rapport_publie else None,
        "rapport-inducements":   (f"{a}/match-report/{rapport_prematch}/inducements/{equipe_prematch}"
                                  if rapport_prematch and equipe_prematch else None),
        "rapport-mercenaires":   (f"{a}/match-report/{rapport_prematch}/step2/{equipe_prematch}/mercenaires"
                                  if rapport_prematch and equipe_prematch else None),
    }
    pages = {n: u for n, u in candidates.items() if u}
    manquantes = [n for n, u in candidates.items() if not u]
    return pages, manquantes
