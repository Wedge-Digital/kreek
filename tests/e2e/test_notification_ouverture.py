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
    non plus ne peut pas se contenter d'un `goto` suivi d'une lecture.

    **L'attente porte sur la confirmation, pas sur la seule présence.** Un envoi
    s'écrit en deux temps — la ligne est réservée (`claimed_at`), puis confirmée
    (`sent_at`) — et une saison en produit une par membre de l'espace. Rendre la
    main dès la première ligne laissait lire le journal **à mi-écriture** : douze
    lignes, onze confirmées, une encore réservée. L'appelant affirmait ensuite
    qu'elles l'étaient toutes, et échouait sur une lecture prise au vol.

    Constaté une fois sur vingt-trois courses. La ligne incriminée était
    confirmée quelques instants plus tard, et le journal ne comptait alors plus
    aucune ligne en attente : le produit avait fait son travail.

    **Et le compte doit être stable.** « Toutes confirmées » est satisfait par un
    sous-ensemble : trois lignes écrites et confirmées sur douze rendraient la
    main aussitôt, et l'appelant compterait trois envois là où il y en a douze.
    L'attente exige donc que le nombre ne bouge plus pendant trois relevés
    consécutifs — soit six dixièmes de seconde sans écriture nouvelle.

    Ce n'est pas un assouplissement — si un envoi restait réellement non
    confirmé, la boucle expire et l'assertion échoue comme avant, mais après dix
    secondes d'attente réelle plutôt que sur un instantané. Vérifié en insérant
    une ligne jamais confirmée : la boucle expire, et l'assertion tombe.
    """
    stables = 0
    precedent = -1
    for _ in range(50):
        lignes = _lignes_du_journal(season_id)
        complet = len(lignes) >= minimum and all(l.endswith("|true") for l in lignes)
        stables = stables + 1 if len(lignes) == precedent else 0
        precedent = len(lignes)
        if complet and stables >= 3:
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
