#!/usr/bin/env bash
# Shared helpers for repository scripts.
set -euo pipefail

SCRIPTS_DIR="$(CDPATH= cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(CDPATH= cd -- "${SCRIPTS_DIR}/.." && pwd)"
RUN_DIR="${ROOT}/.run"
LOG_DIR="${RUN_DIR}/logs"
BACKEND_DIR="${ROOT}/backend"
FRONTEND_DIR="${ROOT}/frontend"

require_cmd() {
  local cmd
  for cmd in "$@"; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
      echo "required command not found: ${cmd}" >&2
      exit 1
    fi
  done
}

ensure_run_dirs() {
  mkdir -p "${LOG_DIR}" "${RUN_DIR}/otelcol/logs"
  chmod 0777 "${RUN_DIR}/otelcol/logs"
  # Preserve filelog offsets across the caddy/ -> logs/ storage path rename.
  if test -d "${RUN_DIR}/otelcol/caddy"; then
    cp -a "${RUN_DIR}/otelcol/caddy/." "${RUN_DIR}/otelcol/logs/"
    rm -rf "${RUN_DIR}/otelcol/caddy"
  fi
}

ensure_gateway_secret() {
  if ! test -s "${RUN_DIR}/observability-gateway-secret"; then
    umask 077
    openssl rand -hex 32 >"${RUN_DIR}/observability-gateway-secret"
  fi
}

require_password_pepper() {
  if test -n "${PASSWORD_PEPPER:-}"; then
    return 0
  fi
  if test -f "${BACKEND_DIR}/.env" && grep -Eq '^(export )?PASSWORD_PEPPER=.+' "${BACKEND_DIR}/.env"; then
    return 0
  fi
  echo "backend/.env または環境変数で PASSWORD_PEPPER を設定してください" >&2
  exit 1
}

wait_for_http_status() {
  local url="$1"
  local expected="$2"
  local log_file="${3:-}"
  local insecure="${4:-}"
  local code=""
  local curl_bin=(curl -s -o /dev/null -w '%{http_code}')

  if test "${insecure}" = "--insecure"; then
    curl_bin+=( -k )
  fi

  local _
  for _ in $(seq 1 30); do
    code="$("${curl_bin[@]}" "${url}" || true)"
    if test "${code}" = "${expected}"; then
      return 0
    fi
    sleep 1
  done

  echo "timed out waiting for ${url} to return ${expected} (last=${code})" >&2
  if test -n "${log_file}" && test -f "${log_file}"; then
    tail -n 100 "${log_file}" >&2
  fi
  exit 1
}

observability_containers=(
  jaeger
  otel-collector
  prometheus
  loki
  grafana
  alloy
  observability-gateway
)
