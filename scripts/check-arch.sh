#!/usr/bin/env bash
# Vérifications architecturales (cf. CLAUDE.md) — utilisé par `make check-arch`.
#
# Axes bloquants (font échouer le script) :
#   2. Pureté du domaine (domain/ sans dépendance framework)
#   3. Souveraineté des données entre BCs (pas de référence croisée)
#   4. Pas de route en dur dans le front (toujours via routes.*)
#   5. Projections event sourcing dans la même transaction
#   8. Carte d'impact e2e — exhaustive et sans entrée morte
#   9. BCs extractibles — aucune adhérence au host (cf. kanban/242)
#
# Axes en avertissement (n'affectent pas le code de sortie) :
#   6. Value objects systématiques (CQRS) — primitifs nus côté écriture domaine
#   7. Taille des fonctions — max 20 lignes (CLAUDE.md §"Taille des fonctions")
#
# Usage : ./scripts/check-arch.sh all

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

GREEN='\033[32m'
RED='\033[31m'
YELLOW='\033[33m'
BOLD='\033[1m'
RESET='\033[0m'

EXIT_CODE=0

# Dérivée du contenu de src/app/ plutôt qu'écrite à la main : une liste figée
# laissait match_report, ranking et spp_calculator hors de l'axe 3, créés après
# l'écriture de ce script sans que personne ne pense à les y ajouter. La boucle
# parcourant cette liste dans les deux sens, un BC absent était doublement
# invisible — ni ses propres références croisées, ni celles qui le visaient.
# `shared_kernel` en est exclu : il est partagé par construction.
BCS=$(find src/app -mindepth 1 -maxdepth 1 -type d -printf '%f\n' \
    | grep -v '^shared_kernel$' \
    | sort \
    | tr '\n' ' ')

# BCs maintenus copiables tels quels dans un autre projet (cf. kanban/242).
# Contraintes plus strictes que les autres BCs : aucune adhérence au host.
# Liste explicite et non dérivée : c'est un statut qu'on accorde et qu'on
# entretient, pas une propriété qu'on découvre en lisant l'arborescence.
EXTRACTABLE_BCS="auth spaces"

print_pass() { echo -e "  ${GREEN}✓ PASS${RESET}"; }
print_fail() {
    echo -e "  ${RED}✗ FAIL${RESET}"
    echo "$1" | sed 's/^/    /'
    EXIT_CODE=1
}
print_warn() {
    echo -e "  ${YELLOW}⚠ AVERTISSEMENT${RESET} ($2 occurrence(s) — non bloquant)"
    if [ -n "$1" ]; then
        echo "$1" | head -20 | sed 's/^/    /'
        if [ "$2" -gt 20 ]; then
            echo "    … ($(($2 - 20)) de plus, non affichées)"
        fi
    fi
}

# Code de production uniquement : tronque au premier marqueur #[cfg(test)]
# (forme externe, module de test inline) ou #![cfg(test)] (forme interne,
# fichier entier gaté depuis son mod.rs parent) — les modules de test
# assemblent légitimement un AppState complet et n'ont pas à respecter la
# souveraineté des BCs.
strip_test_code() {
    awk '/#!?\[cfg\(test\)\]/{exit} {print}' "$1"
}

# Retire les lignes de commentaire Rust. Sans ça, l'axe 9 signalerait les trois
# commentaires d'`auth` qui décrivent précisément l'`AppState` qu'on vient d'en
# sortir — et un verrou qui hurle sur sa propre documentation se fait désactiver
# dans la semaine.
strip_comments() {
    grep -vE '^[[:space:]]*//'
}

echo ""
echo -e "${BOLD}\033[34m┌─ Vérifications architecturales (CLAUDE.md)${RESET}"
echo ""

# ── Axe 2 : pureté du domaine ───────────────────────────────────────────────
echo -e "${BOLD}Axe 2 · Pureté du domaine (domain/ sans dépendance framework)${RESET}"
axe2=$(grep -rnE "^use (axum|sqlx|tower|askama)(::| )" --include="*.rs" src/app/*/domain/ 2>/dev/null || true)
if [ -n "$axe2" ]; then print_fail "$axe2"; else print_pass; fi
echo ""

# ── Axe 3 : souveraineté des données entre BCs ──────────────────────────────
echo -e "${BOLD}Axe 3 · Souveraineté des données entre BCs (pas de référence croisée)${RESET}"
axe3=""
for bc in $BCS; do
    [ -d "src/app/$bc" ] || continue
    while IFS= read -r f; do
        prod_code="$(strip_test_code "$f")"
        for other in $BCS; do
            [ "$other" = "$bc" ] && continue
            # Exemptions : la surface publique d'auth. Dans une extraction, tout
            # le monde dépend du fournisseur d'identité — c'en est la
            # définition. `AuthSession` était déjà exempté ; `User` le rejoint
            # depuis qu'il a quitté `shared_kernel` pour son BC propriétaire
            # (carte 243), sans quoi ses cinq consommateurs hors auth
            # deviendraient des violations alors que rien n'a changé pour eux.
            #
            # `SpacePermissions` relève de la même logique côté spaces : c'est
            # l'extracteur d'autorisation par espace (403 si non membre), le
            # pendant d'`AuthSession` pour le fournisseur d'appartenance. Il
            # vivait dans `src/web/` et n'était donc vu par personne ; la carte
            # 247 le rapatrie dans son BC propriétaire, ce qui rend visible une
            # dépendance qui existait déjà et n'a pas changé de nature.
            hits=$(printf '%s\n' "$prod_code" | grep -nE "use crate::app::${other}::" | grep -vE "::routes::|auth_backend::AuthSession|auth::domain::user::User|space_permissions::SpacePermissions" || true)
            [ -n "$hits" ] && axe3+="$(printf '%s\n' "$hits" | sed "s|^|$f:|")"$'\n'
            hits=$(printf '%s\n' "$prod_code" | grep -nE "\bstate\.${other}\b" || true)
            [ -n "$hits" ] && axe3+="$(printf '%s\n' "$hits" | sed "s|^|$f:|")"$'\n'
        done
    done < <(find "src/app/$bc" -name "*.rs")
done
axe3="$(printf '%s' "$axe3" | sed '/^$/d')"
if [ -n "$axe3" ]; then print_fail "$axe3"; else print_pass; fi
echo ""

# ── Axe 4 : pas de route en dur dans le front ───────────────────────────────
echo -e "${BOLD}Axe 4 · Pas de route en dur dans le front (toujours via routes.*)${RESET}"
axe4=$(grep -rnE '(hx-get|hx-post|hx-put|hx-delete|hx-target|href|action)="\/(app|auth|references|test)\/' --include="*.html" src/ assets/ 2>/dev/null || true)
axe4+=$'\n'"$(grep -rnE "(fetch\(|htmx\.ajax\('(GET|POST|PUT|DELETE)',)\s*['\"]\/(app|auth|references)\/" --include="*.html" src/ 2>/dev/null || true)"
axe4="$(printf '%s' "$axe4" | sed '/^$/d')"
if [ -n "$axe4" ]; then print_fail "$axe4"; else print_pass; fi
echo ""

# ── Axe 5 : projections dans la même transaction ────────────────────────────
echo -e "${BOLD}Axe 5 · Projections event sourcing dans la même transaction${RESET}"
axe5=""
while IFS= read -r f; do
    # Un listener dont `init()` prend un `app_event_bus` réagit à un app event
    # cross-BC déjà committé par un autre BC : par construction impossible de
    # partager une transaction avec ce commit distant. Ce cas est exclu de
    # l'axe 5, qui ne vise que les projections intra-BC (event + projection du
    # même BC, appendés dans le même flux). Cf. CLAUDE.md "Projections event
    # sourcing" et convention de nommage `app_event_bus` vs `event_bus` déjà
    # en place dans les listeners du projet.
    init_sig=$(awk '/fn[ \t]+init[ \t]*\(/{flag=1} flag{print; if (/\{/) {flag=0}}' "$f")
    if printf '%s' "$init_sig" | grep -q 'app_event_bus'; then
        continue
    fi
    # Code de production uniquement — les fichiers de test (gatés en
    # #[cfg(test)] inline ou #![cfg(test)] pour tout le fichier) ne sont pas
    # de vraies fonctions de projection, juste des noms de test qui matchent
    # le regex (ex. `..._in_projection(pool: PgPool)`).
    prod_code="$(strip_test_code "$f")"
    # Signature de la fonction étalée sur plusieurs lignes jusqu'à la
    # première accolade ouvrante : on la rassemble pour pouvoir grep
    # "PgPool" sans dépendre de la mise en forme.
    sig=$(printf '%s\n' "$prod_code" | awk '/fn[ \t]+[A-Za-z0-9_]*(insert|update|delete|upsert|apply|append|save)[A-Za-z0-9_]*projection[A-Za-z0-9_]*[ \t]*\(/{flag=1} flag{print; if (/\{/) {flag=0}}')
    if printf '%s' "$sig" | grep -qE '\bPgPool\b' ; then
        axe5+="$f: fonction de projection prenant un PgPool au lieu d'une Transaction/Connection"$'\n'
    fi
# Seules les fonctions d'écriture de projection (insert/update/delete/...) sont
# visées — les lectures (ex. `list_from_projection`) manipulent un PgPool
# légitimement, sans contrainte de transaction partagée.
done < <(grep -rlE "fn[ \t]+[A-Za-z0-9_]*(insert|update|delete|upsert|apply|append|save)[A-Za-z0-9_]*projection[A-Za-z0-9_]*[ \t]*\(" --include="*.rs" src/ 2>/dev/null || true)
if [ -n "$axe5" ]; then print_fail "$axe5"; else print_pass; fi
echo ""

# ── Axe 6 (avertissement) : value objects systématiques (CQRS) ─────────────
echo -e "${BOLD}Axe 6 · Value objects systématiques — CQRS (avertissement)${RESET}"
axe6=$(grep -rnE "^\s*pub \w+:\s*(String|u32|u8|i32|bool)\b" --include="*.rs" src/app/*/domain/ 2>/dev/null \
    | grep -v "_port\.rs\|_repository_port\.rs" \
    | grep -v "references/domain/models\.rs" \
    | grep -v "arch:ok" \
    || true)
count=$(printf '%s\n' "$axe6" | sed '/^$/d' | wc -l | tr -d ' ')
if [ "$count" -gt 0 ]; then print_warn "$axe6" "$count"; else print_pass; fi
echo ""

# ── Axe 7 (avertissement) : taille des fonctions ───────────────────────────
echo -e "${BOLD}Axe 7 · Taille des fonctions — max 20 lignes (avertissement)${RESET}"
axe7=$(find src/ -name "*.rs" -print0 2>/dev/null | xargs -0 awk '
FNR == 1 { in_fn = 0; depth = 0; fn_line = 0; fname = "" }
{
    line = $0
    if (index(line, "//") > 0) line = substr(line, 1, index(line, "//")-1)
    gsub(/"[^"]*"/, "", line)
    if (!in_fn && line ~ /[[:space:]]fn[[:space:]]/) {
        in_fn = 1; depth = 0; fn_line = FNR
        fname = $0; sub(/^[[:space:]]*/, "", fname); sub(/[({].*/, "", fname)
    }
    if (in_fn) {
        n = length(line)
        for (i = 1; i <= n; i++) {
            c = substr(line, i, 1)
            if (c == "{") depth++
            else if (c == "}") {
                depth--
                if (depth <= 0) {
                    cnt = FNR - fn_line + 1
                    if (cnt > 20) printf "%s:%d: %d lignes — %s\n", FILENAME, fn_line, cnt, fname
                    in_fn = 0
                }
            }
        }
    }
}
' 2>/dev/null || true)
count7=$(printf '%s\n' "$axe7" | sed '/^$/d' | wc -l | tr -d ' ')
if [ "$count7" -gt 0 ]; then print_warn "$axe7" "$count7"; else print_pass; fi
echo ""

# ── Axe 8 : intégrité de la carte d'impact e2e ──────────────────────────────
# La carte n'est utile que si elle est exhaustive : un test absent est traité
# comme transverse (donc toujours exécuté, sélection inutile), une entrée
# orpheline ou un BC inexistant signalent une carte qui a décroché du code.
echo -e "${BOLD}Axe 8 · Carte d'impact e2e — exhaustive et sans entrée morte${RESET}"
axe8=$(python3 - <<'PY' 2>/dev/null || true
import pathlib, sys, tomllib
root = pathlib.Path(".")
carte = root / "tests/impact-map.toml"
if not carte.exists():
    print("tests/impact-map.toml: fichier absent"); sys.exit()
try:
    data = tomllib.loads(carte.read_text())
except Exception as e:
    print(f"tests/impact-map.toml: TOML invalide — {e}"); sys.exit()

tests = data.get("tests", {})
deps = data.get("deps", {})
sur_disque = {p.stem for p in (root / "tests/e2e").glob("test_*.py")}
bcs = {p.name for p in (root / "src/app").iterdir() if p.is_dir() and p.name != "shared_kernel"}

for t in sorted(sur_disque - set(tests)):
    print(f"tests/e2e/{t}.py: test e2e sans entrée dans impact-map.toml")
for t in sorted(set(tests) - sur_disque):
    print(f"tests/impact-map.toml: entrée orpheline « {t} » (fichier de test inexistant)")
for t, declares in sorted(tests.items()):
    for bc in sorted(set(declares) - bcs - {"all"}):
        print(f"tests/impact-map.toml: « {t} » référence un BC inconnu « {bc} »")
for src, dependants in sorted(deps.items()):
    for bc in sorted({src} | set(dependants) - bcs):
        if bc not in bcs:
            print(f"tests/impact-map.toml: [deps] référence un BC inconnu « {bc} »")
PY
)
axe8="$(printf '%s' "$axe8" | sed '/^$/d')"
count8=$([ -z "$axe8" ] && echo 0 || printf '%s\n' "$axe8" | wc -l | tr -d ' ')
if [ "$count8" -gt 0 ]; then print_fail "$axe8"; else print_pass; fi
echo ""

# ── Axe 9 : BCs extractibles ────────────────────────────────────────────────
# La carte 242 a écarté le découpage en crates cargo, qui aurait confié ce
# contrôle au compilateur. Sans ce verrou, rien ne signale une régression : le
# précédent est documenté carte 237, où `recap_controller.rs` a atteint
# `state.spaces` pendant des mois sans que personne ne le voie.
#
# Limite assumée : ce script est un ensemble de `grep`, pas un analyseur
# syntaxique. Il ne voit ni les chaînes littérales, ni le SQL — le
# `LEFT JOIN auth__users` de spaces reste invisible (périmètre exclu de la 242).
echo -e "${BOLD}Axe 9 · BCs extractibles — aucune adhérence au host${RESET}"
axe9=""
for bc in $EXTRACTABLE_BCS; do
    if [ ! -d "src/app/$bc" ]; then
        axe9+="src/app/$bc: BC déclaré extractible mais introuvable"$'\n'
        continue
    fi

    while IFS= read -r f; do
        prod="$(strip_test_code "$f" | strip_comments)"

        add_hits() {
            [ -n "$1" ] && axe9+="$(printf '%s\n' "$1" | sed "s|^|$f:|")"$'\n'
        }

        # 245 — l'état global du host
        add_hits "$(printf '%s\n' "$prod" \
            | grep -nE "crate::state::|\bAppState\b|\bstate\.[a-z_]+" || true)"
        # 246 — l'agrégat de routes de l'application
        add_hits "$(printf '%s\n' "$prod" | grep -nE "\bAppRoutes\b" || true)"
        # 247 — la couche web du host : layout, extracteurs, middlewares
        add_hits "$(printf '%s\n' "$prod" | grep -n "crate::web::" || true)"
        # 244 — le noyau métier Blood Bowl
        add_hits "$(printf '%s\n' "$prod" | grep -n "shared_kernel::bloodbowl" || true)"

        # 246 — tout autre BC, ses routes comprises. L'exemption `::routes::`
        # de l'axe 3 ne vaut pas ici : un lien sortant s'injecte depuis l'hôte.
        # Seule exception, dans le sens spaces → auth : les deux BCs partent en
        # couple (décision 242), donc `spaces` consomme la session d'`auth`.
        # Rien d'autre — et surtout pas `auth::routes`. L'exemption est
        # naturellement directionnelle : `auth` n'importe pas sa propre session
        # par ce chemin, un import de soi n'étant pas un import croisé.
        for other in $BCS; do
            [ "$other" = "$bc" ] && continue
            add_hits "$(printf '%s\n' "$prod" | grep -nE "use crate::app::${other}::" \
                | grep -vE "auth::auth_backend::AuthSession" || true)"
        done
    done < <(find "src/app/$bc" -name "*.rs")

    # 247 — le chrome et les composants du host, côté templates. `import`
    # couple autant qu'`extends` : Askama résout les deux statiquement, et
    # `askama.toml` déclarant les onze dossiers dans un seul espace de noms,
    # rien ne signale qu'un template résolu vit chez le host. Le critère est
    # donc physique : la cible doit exister dans le dossier de templates du BC.
    while IFS= read -r t; do
        while IFS= read -r ligne; do
            [ -z "$ligne" ] && continue
            cible=$(printf '%s' "$ligne" \
                | sed -n 's/.*{%[[:space:]]*\(extends\|import\)[[:space:]]*"\([^"]*\)".*/\2/p')
            [ -z "$cible" ] && continue
            [ -f "src/app/$bc/io/web/templates/$cible" ] && continue
            axe9+="$t:${ligne%%:*}: template « $cible » hors du BC"$'\n'
        done < <(grep -nE "\{%[[:space:]]*(extends|import)" "$t" || true)
    done < <(find "src/app/$bc" -name "*.html")
done

# Réciproque du critère de sortie de la 242 : le noyau partagé ne connaît
# aucun BC extractible, sans quoi le copier n'emporterait pas grand-chose.
for bc in $EXTRACTABLE_BCS; do
    hits=$(grep -rn "crate::app::${bc}::" src/app/shared_kernel 2>/dev/null || true)
    [ -n "$hits" ] && axe9+="$hits"$'\n'
done

axe9="$(printf '%s' "$axe9" | sed '/^$/d')"
if [ -n "$axe9" ]; then print_fail "$axe9"; else print_pass; fi
echo ""

if [ "$EXIT_CODE" -eq 0 ]; then
    echo -e "${GREEN}${BOLD}✓ Toutes les vérifications bloquantes passent${RESET}"
else
    echo -e "${RED}${BOLD}✗ Des vérifications architecturales ont échoué${RESET}"
fi
echo ""

exit "$EXIT_CODE"
