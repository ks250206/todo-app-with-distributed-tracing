#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=scripts/_lib.sh
source "$(CDPATH= cd -- "$(dirname "$0")" && pwd)/_lib.sh"

require_cmd caddy
mkdir -p "${LOG_DIR}"

OBSERVABILITY_GATEWAY_SECRET=validation \
  caddy validate --config "${ROOT}/Caddyfile"

OBSERVABILITY_GATEWAY_SECRET=validation \
  OBSERVABILITY_CADDY_ACCESS_LOG="${LOG_DIR}/observability-gateway-access.json" \
  caddy validate --adapter caddyfile --config "${ROOT}/observability-Caddyfile"
