#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=scripts/_lib.sh
source "$(CDPATH= cd -- "$(dirname "$0")" && pwd)/_lib.sh"

require_cmd podman

service="${1:-}"
case "${service}" in
  jaeger | otel-collector | prometheus | loki | grafana | alloy | observability-gateway) ;;
  *)
    echo "usage: $0 <jaeger|otel-collector|prometheus|loki|grafana|alloy|observability-gateway>" >&2
    exit 1
    ;;
esac

podman logs -f "${service}"
