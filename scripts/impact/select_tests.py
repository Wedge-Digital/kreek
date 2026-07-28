#!/usr/bin/env python3
"""Étape 2+3 du skill test-impact : jetons -> tests e2e à exécuter.

Lit les jetons de `changed_bcs.sh` sur stdin (ou les prend en arguments),
applique les règles d'amplification, croise avec `tests/impact-map.toml`.

Sortie :
  - le rapport au format défini dans `.claude/skills/test-impact/SKILL.md`,
    sur stderr ;
  - la liste des fichiers de test à passer à pytest, un par ligne, sur stdout
    (vide si run-all, cf. code de sortie).

Codes de sortie :
  0  sélection partielle (stdout contient les tests retenus), ou diff sans
     aucun BC touché (rien à lancer, légitimement)
  10 run all déclenché — l'appelant doit lancer la suite complète
  11 des BCs sont touchés mais aucun test ne les couvre. Ce n'est pas un
     succès : c'est un trou de couverture e2e, l'appelant doit le dire.

Ce programme ne remplace pas le skill : il en fige la partie mécanique. Le
jugement (grep d'un type d'événement, diagnostic de drift, rédaction d'une
entrée manquante) reste au skill. Les deux doivent donner le même résultat ;
une divergence est un bug de l'un des deux.
"""
import pathlib
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[2]
MAP = ROOT / "tests" / "impact-map.toml"
E2E = ROOT / "tests" / "e2e"

RUN_ALL_TOKENS = {
    "@shared_kernel": "types et événements partagés",
    "@migrations": "le schéma est un contrat global",
    "@config": "la config traverse les BCs",
    "@build": "l'environnement d'exécution change",
    "@core": "plomberie transverse (layout, middleware, routeur, bus, seed)",
    "@assets": "CSS/JS globaux",
    "@e2e_harness": "toute la suite dépend de ces helpers",
}


def load_map():
    data = tomllib.loads(MAP.read_text())
    return data.get("tests", {}), data.get("deps", {})


def amplify(tokens, deps):
    """Jetons -> (BCs impactés, tests forcés, raison de run-all)."""
    bcs, forced = set(), set()
    for tok in tokens:
        if tok in RUN_ALL_TOKENS:
            return None, None, f"{tok} ({RUN_ALL_TOKENS[tok]})"
        if tok.startswith("@unknown:"):
            return None, None, f"{tok} (fichier non résolu : aucune garantie)"
        if tok.startswith("@contract:"):
            bc = tok.split(":", 1)[1]
            bcs.add(bc)
            # Seul cas où [deps] s'applique : le contrat est ce que les autres
            # BCs voient de celui-ci.
            bcs.update(deps.get(bc, []))
        elif tok.startswith("@templates:"):
            bcs.add(tok.split(":", 1)[1])
        elif tok.startswith("@test:"):
            forced.add(tok.split(":", 1)[1])
        elif not tok.startswith("@"):
            bcs.add(tok)
    return bcs, forced, None


def select(bcs, forced, tests):
    """Un test sans entrée dans la carte est traité comme "all"."""
    on_disk = {p.stem for p in E2E.glob("test_*.py")}
    unmapped = on_disk - set(tests)
    retained = set(forced) | unmapped
    for name, declared in tests.items():
        if "all" in declared or bcs & set(declared):
            retained.add(name)
    return sorted(retained & on_disk), sorted(unmapped)


def main():
    tokens = sys.argv[1:] or sys.stdin.read().split()
    tests, deps = load_map()
    bcs, forced, run_all = amplify(tokens, deps)

    print(f"Jetons          : {', '.join(tokens) or '(aucun)'}", file=sys.stderr)

    if run_all:
        print(f"Amplification   : RUN ALL déclenché par {run_all}", file=sys.stderr)
        print(f"Tests exécutés  : {len(tests)}/{len(tests)} — suite complète", file=sys.stderr)
        print("Rappel          : la CI exécutera la suite complète.", file=sys.stderr)
        return 10

    retained, unmapped = select(bcs, forced, tests)
    print(f"Amplification   : {', '.join(sorted(bcs)) or '(aucun BC)'}", file=sys.stderr)
    if unmapped:
        print(f"⚠ Hors carte    : {', '.join(unmapped)} — traité(s) comme \"all\", "
              f"à déclarer dans tests/impact-map.toml", file=sys.stderr)
    print(f"Tests exécutés  : {len(retained)}/{len(tests)} — {', '.join(retained) or '—'}",
          file=sys.stderr)
    print(f"Tests ignorés   : {len(tests) - len(retained)}", file=sys.stderr)
    print("Rappel          : la CI exécutera la suite complète.", file=sys.stderr)

    if not retained:
        if not bcs:
            # Diff entièrement neutre (docs, kanban, outillage) : il n'y a
            # légitimement rien à lancer.
            print("Aucun BC touché — rien à lancer.", file=sys.stderr)
            return 0
        print(f"⚠ Aucun test ne couvre {', '.join(sorted(bcs))} — ce n'est pas un "
              "succès : ce(s) BC(s) n'ont pas de couverture e2e.", file=sys.stderr)
        return 11

    for name in retained:
        print(f"{name}.py")
    return 0


if __name__ == "__main__":
    sys.exit(main())
