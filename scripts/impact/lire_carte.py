"""Le lecteur de `tests/impact-map.toml`, partagé par ses deux consommateurs :
`scripts/check-arch.sh` (axe 8) et `scripts/impact/select_tests.py`.

# Pourquoi pas `tomllib`

Les deux l'importaient. Il n'est entré dans la bibliothèque standard qu'en
**Python 3.11**, et le `python3` d'un macOS récent est en 3.9 : l'axe 8
n'affichait vert que parce qu'il ne démarrait pas, et `make e2e-impact`
n'exécutait plus aucun test (carte 480).

Un verrou qui ne tourne pas sur la machine de celui qui code ne garde rien. Le
sous-ensemble employé par la carte est minuscule — deux tables, des clefs entre
guillemets, des tableaux de chaînes, des commentaires `#` — et le lire à la main
coûte moins que la dépendance.

# Le lecteur est strict, et c'est ce qui le rend sûr

**Toute ligne incomprise lève une erreur, jamais un silence.** Une syntaxe qui
sortirait du sous-ensemble arrête ses appelants au lieu d'être sautée — sans
quoi on aurait remplacé un verrou muet par un autre.

Vérifié rendre exactement le même résultat que `tomllib` sur le fichier réel.
"""

import re


def lire_carte(texte):
    """Le contenu de la carte, sous la forme `{table: {clef: [valeurs]}}`."""
    tables, table, cle, valeurs = {}, None, None, None
    for n, brute in enumerate(texte.splitlines(), 1):
        ligne = re.sub(r"\s*#.*$", "", brute).strip()
        if not ligne:
            continue
        if cle is None:
            m = re.fullmatch(r"\[(\w+)\]", ligne)
            if m:
                table = m.group(1)
                tables.setdefault(table, {})
                continue
            m = re.fullmatch(r'"([^"]+)"\s*=\s*\[(.*)', ligne)
            if not m:
                raise ValueError(f"ligne {n} incomprise : {brute.strip()!r}")
            if table is None:
                raise ValueError(f"ligne {n} : clef hors de toute table")
            cle, reste, valeurs = m.group(1), m.group(2), []
        else:
            reste = ligne
        fin = reste.rstrip().endswith("]")
        for item in (reste.rstrip()[:-1] if fin else reste).split(","):
            item = item.strip()
            if not item:
                continue
            m = re.fullmatch(r'"([^"]*)"', item)
            if not m:
                raise ValueError(f"ligne {n} : valeur incomprise {item!r}")
            valeurs.append(m.group(1))
        if fin:
            tables[table][cle] = valeurs
            cle = None
    if cle is not None:
        raise ValueError("tableau non refermé en fin de fichier")
    return tables
