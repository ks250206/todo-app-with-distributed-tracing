#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=scripts/_lib.sh
source "$(CDPATH= cd -- "$(dirname "$0")" && pwd)/_lib.sh"

require_cmd caddy
export OTEL_SERVICE_NAME=caddy
export OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4318
export OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf
export OBSERVABILITY_GATEWAY_SECRET
OBSERVABILITY_GATEWAY_SECRET="$(cat "${RUN_DIR}/observability-gateway-secret")"
exec caddy run --config "${ROOT}/Caddyfile"
