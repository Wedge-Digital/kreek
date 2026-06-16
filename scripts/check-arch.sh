#!/usr/bin/env bash
# Vérifications architecturales (cf. CLAUDE.md) — utilisé par `make check-arch`.
#
# Axes bloquants (font échouer le script) :
#   2. Pureté du domaine (domain/ sans dépendance framework)
#   3. Souveraineté des données entre BCs (pas de référence croisée)
#   4. Pas de route en dur dans le front (toujours via routes.*)
#   5. Projections event sourcing dans la même transaction
#
# Axe en avertissement (n'affecte pas le code de sortie) :
#   6. Value objects systématiques (CQRS) — primitifs nus côté écriture domaine
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

BCS="auth competitions news players references spaces team_creation teams"

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

# Code de production uniquement : tronque au premier marqueur #[cfg(test)],
# les modules de test assemblent légitimement un AppState complet et n'ont
# pas à respecter la souveraineté des BCs.
strip_test_code() {
    awk '/#\[cfg\(test\)\]/{exit} {print}' "$1"
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
            hits=$(printf '%s\n' "$prod_code" | grep -nE "use crate::app::${other}::" | grep -vE "::routes::|auth_backend::AuthSession" || true)
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
    # Signature de la fonction étalée sur plusieurs lignes jusqu'à la
    # première accolade ouvrante : on la rassemble pour pouvoir grep
    # "PgPool" sans dépendre de la mise en forme.
    sig=$(awk '/fn[ \t]+[A-Za-z0-9_]*(update_projection|_projection)[A-Za-z0-9_]*[ \t]*\(/{flag=1} flag{print; if (/\{/) {flag=0}}' "$f")
    if printf '%s' "$sig" | grep -qE '\bPgPool\b' ; then
        axe5+="$f: fonction de projection prenant un PgPool au lieu d'une Transaction/Connection"$'\n'
    fi
done < <(grep -rlE "fn[ \t]+[A-Za-z0-9_]*(update_projection|_projection)[A-Za-z0-9_]*[ \t]*\(" --include="*.rs" src/ 2>/dev/null || true)
if [ -n "$axe5" ]; then print_fail "$axe5"; else print_pass; fi
echo ""

# ── Axe 6 (avertissement) : value objects systématiques (CQRS) ─────────────
echo -e "${BOLD}Axe 6 · Value objects systématiques — CQRS (avertissement)${RESET}"
axe6=$(grep -rnE "^\s*pub \w+:\s*(String|u32|u8|i32|bool)\b" --include="*.rs" src/app/*/domain/ 2>/dev/null \
    | grep -v "_port.rs" || true)
count=$(printf '%s\n' "$axe6" | sed '/^$/d' | wc -l | tr -d ' ')
if [ "$count" -gt 0 ]; then print_warn "$axe6" "$count"; else print_pass; fi
echo ""

if [ "$EXIT_CODE" -eq 0 ]; then
    echo -e "${GREEN}${BOLD}✓ Toutes les vérifications bloquantes passent${RESET}"
else
    echo -e "${RED}${BOLD}✗ Des vérifications architecturales ont échoué${RESET}"
fi
echo ""

exit "$EXIT_CODE"
