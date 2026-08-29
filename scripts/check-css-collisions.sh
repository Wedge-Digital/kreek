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
# ── Ce qui est hors périmètre, et déduit plutôt que listé ────────────────────
#
# Une feuille que **aucun template de `src/` ne charge** n'est pas dans
# l'application : soit elle ne sert qu'aux maquettes de `assets/templates/`,
# soit elle ne sert à rien. La scoper ne protégerait rien, et la carte 342
# l'exclut de toute façon du bundle.
#
# Le script le déduit du dépôt, il ne le tient pas en liste — mais il les
# **compte et les nomme**, pour qu'une feuille sortie du périmètre reste
# visible plutôt que d'être oubliée.
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

# Le contrôle C (proximité des tokens) répond à un défaut d'une autre nature :
# `--dark-6` (#F0F2F4) et `--dark-7` (#EFF2F5) ont coexisté pendant des mois à
# **1,0012 de rapport de contraste** — la même couleur écrite deux fois. Quatre
# feuilles les opposaient pour distinguer un zébrage d'un survol, et le survol
# des deux classements était donc invisible une ligne sur deux, en production,
# sous les yeux de tous les coachs (carte 448).
#
# Le contrôle porte sur les **valeurs** des tokens, pas sur leurs usages : il
# attrape la cause plutôt que ses effets, et une seule fois plutôt qu'en chaque
# endroit. Un token indistinguable d'un autre n'a aucun usage légitime — s'il en
# avait un, il serait un alias, et un alias s'écrit `--x: var(--y)`.
#
# Le seuil de 1,05 n'est pas une norme d'accessibilité : c'est le plancher en
# deçà duquel deux fonds ne se distinguent pas à l'œil sur un écran ordinaire.
# `--dark-5`/`--dark-6` valent 1,1074 et se voient ; 1,0012 ne se voit pas.

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
# Le périmètre se lisait dans les `<link>` des templates. La carte 342 les a
# tous supprimés — les feuilles sont désormais réunies en un fichier unique — et
# le verrou s'est retrouvé à ne surveiller qu'une feuille sur quarante-six, sans
# rien signaler.
#
# La source de vérité est donc la **liste du bundle**, dans
# `src/web/css_bundle.rs`, augmentée des feuilles que le BC `auth` charge encore
# lui-même. Une feuille absente des deux n'est pas dans l'application.
_MOD = pathlib.Path('src/web/css_bundle.rs').read_text()
_LISTE = _MOD.split('FEUILLES_APP: &[&str] = &[', 1)[1].split('];', 1)[0]
TEMPLATES_SRC = {m.group(1).rsplit('/', 1)[-1]
                 for m in re.finditer(r'"([^"]+\.css)"', _LISTE)}
for _t in pathlib.Path('src/app/auth').rglob('*.html'):
    for _m in re.finditer(r'href="/static/css/([^"]+\.css)"', _t.read_text()):
        TEMPLATES_SRC.add(_m.group(1).rsplit('/', 1)[-1])
SCOPES = ('pages', 'widgets')          # doivent être scopés
GLOBAUX = ('components',)              # partagés par conception
AMONT = ('vendor',)                    # feuilles tierces, non réécrites

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
def chargee_par_l_application(chemin):
    """Vrai si un template de `src/` pose un `<link>` vers cette feuille.

    Les maquettes de `assets/templates/` ne comptent pas : elles ne sont pas
    l'application, et la carte 342 les exclut du bundle.
    """
    return chemin.name in TEMPLATES_SRC


def est_declaree_globale(chemin):
    """Un fichier de `pages/` ou `widgets/` peut être partagé par conception.
    Il le dit dans son en-tête, avec son motif."""
    return 'css:global' in chemin.read_text()[:800]


toutes = sorted(f for d in SCOPES for f in (RACINE / d).rglob('*.css'))
hors_application = [f for f in toutes if not chargee_par_l_application(f)]
globales_declarees = [f for f in toutes
                      if chargee_par_l_application(f) and est_declaree_globale(f)]
a_scoper = [f for f in toutes
            if chargee_par_l_application(f) and not est_declaree_globale(f)]
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
if hors_application:
    print(f"  · {len(hors_application)} feuilles qu'aucun template de `src/` ne charge, "
          "hors périmètre : " + ', '.join(f.stem for f in hors_application))
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
# Même périmètre que le contrôle A : une feuille qu'aucun template de `src/`
# ne charge n'est pas dans l'application, la carte 342 l'exclut du bundle, et
# une collision avec elle est **inerte**. La compter ferait rougir le verrou sur
# du code mort, ce qui est le meilleur moyen qu'on cesse de le regarder.
#
# Les feuilles de `components/` et les globales de la racine, elles, comptent :
# elles sont bien dans l'application, et c'est entre elles que se joue ce qui
# reste.
#
# `vendor/` en est écarté, et c'est d'une autre nature que les exclusions
# ci-dessus : une feuille amont **n'est pas réécrivable**. La scoper serait la
# réécrire, et la prochaine montée de version écraserait le travail sans que
# rien ne le signale. Ses collisions avec nos surcharges ne sont d'ailleurs pas
# des divergences accidentelles : `components/tom-select.css` existe
# précisément pour redéfinir ce que `vendor/tom-select.min.css` pose.
#
# Le hors-périmètre est porté par le **dossier** et non par un marqueur
# `css:global` en tête de fichier : une feuille minifiée sur une ligne n'a pas
# d'en-tête commode, et lui en ajouter un reviendrait à modifier le fichier
# qu'on veut pouvoir remplacer à l'identique. Un dossier reste une information
# adjacente au fichier, pas une liste tenue ailleurs.
def dans_l_application(chemin):
    if chemin.parent.name in AMONT:
        return False
    return (chemin.parent.name not in SCOPES) or chargee_par_l_application(chemin)


par_selecteur = defaultdict(dict)
amont = []
for f in sorted(RACINE.rglob('*.css')):
    if f.parent.name in AMONT:
        amont.append(f)
        continue
    if not dans_l_application(f):
        continue
    for sel, decls in regles(f):
        par_selecteur[sel][f.as_posix()] = decls

divergents = {s: v for s, v in par_selecteur.items()
              if len(v) > 1 and len(set(v.values())) > 1}

print(f"{BOLD}Contrôle B · Collisions — aucun sélecteur divergent entre feuilles{RESET}")
# Nommées et non tues : une exclusion muette est une tolérance qui s'installe.
for f in amont:
    print(f"  · {f.as_posix()} — feuille amont, hors périmètre (non réécrivable)")
if divergents:
    print(f"  {RED}✗ FAIL{RESET}  {len(divergents)} sélecteurs définis différemment dans plusieurs feuilles")
    for s, v in sorted(divergents.items(), key=lambda kv: -len(kv[1]))[:10]:
        print(f"       {len(v)}×  {s}")
    if len(divergents) > 10:
        print(f"       … et {len(divergents) - 10} autres sélecteurs")
else:
    print(f"  {GREEN}✓ PASS{RESET}")
print()

# ── Contrôle C : proximité des tokens de couleur ────────────────────────────

import re as _re


def _luminance(hexa):
    """Luminance relative WCAG."""
    hexa = hexa.lstrip("#")
    canaux = []
    for i in (0, 2, 4):
        c = int(hexa[i:i + 2], 16) / 255
        canaux.append(c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4)
    r, v, b = canaux
    return 0.2126 * r + 0.7152 * v + 0.0722 * b


def _contraste(a, b):
    la, lb = _luminance(a), _luminance(b)
    haut, bas = max(la, lb), min(la, lb)
    return (haut + 0.05) / (bas + 0.05)


SEUIL = 1.05

_common = pathlib.Path("assets/static/css/common.css")
_tokens = _re.findall(r"(--[\w-]+):\s*(#[0-9A-Fa-f]{6})\s*;", _common.read_text())

# Les alias déclarés (`--x: var(--y)`) ne sont pas concernés : ils ne prétendent
# pas être une autre couleur. Seules les valeurs littérales entrent ici.
trop_proches = []
for i, (nom_a, val_a) in enumerate(_tokens):
    for nom_b, val_b in _tokens[i + 1:]:
        r = _contraste(val_a, val_b)
        if r < SEUIL:
            trop_proches.append((nom_a, val_a, nom_b, val_b, r))

print(f"{BOLD}Contrôle C · Tokens — aucune paire indistinguable (< {SEUIL}){RESET}")
if trop_proches:
    print(f"  {RED}✗ FAIL{RESET}  {len(trop_proches)} paire(s) de tokens indistinguables")
    for nom_a, val_a, nom_b, val_b, r in trop_proches:
        print(f"       {nom_a} ({val_a}) / {nom_b} ({val_b}) = {r:.4f}")
    print("       Deux tokens si proches ne peuvent pas distinguer deux états.")
    print("       Fusionnez-les, ou écartez la valeur de l'un.")
else:
    print(f"  {GREEN}✓ PASS{RESET}")
print()

sys.exit(1 if (fautifs or divergents or trop_proches) else 0)
PYTHON
CODE=$?

if [ "$CODE" -eq 0 ]; then
    echo "${GREEN}${BOLD}✓ La portée du CSS est tenue${RESET}"
else
    echo "${RED}${BOLD}✗ La portée du CSS n'est pas tenue${RESET}"
fi
echo ""
exit "$CODE"
