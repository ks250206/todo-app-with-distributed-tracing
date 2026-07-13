#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=scripts/_lib.sh
source "$(CDPATH= cd -- "$(dirname "$0")" && pwd)/_lib.sh"

# PIDファイルと実際の待受プロセスの両方を確認する。
# cargo runの親だけが終了してBackendが残った場合も、repository外のプロセスは停止しない。

pid_file="${RUN_DIR}/backend.pid"
listener_pids="$(lsof -t -nP -iTCP:3000 -sTCP:LISTEN 2>/dev/null || true)"
recorded_pid=""
if test -s "${pid_file}"; then
  recorded_pid="$(cat "${pid_file}")"
fi

backend_pids=""
for process_pid in ${recorded_pid} ${listener_pids}; do
  test -n "${process_pid}" || continue
  kill -0 "${process_pid}" 2>/dev/null || continue

  process_cwd="$(lsof -a -p "${process_pid}" -d cwd -Fn 2>/dev/null | sed -n 's/^n//p')"
  process_command="$(ps -p "${process_pid}" -o command= 2>/dev/null || true)"

  is_backend=false
  case "${process_command}" in
    *axum-otel-crud* | *'cargo run'*) is_backend=true ;;
  esac

  if test "${process_cwd}" = "${BACKEND_DIR}" && test "${is_backend}" = true; then
    case " ${backend_pids} " in
      *" ${process_pid} "*) ;;
      *) backend_pids="${backend_pids} ${process_pid}" ;;
    esac
  elif printf '%s\n' "${listener_pids}" | grep -qx "${process_pid}"; then
    echo "port 3000 is used by another process (PID ${process_pid}): ${process_command}" >&2
    exit 1
  fi
done

for process_pid in ${backend_pids}; do
  kill -TERM "${process_pid}" 2>/dev/null || true
done

for _ in $(seq 1 30); do
  any_running=false
  for process_pid in ${backend_pids}; do
    if kill -0 "${process_pid}" 2>/dev/null; then
      any_running=true
    fi
  done
  if test "${any_running}" != true; then
    break
  fi
  sleep 1
done

for process_pid in ${backend_pids}; do
  if kill -0 "${process_pid}" 2>/dev/null; then
    echo "backend did not stop gracefully (PID ${process_pid})" >&2
    exit 1
  fi
done

rm -f "${pid_file}"
