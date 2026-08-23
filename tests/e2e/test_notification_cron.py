"""Le déclencheur périodique (carte 367, épic E02).

C'est le test du **critère de clôture de l'épic** :

    Un coach inscrit à une compétition reçoit, la veille d'une journée, un
    e-mail listant ses matchs — sans qu'une seconde exécution du cron le même
    jour lui en envoie un second.

Les tests unitaires de `send_due_notifications_use_case` couvrent la couture
entre le SQL de sélection et `due_today()`, en doublant les deux ports inter-BC
(`teams`, `members`). **Ce que ces doublures cachent est exactement ce que ce
fichier vérifie** : que la CLI câble pour de vrai les adapters correspondants.
Un port mal branché dans `src/cli/send_notifications.rs` passerait tous les
tests unitaires.

Comme `test_notification_ouverture.py`, il lit la **table du journal** et non une
boîte de réception : la suite tourne en `EMAIL__PROVIDER=console`, et un test qui
dépendrait du fournisseur ne pourrait pas s'exécuter en CI.

Il faut des équipes **inscrites** : la veille de journée ne s'adresse qu'à elles,
là où l'annonce d'ouverture part à tous les membres de l'espace. Une compétition
publiée mais vide ne déclencherait donc rien, et le test passerait à vide.

Prérequis : serveur kreek lancé en dev, et une chaîne de compilation utilisable
— le test invoque le binaire.
"""

import subprocess
from datetime import date, timedelta
from pathlib import Path

import pytest

from competition_lifecycle import build_full_competition
from db_helpers import execute_db, query_db

RACINE = Path(__file__).resolve().parents[2]

# `due_today()` compare `date_start` à `today + 1` pour la veille de journée.
# Le test choisit donc les deux dates ensemble, plutôt que d'attendre demain.
AUJOURDHUI = date.today()
DEMAIN = AUJOURDHUI + timedelta(days=1)


def _lancer_le_cron(*extra: str) -> subprocess.CompletedProcess:
    """`--date` vise le jour choisi : sans lui, le test dépendrait de l'heure à
    laquelle il tourne, et échouerait au passage de minuit."""
    return subprocess.run(
        [
            "cargo", "run", "--quiet", "--",
            "send-notifications", "--date", AUJOURDHUI.isoformat(), *extra,
        ],
        cwd=RACINE,
        capture_output=True,
        text=True,
        timeout=600,
    )


def _journal(season_id: str) -> list[str]:
    return query_db(
        "SELECT notification_type, (sent_at IS NOT NULL)::text "
        f"FROM competition_notification_deliveries WHERE season_id = '{season_id}'"
    )


def _programmer_une_journee_demain(season_id: str) -> None:
    """L'état que le parcours utilisateur ne permet pas d'atteindre au jour près.

    Le magicien pose bien des dates, mais les faire tomber exactement sur demain
    depuis un test rendrait celui-ci dépendant de son propre calendrier. On
    déclare l'écriture directe plutôt que de la glisser dans une lecture — cf.
    `execute_db`.
    """
    execute_db(
        "UPDATE competition_match_days "
        f"SET date_start = '{DEMAIN.isoformat()}', date_end = NULL, "
        "    day_type = 'fixed_date' "
        f"WHERE season_id = '{season_id}'"
    )


@pytest.fixture(scope="module")
def saison_avec_journee_demain(browser, space_id) -> str:
    """Deux équipes inscrites, une journée pairée, reprogrammée à demain.

    Portée module : la construction coûte deux soumissions d'équipe et une
    génération de calendrier. Chaque test vide le journal avant de s'exécuter,
    ce qui les rend indépendants sans reconstruire la compétition.
    """
    full = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    _programmer_une_journee_demain(full["season_id"])
    return full["season_id"]


@pytest.fixture(autouse=True)
def _journal_vide(saison_avec_journee_demain):
    """L'annonce d'ouverture est partie à la publication ; on ne veut observer
    que ce que le cron ajoute — et chaque test doit repartir d'un journal vide,
    l'idempotence étant précisément ce qu'on mesure."""
    execute_db(
        "DELETE FROM competition_notification_deliveries "
        f"WHERE season_id = '{saison_avec_journee_demain}'"
    )


def test_la_veille_d_une_journee_le_cron_previent_les_inscrits(
    saison_avec_journee_demain,
):
    r = _lancer_le_cron()
    assert r.returncode == 0, f"le cron a échoué :\n{r.stderr}"

    lignes = _journal(saison_avec_journee_demain)
    assert lignes, (
        "aucune ligne dans le journal : la veille de journée n'a rien déclenché.\n"
        f"sortie du cron :\n{r.stdout}\n{r.stderr}"
    )
    assert all(l.startswith("round_eve|") for l in lignes), (
        f"seule la veille de journée doit partir : {lignes}"
    )
    # `sent_at` renseigné : l'envoi a été confirmé, pas seulement réservé. Une
    # ligne restée à NULL serait l'échec constaté de R1.
    assert all(l.endswith("|true") for l in lignes), (
        f"des envois sont restés non confirmés : {lignes}"
    )


def test_une_seconde_execution_le_meme_jour_n_envoie_pas_un_second_mail(
    saison_avec_journee_demain,
):
    """La seconde moitié du critère de l'épic. C'est l'index unique du journal
    qui la tient — `COALESCE(round_id, '')` compris — et non une garde
    applicative qu'une refonte pourrait retirer sans que rien ne proteste."""
    assert _lancer_le_cron().returncode == 0
    avant = _journal(saison_avec_journee_demain)
    assert avant, "sans premier envoi, ce test ne prouverait rien"

    r = _lancer_le_cron()
    assert r.returncode == 0, f"la seconde exécution a échoué :\n{r.stderr}"

    apres = _journal(saison_avec_journee_demain)
    assert len(apres) == len(avant), (
        f"la seconde exécution a dupliqué des envois : {len(avant)} → {len(apres)}"
    )


def test_dry_run_ne_reserve_rien(saison_avec_journee_demain):
    """La commande qu'on lancera **en premier** sur la production. Réserver sans
    expédier laisserait des lignes qui bloqueraient le vrai passage, et R9
    interdit de les rejouer le lendemain."""
    r = _lancer_le_cron("--dry-run")
    assert r.returncode == 0, f"le dry-run a échoué :\n{r.stderr}"

    assert not _journal(saison_avec_journee_demain), (
        "le dry-run a écrit dans le journal — il est censé ne rien réserver"
    )

    # **Sans ce second passage le test serait vide** : un journal resté vide
    # parce que rien n'était dû ressemblerait trait pour trait à un dry-run
    # correct. Le même état, exécuté pour de vrai, doit écrire — et c'est ce qui
    # établit qu'il y avait bien quelque chose à ne pas réserver.
    assert _lancer_le_cron().returncode == 0
    assert _journal(saison_avec_journee_demain), (
        "rien n'était dû de toute façon : le dry-run n'était donc pas éprouvé"
    )
