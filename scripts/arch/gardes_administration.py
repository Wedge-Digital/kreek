#!/usr/bin/env python3
"""Axe 19 — tout handler d'administration **routé** vérifie le droit d'accès.

# Ce que cet axe remplace

La carte 543 a trouvé quatre fragments de lecture sans garde —  la barre latérale
du calendrier, le détail d'une journée, les cartes de poules, le vivier des non
affectées. La carte 416 avait fermé le même trou sur treize routes de **mutation**
deux mois plus tôt ; son périmètre était les écritures, et rien depuis n'avait
regardé les lectures.

Sa conclusion disait déjà : *« ces tests sont le seul filet. Rien dans le
compilateur ne signale un handler qui ne contrôle rien. »* Cet axe est ce filet,
côté statique.

# Pourquoi la commande d'audit de la carte ne suffisait pas

Elle cherchait `require_admin_access` dans le corps de chaque `pub async fn` d'un
fichier d'administration. Écrite en carte 519, elle rendait quatre lignes ; rejouée
en carte 543, elle en rend **vingt-deux dont quatre vraies**, et le bruit vient de
deux sources :

  - des fonctions qui ne sont **pas routées** — les résolveurs d'`admin_scope`,
    `build_summary_fragment`, `charger_le_panneau` — donc atteignables seulement
    depuis un handler déjà gardé ;
  - des handlers gardés **par une aide** : `admin_page` passe par
    `render_admin_page`, et les dix actions de présence par `contexte()`, qui
    appelle la garde en première ligne.

Une vérification qui rend dix-huit faux positifs n'est plus lue. Ce script
croise donc avec le routeur, et connaît les aides gardiennes.

# Les aides gardiennes sont **déclarées**, pas devinées

`GARDIENNES` ci-dessous. Une aide qui apparaît dans le corps d'un handler vaut
garde, parce qu'elle appelle elle-même `require_admin_access` en première ligne.
C'est une liste à tenir : y ajouter une fonction qui ne garde rien ouvrirait le
trou que cet axe surveille — d'où le contrôle qui suit, qui vérifie que chaque
aide déclarée appelle bien la garde.

Le même choix que `EXTRACTABLE_BCS` en tête de `check-arch.sh` : une liste courte,
visible en revue, plutôt qu'un marqueur qui s'essaime.
"""

import re
import sys
from pathlib import Path

RACINE = Path(__file__).resolve().parents[2]
ROUTEUR = RACINE / "src/app/competitions/router.rs"
DOSSIERS = [
    RACINE / "src/app/competitions/io/web/admin",
    RACINE / "src/app/competitions/io/web/admin/settings",
]

GARDE = "require_admin_access"

# Aides qui appellent `require_admin_access` et valent donc garde. Chacune est
# vérifiée plus bas : une aide déclarée qui cesserait de garder fait échouer
# l'axe, plutôt que d'ouvrir silencieusement le trou.
GARDIENNES = {
    "render_admin_page": "enveloppe la page d'administration et ses onglets",
    "contexte": "résout le contexte des dix actions de présence",
}

METHODES = ("get", "post", "put", "delete", "patch")


def handlers_routes() -> set[str]:
    """Les noms de handlers que le routeur expose, toutes méthodes confondues."""
    texte = ROUTEUR.read_text()
    motif = re.compile(r"\b(?:%s)\(\s*([a-z_][a-z0-9_]*)\s*\)" % "|".join(METHODES))
    return set(motif.findall(texte))


def corps_des_fonctions(fichier: Path) -> dict[str, str]:
    """Le corps de chaque `pub async fn` du fichier, jusqu'à son accolade de
    fermeture en colonne zéro — la même heuristique que l'awk d'origine, qui
    suffit pour du code passé par `cargo fmt`."""
    corps, nom, accumule = {}, None, []
    for ligne in fichier.read_text().splitlines():
        if ligne.startswith("pub async fn ") or ligne.startswith("async fn "):
            nom = ligne.split("fn ", 1)[1].split("(")[0].split("<")[0].strip()
            accumule = []
        if nom is not None:
            accumule.append(ligne)
            if ligne == "}":
                corps[nom] = "\n".join(accumule)
                nom = None
    return corps


def main() -> int:
    routes = handlers_routes()
    fautifs, aides_muettes = [], []
    connues: dict[str, str] = {}

    for dossier in DOSSIERS:
        if not dossier.is_dir():
            continue
        for fichier in sorted(dossier.glob("*.rs")):
            for nom, corps in corps_des_fonctions(fichier).items():
                connues.setdefault(nom, corps)
                if nom not in routes:
                    continue
                if GARDE in corps:
                    continue
                if any(f"{aide}(" in corps for aide in GARDIENNES):
                    continue
                fautifs.append(f"{fichier.name}::{nom}")

    # Une aide déclarée gardienne qui ne garde plus rendrait l'axe complice.
    for aide, motif in GARDIENNES.items():
        corps = connues.get(aide)
        if corps is None:
            aides_muettes.append(f"{aide} — déclarée gardienne, introuvable ({motif})")
        elif GARDE not in corps:
            aides_muettes.append(f"{aide} — déclarée gardienne, n'appelle plus {GARDE}")

    for ligne in fautifs:
        print(f"    {ligne}  ← handler d'administration routé sans contrôle d'accès")
    for ligne in aides_muettes:
        print(f"    {ligne}")

    return 1 if (fautifs or aides_muettes) else 0


if __name__ == "__main__":
    sys.exit(main())
