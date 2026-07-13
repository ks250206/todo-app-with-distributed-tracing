#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=scripts/_lib.sh
source "$(CDPATH= cd -- "$(dirname "$0")" && pwd)/_lib.sh"

require_cmd podman

if test -f "${RUN_DIR}/backend.pid" && kill -0 "$(cat "${RUN_DIR}/backend.pid")" 2>/dev/null; then
  echo "backend: running"
else
  echo "backend: stopped"
fi

if test -f "${FRONTEND_DIR}/dist/index.html"; then
  echo "frontend: built"
else
  echo "frontend: not built"
fi

if test -f "${RUN_DIR}/caddy.pid" && kill -0 "$(cat "${RUN_DIR}/caddy.pid")" 2>/dev/null; then
  echo "caddy: running"
else
  echo "caddy: stopped"
fi

podman ps \
  --filter name=jaeger \
  --filter name=otel-collector \
  --filter name=prometheus \
  --filter name=loki \
  --filter name=grafana \
  --filter name=alloy \
  --filter name=observability-gateway
