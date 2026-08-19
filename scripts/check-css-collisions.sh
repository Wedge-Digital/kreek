#!/usr/bin/env bash
# Vérifie la règle de portée du CSS (cf. kanban/341, épic E03).
#
# ── La règle ────────────────────────────────────────────────────────────────
#
#   Toute règle d'une feuille de `pages/` ou `widgets/` est scopée sous une
#   classe qui porte le nom du fichier.
#
#       pages/team-page.css   ⇒   .team-page .team-header-logo { … }
#       widgets/dismissals.css ⇒  .dismissals .cart-line { … }
#
# Le nom du fichier **est** le sélecteur de portée. Il n'y a donc aucune table
# de correspondance à tenir, et rien qui puisse diverger : la règle se vérifie
# en comparant un nom de fichier à un préfixe de sélecteur.
#
# `components/`, `common.css` et `layout-app.css` restent **globaux** : ils sont
# partagés par conception, et les scoper n'aurait pas de sens.
#
# Une feuille de `pages/` peut l'être aussi — `match-report-shared.css` est
# chargée par dix templates. Elle le déclare **dans son en-tête**, par un
# commentaire `css:global` portant son motif, plutôt que dans une liste tenue
# ici. Une liste dérive ; un marqueur adjacent au fichier ne le peut pas.
#
# ── Pourquoi ─────────────────────────────────────────────────────────────────
#
# Les feuilles ont été écrites en isolation totale — chaque page charge la
# sienne via un `<link>` dans son fragment, et deux feuilles de page ne
# cohabitent jamais. Cette isolation a autorisé, sans que rien ne le signale, la
# réutilisation des mêmes noms de classe avec des valeurs différentes.
#
# Tant que les feuilles restent isolées, ces divergences sont invisibles. Dès
# qu'on les réunit dans un fichier unique — ce que fait la carte 342 pour
# supprimer le clignotement — la dernière chargée gagne, **pour toute
# l'application**. Le bundle ne neutralise pas les collisions : il les active.
#
# ── Ce que ce script vérifie, et pourquoi deux contrôles ────────────────────
#
# Le contrôle A (portée) est la règle durable ; le contrôle B (collisions) est
# l'objectif. A implique B pour les feuilles de page et de widget — deux
# feuilles scopées sous des noms distincts ne peuvent plus se rencontrer. B
# reste utile pour ce qui demeure global, où seule la vigilance protège.
#
# Vérifier A seul laisserait passer une collision entre deux fichiers de
# `components/` ; vérifier B seul laisserait passer la 63e feuille écrite sans
# portée, qui ne collisionne avec rien **aujourd'hui**.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BOLD=$'\033[1m'; RESET=$'\033[0m'
GREEN=$'\033[32m'; RED=$'\033[31m'; YELLOW=$'\033[33m'

echo ""
echo "${BOLD}${YELLOW}┌─ Portée du CSS (cf. kanban/341)${RESET}"
echo ""

python3 - "$@" <<'PYTHON'
import re
import sys
import pathlib
from collections import defaultdict

RACINE = pathlib.Path('assets/static/css')
SCOPES = ('pages', 'widgets')          # doivent être scopés
GLOBAUX = ('components',)              # partagés par conception

BOLD = '\033[1m'; RESET = '\033[0m'
GREEN = '\033[32m'; RED = '\033[31m'


def regles(chemin):
    """Rend (sélecteur, déclarations) pour chaque règle du fichier.

    Les étapes de `@keyframes` (`from`, `to`, `50%`) sont écartées : ce sont des
    positions dans une animation, pas des sélecteurs, et elles ne peuvent pas
    être scopées.
    """
    texte = re.sub(r'/\*.*?\*/', '', chemin.read_text(), flags=re.S)
    sortie = []
    pile = []          # at-rules englobantes
    prelude = ''
    i = 0
    while i < len(texte):
        c = texte[i]
        if c == '{':
            tete = prelude.strip()
            if tete.startswith('@'):
                pile.append(tete.split()[0].lower())
                sortie.append(None)          # marque : bloc at-rule ouvert
            else:
                pile.append(None)
                if not any(p == '@keyframes' for p in pile if p):
                    corps = texte[i + 1:texte.find('}', i)]
                    decls = ' '.join(sorted(
                        d.strip() for d in corps.split(';') if d.strip()))
                    for sel in tete.split(','):
                        sel = ' '.join(sel.split())
                        if sel:
                            sortie.append((sel, decls))
            prelude = ''
        elif c == '}':
            if pile:
                pile.pop()
            prelude = ''
        elif c == ';':
            prelude = ''
        else:
            prelude += c
        i += 1
    return [r for r in sortie if r]


def porte_le_scope(selecteur, scope):
    if selecteur == scope:
        return True
    return selecteur.startswith(scope) and not (
        selecteur[len(scope)].isalnum() or selecteur[len(scope)] in '-_')


# ── Contrôle A : portée ─────────────────────────────────────────────────────
def est_declaree_globale(chemin):
    """Un fichier de `pages/` ou `widgets/` peut être partagé par conception.
    Il le dit dans son en-tête, avec son motif."""
    return 'css:global' in chemin.read_text()[:800]


a_scoper = sorted(f for d in SCOPES for f in (RACINE / d).rglob('*.css')
                  if not est_declaree_globale(f))
globales_declarees = sorted(f for d in SCOPES for f in (RACINE / d).rglob('*.css')
                            if est_declaree_globale(f))
fautifs = {}
conformes = 0
vides = []
for f in a_scoper:
    regles_du_fichier = regles(f)
    # Une feuille sans règle n'est pas « conforme », elle est vide. La compter
    # avec les autres ferait progresser le compteur sans qu'aucun travail
    # n'ait eu lieu — un compteur qui ment est pire qu'aucun compteur.
    if not regles_du_fichier:
        vides.append(f)
        continue
    scope = '.' + f.stem
    manquants = [s for s, _ in regles_du_fichier if not porte_le_scope(s, scope)]
    if manquants:
        fautifs[f] = manquants
    else:
        conformes += 1

print(f"{BOLD}Contrôle A · Portée — chaque feuille sous la classe de son nom{RESET}")
print(f"  {conformes}/{len(a_scoper) - len(vides)} feuilles conformes")
if vides:
    print(f"  · {len(vides)} feuilles sans aucune règle, non comptées : "
          + ', '.join(f.stem for f in vides))
for f in globales_declarees:
    print(f"  · {f.as_posix()} — déclarée globale (css:global), hors périmètre")
if fautifs:
    print(f"  {RED}✗ FAIL{RESET}  {len(fautifs)} feuilles portent des règles hors de leur portée")
    for f in sorted(fautifs, key=lambda x: -len(fautifs[x]))[:10]:
        exemples = ', '.join(fautifs[f][:3])
        print(f"       {f.as_posix()} — {len(fautifs[f])} règles (ex. {exemples})")
    if len(fautifs) > 10:
        print(f"       … et {len(fautifs) - 10} autres feuilles")
else:
    print(f"  {GREEN}✓ PASS{RESET}")
print()

# ── Contrôle B : collisions divergentes ─────────────────────────────────────
par_selecteur = defaultdict(dict)
for f in sorted(RACINE.rglob('*.css')):
    for sel, decls in regles(f):
        par_selecteur[sel][f.as_posix()] = decls

divergents = {s: v for s, v in par_selecteur.items()
              if len(v) > 1 and len(set(v.values())) > 1}

print(f"{BOLD}Contrôle B · Collisions — aucun sélecteur divergent entre feuilles{RESET}")
if divergents:
    print(f"  {RED}✗ FAIL{RESET}  {len(divergents)} sélecteurs définis différemment dans plusieurs feuilles")
    for s, v in sorted(divergents.items(), key=lambda kv: -len(kv[1]))[:10]:
        print(f"       {len(v)}×  {s}")
    if len(divergents) > 10:
        print(f"       … et {len(divergents) - 10} autres sélecteurs")
else:
    print(f"  {GREEN}✓ PASS{RESET}")
print()

sys.exit(1 if (fautifs or divergents) else 0)
PYTHON
CODE=$?

if [ "$CODE" -eq 0 ]; then
    echo "${GREEN}${BOLD}✓ La portée du CSS est tenue${RESET}"
else
    echo "${RED}${BOLD}✗ La portée du CSS n'est pas tenue${RESET}"
fi
echo ""
exit "$CODE"
