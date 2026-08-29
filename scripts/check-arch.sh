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
#  10. Génération d'URLs — aucun placeholder substitué par un littéral
#  11. Use cases async — instrumentés ou motivés (cf. kanban/348, 355)
#  12. Émission d'événements — via emettre()/publier() (cf. kanban/350, 351, 355)
#  13. Cible de journalisation — toujours sous kreek:: (cf. kanban/355)
#  14. Bundle CSS — aucune feuille orpheline (cf. kanban/342)
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
# Glob shell plutôt que `find -printf` : `-printf` est une extension GNU que le
# `find` de macOS ignore. La liste y était donc **vide**, et les axes 3 et 9 ne
# parcouraient rien tout en affichant « PASS » — un verrou qui rassure sans
# jamais regarder. La CI tournant sous Linux, l'écart ne se voyait qu'en local.
BCS=$(for d in src/app/*/; do basename "$d"; done \
    | grep -v '^shared_kernel$' \
    | sort \
    | tr '\n' ' ')

if [ -z "$BCS" ]; then
    echo "check-arch : aucun bounded context trouvé sous src/app/ — arrêt." >&2
    exit 1
fi

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

# Recolle les lignes de continuation d'un chaînage rustfmt (celles qui
# commencent par `.`) sur la ligne précédente. Sans ça, `state\n.spaces\n…` — la
# mise en forme par défaut d'un appel trop long — ne matche jamais
# `\bstate\.<bc>\b` : la sous-chaîne littérale n'existe sur aucune ligne prise
# isolément. C'est ce seul retour à la ligne qui a rendu invisibles les
# violations des cartes 277 et 296.
#
# Contrepartie assumée : le numéro rapporté devient celui du **début** du
# chaînage, pas la position exacte de l'appel fautif. L'axe désigne un repère à
# vérifier à la main, pas une position cliquable.
join_chains() {
    awk '{
        if ($0 ~ /^[[:space:]]*\./) { gsub(/^[[:space:]]+/, ""); line = line $0; next }
        if (line != "") print line
        line = $0
    }
    END { if (line != "") print line }'
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
#
# Ligne de base — violations connues, tolérées le temps qu'une carte les traite.
#
# Elle n'existe que parce que la réparation de l'axe (carte 297) a découvert
# d'un coup six violations que le verrou n'avait jamais pu voir : `find -printf`
# vidait la liste des BCs sur macOS, et les chaînages coupés par rustfmt
# échappaient au grep. Les corriger toutes d'un bloc aurait mêlé la réparation
# du verrou à un chantier d'ACL sans rapport.
#
# Chaque entrée porte le fichier et la carte qui la traitera. Toute violation
# **nouvelle** fait toujours échouer l'axe : c'est là tout l'intérêt d'une
# ligne de base explicite plutôt que d'un axe rendu non bloquant.
#
# Posée le 2026-08-13. Une entrée dont la carte est faite doit disparaître d'ici.
AXE3_BASELINE_REGEX='src/app/team_creation/io/web/finalize_team\.rs|src/app/team_creation/io/web/build_team/submit_team\.rs|src/app/team_creation/io/web/build_team/display_page\.rs|src/app/teams/io/web/widgets/team_selection_tester\.rs'
# ↑ team_creation → competitions (5 occurrences) ................... carte 300
#   teams → spaces, page de test des widgets (1 occurrence) ........ carte 301

echo -e "${BOLD}Axe 3 · Souveraineté des données entre BCs (pas de référence croisée)${RESET}"
axe3=""
for bc in $BCS; do
    [ -d "src/app/$bc" ] || continue
    while IFS= read -r f; do
        # `strip_comments` comme à l'axe 9 : sans lui, le commentaire qui
        # explique *pourquoi* on ne fait plus `state.<bc>` déclenche l'axe qu'il
        # documente. Un verrou qui hurle sur sa propre documentation se fait
        # désactiver dans la semaine.
        prod_code="$(strip_test_code "$f" | strip_comments | join_chains)"
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
axe3="$(printf '%s' "$axe3" | sed '/^$/d' | grep -vE "^($AXE3_BASELINE_REGEX):" || true)"
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

# ── Axe 10 : génération d'URLs ──────────────────────────────────────────────
#
# Un placeholder de route substitué par un **littéral** produit une URL qui
# compile, qui ne contient plus d'accolade, et qui est fausse. Vécu :
# `APPROVE_ALL_ENROLLMENTS.replace("{space_id}", "_")` — le handler n'avait pas
# besoin du `space_id`, le bouche-trou a tenu jusqu'au jour où
# `space_scope_middleware` s'est mis à exiger un ULID. Deux mois plus tard, un
# `400` muet, dans tous les espaces.
#
# La règle est étroite et sans faux positif : la valeur d'un placeholder vient
# d'un paramètre, jamais d'une constante écrite sur place. Le cas « placeholder
# oublié » n'est pas couvert ici — il l'est par les tests unitaires de `Routes`,
# qui appellent chaque constructeur et refusent une accolade survivante.
echo -e "${BOLD}Axe 10 · Génération d'URLs — aucun placeholder substitué par un littéral${RESET}"
axe10=$(grep -rnE 'replace\("\{[a-z_]+\}",[[:space:]]*"' --include="routes.rs" src/ 2>/dev/null || true)
axe10="$(printf '%s' "$axe10" | sed '/^$/d')"
if [ -n "$axe10" ]; then print_fail "$axe10"; else print_pass; fi
echo ""

# ── Axe 11 : instrumentation des use cases ──────────────────────────────────
#
# Un use case non instrumenté est un trou dans le journal, et il ne se voit
# nulle part : le code compile, les tests passent, et la seule conséquence est
# qu'une action utilisateur ne laisse aucune trace en production. C'est le
# manque qui a motivé l'épic E11 — les deux bugs du mode customisation
# n'échouaient pas, ils se trompaient, et rien ne les racontait.
#
# **La première version de cet axe ne visait que les fonctions dont le premier
# paramètre est `cmd: …Command`.** Elle vérifiait donc la forme rencontrée, pas
# la règle : les treize use cases de `competitions/admin/`, qui travaillent par
# identifiants nus, lui échappaient déjà — et un quatorzième serait passé muet.
#
# Le critère est maintenant « toute `pub async fn` de `use_cases/` ». Une
# fonction async y touche un dépôt, un port ou un bus : elle a une intention à
# raconter. Les helpers purs (`domain_error_message`, `resolve_skill_cost`) sont
# `pub fn` et restent hors périmètre — les instrumenter serait du bruit.
#
# L'exception se déclare **dans le code**, par `// arch:no-instrument — motif`,
# comme l'axe 6 le fait déjà avec `arch:ok`. Une liste tenue dans ce script
# aurait dérivé ; un marqueur adjacent à la fonction ne le peut pas, et il
# oblige à écrire pourquoi.
#
# ── Deux défauts de la première mise en œuvre ───────────────────────────────
#
# Elle ne lisait que **la ligne précédant la signature**, et y cherchait le mot
# `instrument`. Deux conséquences, opposées et toutes deux fausses :
#
# **Un attribut replié était refusé.** `cargo fmt` replie
# `#[tracing::instrument(…)]` dès qu'il dépasse la largeur : la ligne précédente
# devient `)]`, que le contrôle ne reconnaît pas. Une fonction correctement
# instrumentée échouait donc — et le symptôme pousse à poser un
# `arch:no-instrument` mensonger pour débloquer. Rencontré en carte 450.
#
# **N'importe quelle prose exemptait.** `precedente !~ /instrument/` est
# satisfait par une ligne de documentation contenant le mot : « ce use case
# n'est pas instrumenté » **taisait le contrôle**. Il se laissait donc réduire
# au silence par le mot même qu'il cherchait.
#
# Le critère porte maintenant sur un **état**, remis à zéro par une ligne vide
# ou une accolade en colonne 0 — les deux frontières d'un élément de premier
# niveau —, et les motifs sont précis : `#[tracing::instrument` et non
# `instrument`.
echo -e "${BOLD}Axe 11 · Use cases async — instrumentés ou motivés${RESET}"
axe11=$(
  for fichier in $(find src/app -path '*/use_cases/*' -name '*.rs' 2>/dev/null); do
    awk -v f="$fichier" '
      /^#\[cfg\(test\)\]/ { dans_test = 1 }
      dans_test { next }
      /^\}/                     { annote = 0 }
      /^[[:space:]]*$/          { annote = 0; next }
      /#\[tracing::instrument/  { annote = 1 }
      /arch:no-instrument/      { annote = 1 }
      /^pub async fn / {
        if (!annote) { print f ":" NR ": " $0 }
        annote = 0
      }
    ' "$fichier"
  done
)
axe11="$(printf '%s' "$axe11" | sed '/^$/d')"
if [ -n "$axe11" ]; then print_fail "$axe11"; else print_pass; fi
echo ""

# ── Axe 12 : émission d'événements ──────────────────────────────────────────
#
# `to_enveloppe()` engendre un **nouvel** identifiant : l'app event n'a pas
# celui du domain event dont il est issu. Une ligne écrite à la main au-dessus
# d'un `send` a donc toutes les chances de reprendre l'identifiant reçu, et de
# produire une trace qui a l'air correcte et ne corrèle rien. `emettre()` et
# `publier()` ne voient que l'enveloppe produite : le piège est fermé par
# construction.
#
# **Les deux premières versions de cet axe cherchaient des noms de variables**
# — `app_event_bus`, puis `bus` et `event_bus`. Un bus nommé autrement serait
# passé, et rien ne l'aurait signalé. Le critère porte maintenant sur `.send(`
# quel que soit le récepteur ; ce qui n'est pas un bus se déclare par
# `// arch:ok`, comme partout ailleurs dans ce script.
echo -e "${BOLD}Axe 12 · Émission d'événements — toujours via emettre() ou publier()${RESET}"
axe12=$(
  for fichier in $(grep -rl '\.send(' --include="*.rs" src/ 2>/dev/null); do
    if [[ "$fichier" == */tests/* ]] \
      || [[ "$fichier" == *domain_event_publication.rs ]] \
      || [[ "$fichier" == *app_event_publication.rs ]]; then
      continue
    fi
    awk -v f="$fichier" '
      /^#\[cfg\(test\)\]/ { dans_test = 1 }
      dans_test { next }
      # `arch:ok` accepté sur la ligne ou juste au-dessus : un `.send(` qui
      # ouvre un appel multi-lignes ne laisse pas de place pour un commentaire
      # de fin de ligne que `rustfmt` ne déplacerait pas.
      /\.send\(/ && !/arch:ok/ && precedente !~ /arch:ok/ { print f ":" NR ": " $0 }
      { precedente = $0 }
    ' "$fichier"
  done
)
axe12="$(printf '%s' "$axe12" | sed '/^$/d')"
if [ -n "$axe12" ]; then print_fail "$axe12"; else print_pass; fi
echo ""

# ── Axe 13 : cible de journalisation ────────────────────────────────────────
#
# Le filtre vaut `kreek=<niveau>,sqlx=warn`. Une cible qui n'en relève pas
# n'est activée par aucune directive : **la ligne n'existe pas**, et rien ne le
# signale — ni à la compilation, ni aux tests, ni au démarrage.
#
# Le piège a coûté deux cartes. La 344 a trouvé le `TraceLayer` muet sur
# `tower_http::trace`. La 349 a failli livrer un `CatchPanicLayer` muet sur
# `tower_http::catch_panic`, avec un `500` propre et zéro ligne de journal —
# c'est-à-dire l'apparence exacte du travail fait.
#
# Il reparaîtra à chaque couche tierce branchée en comptant sur sa
# journalisation intégrée : une bibliothèque journalise sur son propre nom, et
# notre filtre ne connaît que le nôtre.
echo -e "${BOLD}Axe 13 · Cible de journalisation — toujours sous kreek::${RESET}"
axe13=$(
  for fichier in $(grep -rl 'target: *"' --include="*.rs" src/ 2>/dev/null); do
    [[ "$fichier" == */tests/* ]] && continue
    awk -v f="$fichier" '
      /^#\[cfg\(test\)\]/ { dans_test = 1 }
      dans_test { next }
      /target: *"/ && !/target: *"kreek/ { print f ":" NR ": " $0 }
    ' "$fichier"
  done
)
axe13="$(printf '%s' "$axe13" | sed '/^$/d')"
if [ -n "$axe13" ]; then print_fail "$axe13"; else print_pass; fi
echo ""

# ── Axe 14 : exhaustivité du bundle CSS ─────────────────────────────────────
#
# Depuis la carte 342, les feuilles ne sont plus liées par les templates : elles
# sont réunies en un fichier unique dont la **liste vit dans le code**. Une
# feuille ajoutée sans y être inscrite ne serait servie nulle part, et rien ne
# le dirait — la page s'afficherait simplement sans ses styles.
#
# Une feuille qui n'a pas vocation à être servie le déclare dans son en-tête par
# `css:mort`, avec son motif, comme `css:global` déclare une feuille partagée.
# Le marqueur vit dans le fichier et non dans une liste tenue ici : une liste
# dérive, un marqueur adjacent ne le peut pas.
echo -e "${BOLD}Axe 14 · Bundle CSS — aucune feuille orpheline${RESET}"
axe14=$(
  liste=$(sed -n '/FEUILLES_APP: &\[&str\] = &\[/,/^\];/p' src/web/css_bundle.rs)
  for f in $(find assets/static/css -name '*.css' | sed 's|assets/static/css/||' | sort); do
    grep -q "\"$f\"" <<< "$liste" && continue
    grep -q 'css:mort' "assets/static/css/$f" && continue
    grep -rq "href=\"/static/css/$f\"" src/app/auth 2>/dev/null && continue
    echo "assets/static/css/$f : ni dans le bundle, ni déclarée morte, ni chargée par auth"
  done
)
axe14="$(printf '%s' "$axe14" | sed '/^$/d')"
if [ -n "$axe14" ]; then print_fail "$axe14"; else print_pass; fi
echo ""

# ── Axe 15 : portée du CSS ──────────────────────────────────────────────────
#
# Délégué à `scripts/check-css-collisions.sh`, et non recopié ici : il porte
# deux contrôles, sa propre documentation et ses messages, et les absorber
# dupliquerait deux cents lignes de Python sans rien resserrer.
#
# Il existait depuis la carte 341 et n'était **branché nulle part** — ni ici, ni
# dans `make lint`, ni dans un job de CI. Il était rouge depuis la carte 17, et
# personne ne l'a su : c'est exactement ce que le CLAUDE.md décrit à propos du
# formatage, une cible que personne n'exécute finit rouge en silence. Le verrou
# de la 341 n'avait donc jamais rien gardé.
echo -e "${BOLD}Axe 15 · Portée du CSS — délégué à check-css-collisions.sh${RESET}"
if axe15=$(bash "$(dirname "$0")/check-css-collisions.sh" 2>&1); then
  print_pass
else
  # La sortie complète est réaffichée : savoir que ça échoue ne sert à rien sans
  # savoir quels sélecteurs divergent.
  print_fail "$axe15"
fi
echo ""

if [ "$EXIT_CODE" -eq 0 ]; then
    echo -e "${GREEN}${BOLD}✓ Toutes les vérifications bloquantes passent${RESET}"
else
    echo -e "${RED}${BOLD}✗ Des vérifications architecturales ont échoué${RESET}"
fi
echo ""

exit "$EXIT_CODE"
