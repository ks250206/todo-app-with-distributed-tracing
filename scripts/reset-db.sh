#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=scripts/_lib.sh
source "$(CDPATH= cd -- "$(dirname "$0")" && pwd)/_lib.sh"

# SQLiteを削除する前にBackendを止め、削除済みDBのopen file descriptorが残るのを防ぐ。
"${SCRIPTS_DIR}/backend-stop.sh"
rm -f "${BACKEND_DIR}/app.db" "${BACKEND_DIR}/app.db-shm" "${BACKEND_DIR}/app.db-wal"
echo "database reset; run 'just start' to recreate it"
