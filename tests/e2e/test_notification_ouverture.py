"""Le déclencheur d'ouverture des inscriptions (carte 340, R11).

C'est le test qui atteste que **la fonctionnalité est allumée**. Les neuf cartes
précédentes de l'épic E02 ont posé la table, le domaine, les gabarits et le cœur
d'expédition ; aucune ne faisait partir un e-mail.

Il lit la **table du journal**, pas une boîte de réception : la suite tourne en
`EMAIL__PROVIDER=console`, et un test qui dépendrait du fournisseur ne pourrait
pas s'exécuter en CI. Ce que la table prouve est exactement ce qui compte — une
ligne réservée par destinataire, et `sent_at` renseigné veut dire que l'envoi a
été confirmé.

L'ouverture ne passe pas par le cron : elle se déclenche sur un **fait**, la
saison qui s'ouvre, et non sur une date à comparer au jour. Attendre le cron du
lendemain ferait arriver l'annonce un jour trop tard.

Prérequis : serveur kreek lancé en dev.
"""

import time

from playwright.sync_api import Page

from competition_lifecycle import create_full_competition
from db_helpers import query_db


def _lignes_du_journal(season_id: str) -> list[str]:
    return query_db(
        "SELECT notification_type, coach_id, (sent_at IS NOT NULL)::text "
        f"FROM competition_notification_deliveries WHERE season_id = '{season_id}'"
    )


def _attendre_le_journal(season_id: str, minimum: int = 1) -> list[str]:
    """Le listener est détaché : la réponse HTTP ne l'attend pas, donc le test
    non plus ne peut pas se contenter d'un `goto` suivi d'une lecture."""
    for _ in range(50):
        lignes = _lignes_du_journal(season_id)
        if len(lignes) >= minimum:
            return lignes
        time.sleep(0.2)
    return _lignes_du_journal(season_id)


def test_publier_une_competition_annonce_l_ouverture_aux_membres(
    page: Page, competition_create_url
):
    comp = create_full_competition(
        page, competition_create_url, num_rounds=1, access_mode="open"
    )

    lignes = _attendre_le_journal(comp["season_id"])

    assert lignes, (
        "aucune ligne dans le journal d'envois : la publication n'a rien déclenché"
    )
    assert all(l.startswith("registration_open|") for l in lignes), (
        f"seule l'ouverture doit partir à la publication : {lignes}"
    )
    # `sent_at` renseigné : l'envoi a été confirmé, pas seulement réservé. Une
    # ligne restée à NULL serait l'échec constaté de R1.
    assert all(l.endswith("|true") for l in lignes), (
        f"des envois sont restés non confirmés : {lignes}"
    )


def test_republier_n_annonce_pas_une_seconde_fois(page: Page, space_id, competition_create_url):
    """R3 de bout en bout. La clé d'idempotence ne porte pas de journée pour
    l'ouverture — c'est le cas que l'index protège par `COALESCE(round_id, '')`,
    et celui qu'une contrainte `UNIQUE` ordinaire laisserait passer."""
    comp = create_full_competition(
        page, competition_create_url, num_rounds=1, access_mode="open"
    )
    avant = _attendre_le_journal(comp["season_id"])
    assert avant, "sans première annonce, ce test ne prouverait rien"

    # Repasser par l'étape 5 et republier.
    page.goto(
        f"http://localhost:3210/app/{space_id}/competitions/create/"
        f"{comp['competition_id']}/{comp['season_id']}/validation",
        wait_until="load",
    )
    page.click(".btn-cta")
    page.wait_for_timeout(2000)

    apres = _lignes_du_journal(comp["season_id"])
    assert len(apres) == len(avant), (
        f"la republication a dupliqué des envois : {len(avant)} → {len(apres)}"
    )
