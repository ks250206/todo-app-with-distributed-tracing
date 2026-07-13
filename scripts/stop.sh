#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=scripts/_lib.sh
source "$(CDPATH= cd -- "$(dirname "$0")" && pwd)/_lib.sh"

if test -f "${RUN_DIR}/caddy.pid"; then
  kill "$(cat "${RUN_DIR}/caddy.pid")" 2>/dev/null || true
fi
"${SCRIPTS_DIR}/backend-stop.sh"
"${SCRIPTS_DIR}/observability-stop.sh"
rm -rf "${RUN_DIR}"
