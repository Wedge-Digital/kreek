#!/usr/bin/env bash
# Lance les seuls tests Rust des fichiers donnés et écrit un rapport JUnit —
# appelé par `make test-files FILES=… REPORT=…`.
#
# Les tests sont co-localisés dans src/ : un fichier y est un module, et son
# chemin donne le préfixe des noms de tests (src/app/x/tests/test_y.rs →
# app::x::tests::test_y::). cargo-nextest filtre sur ce préfixe et écrit le
# rapport (profil `files`, cf. .config/nextest.toml).
#
# Le rapport est écrit même en cas d'échec. Quand nextest n'en produit pas —
# erreur de compilation, cas ordinaire d'un test qui appelle du code pas encore
# écrit, ou fichier sans aucun test — un rapport d'un seul cas en erreur le
# remplace, avec la fin de la sortie de cargo. En <error>, pas en <failure> :
# la porte rouge de l'usine n'accepte une erreur que si la sortie de rustc dit
# « code absent » (red_missing_code_patterns de .factory.toml) ; un test qui ne
# compile pas pour une autre raison, ou un fichier sans test, est refusé.
#
# Code de retour : celui de nextest (0 = tous les tests passent).
set -uo pipefail

report="$1"; shift
[ $# -gt 0 ] || { echo "aucun fichier de test" >&2; exit 2; }

filterset=""
for f in "$@"; do
    case "$f" in
        src/*.rs) ;;
        *) echo "« $f » : seuls les tests Rust de src/ (*.rs) sont pris en charge" >&2; exit 2 ;;
    esac
    [ -f "$f" ] || { echo "« $f » : fichier introuvable" >&2; exit 2; }
    mod="${f#src/}"; mod="${mod%.rs}"; mod="${mod%/mod}"
    filterset="${filterset:+$filterset | }test(/^${mod//\//::}::/)"
done

junit="target/nextest/files/junit.xml"
log="$(mktemp)"
trap 'rm -f "$log"' EXIT
rm -f "$junit"
mkdir -p "$(dirname "$report")"

cargo nextest run --profile files --no-fail-fast --no-tests=fail -E "$filterset" 2>&1 | tee "$log"
rc=${PIPESTATUS[0]}

if [ -f "$junit" ] && grep -q '<testcase' "$junit"; then
    cp "$junit" "$report"
else
    [ "$rc" -ne 0 ] || rc=1
    details="$(tail -n 200 "$log" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' -e 's/"/\&quot;/g')"
    cat > "$report" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="test-files" tests="1" failures="0" errors="1">
  <testsuite name="test-files" tests="1" failures="0" errors="1">
    <testcase classname="test-files" name="compilation ou sélection des tests">
      <error type="cargo-nextest" message="aucun test exécuté (code $rc) : erreur de compilation ou fichier sans test">$details</error>
    </testcase>
  </testsuite>
</testsuites>
EOF
fi

echo "rapport JUnit : $report"
exit "$rc"
