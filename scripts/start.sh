#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=scripts/_lib.sh
source "$(CDPATH= cd -- "$(dirname "$0")" && pwd)/_lib.sh"

require_cmd openssl curl cargo vp caddy podman lsof
require_password_pepper
ensure_run_dirs
ensure_gateway_secret

"${SCRIPTS_DIR}/observability-start.sh"
"${SCRIPTS_DIR}/backend-stop.sh"

(
  cd "${BACKEND_DIR}"
  cargo build --release
)
nohup sh -c 'cd "$1" && exec ./target/release/axum-otel-crud' sh "${BACKEND_DIR}" \
  >"${LOG_DIR}/backend.json" 2>&1 &
echo $! >"${RUN_DIR}/backend.pid"
wait_for_http_status "http://127.0.0.1:3000/api/me" "401" "${LOG_DIR}/backend.json"

(
  cd "${FRONTEND_DIR}"
  vp build
)

export OTEL_SERVICE_NAME=caddy
export OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4318
export OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf
export OBSERVABILITY_GATEWAY_SECRET
OBSERVABILITY_GATEWAY_SECRET="$(cat "${RUN_DIR}/observability-gateway-secret")"

if test -f "${RUN_DIR}/caddy.pid" && kill -0 "$(cat "${RUN_DIR}/caddy.pid")" 2>/dev/null; then
  caddy reload --config "${ROOT}/Caddyfile"
else
  rm -f "${RUN_DIR}/caddy.pid"
  caddy start --config "${ROOT}/Caddyfile" --pidfile "${RUN_DIR}/caddy.pid" \
    >"${RUN_DIR}/caddy.log" 2>&1
fi
wait_for_http_status "https://localhost/api/me" "401" "${RUN_DIR}/caddy.log" --insecure

echo "Frontend: https://localhost/"
echo "API: https://localhost/api/"
echo "Jaeger: https://localhost/jaeger/"
echo "Prometheus: https://localhost/prometheus/"
echo "Grafana: https://localhost/grafana/"
