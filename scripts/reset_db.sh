#!/usr/bin/env bash
set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ -n "${EXEC_PROFILE:-}" ]; then
    :
elif [ -n "${1:-}" ] && [[ "${1}" != -* ]]; then
    EXEC_PROFILE="$1"; shift
else
    EXEC_PROFILE="dev"
fi
export EXEC_PROFILE

"${SCRIPTS_DIR}/import_users.sh" "$@"
"${SCRIPTS_DIR}/import_spaces.sh" "$@"
"${SCRIPTS_DIR}/import_articles.sh" "$@"
