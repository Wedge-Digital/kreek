#!/usr/bin/env python3
"""Les méthodes du dépôt de saisons dont le SQL **pose le statut**.

Imprime un nom par ligne, trié. Sert l'axe 16 de `check-arch.sh`, qui interdit
aux panneaux de réglages d'appeler ces méthodes-là.

**Pourquoi le déduire plutôt que le lister.** L'axe pourrait porter les quatre
noms en dur. Une liste écrite à la main dérive : c'est exactement le reproche
que l'axe fait aux assertions recopiées panneau par panneau, qui n'ont pas
empêché le troisième cas d'arriver (carte 485). Ici la source de vérité est le
SQL lui-même — ajouter une méthode qui pose un statut la fait apparaître sans
que personne y pense.

**Le piège du commentaire.** Les fichiers `*_keep_status.sql` *expliquent* dans
leur en-tête qu'ils ne posent pas de statut, contrairement à leur jumeau — donc
la chaîne `status = '…'` figure dans leur commentaire. Un `grep` naïf les
accuse tous les deux ; c'est arrivé en écrivant cet axe. Les commentaires sont
retirés d'abord.

**Et celui du WHERE.** Un filtre `WHERE status = 'draft'` n'écrit rien. Seule
la portion entre `SET` et `WHERE` est examinée.
"""

import pathlib
import re
import sys

RACINE = pathlib.Path(__file__).resolve().parents[2]
DEPOT = RACINE / "src/app/competitions/io/repository/season_repository.rs"
SQL = RACINE / "src/app/competitions/io/repository"

METHODE = re.compile(r"async fn ((?:save_|set_)[a-z_]+)")
INCLUDE = re.compile(r'include_str!\("(sql/seasons/[a-z_]+\.sql)"\)')


def pose_un_statut(chemin: pathlib.Path) -> bool:
    sql = re.sub(r"--[^\n]*", "", chemin.read_text())
    entre = re.search(r"\bSET\b(.*?)\bWHERE\b", sql, re.S | re.I)
    return bool(entre and re.search(r"\bstatus\s*=", entre.group(1), re.I))


def main() -> int:
    courante, trouvees = None, []
    for ligne in DEPOT.read_text().splitlines():
        if m := METHODE.search(ligne):
            courante = m.group(1)
        elif courante and (m := INCLUDE.search(ligne)):
            if pose_un_statut(SQL / m.group(1)):
                trouvees.append(courante)
            courante = None
    if not trouvees:
        print("aucune méthode trouvée — l'analyse a échoué", file=sys.stderr)
        return 1
    print("\n".join(sorted(trouvees)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
