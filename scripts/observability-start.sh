#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=scripts/_lib.sh
source "$(CDPATH= cd -- "$(dirname "$0")" && pwd)/_lib.sh"

require_cmd openssl podman
ensure_run_dirs
ensure_gateway_secret

network_name=crudwithotel-observability
podman network exists "${network_name}" || podman network create "${network_name}"
podman volume exists crudwithotel-loki-data || podman volume create crudwithotel-loki-data
podman volume exists crudwithotel-grafana-data || podman volume create crudwithotel-grafana-data
podman volume exists crudwithotel-alloy-data || podman volume create crudwithotel-alloy-data
podman rm -f "${observability_containers[@]}" 2>/dev/null || true

podman run --rm -d \
  --name loki \
  --network "${network_name}" \
  -v "${ROOT}/loki-config.yaml:/etc/loki/local-config.yaml:ro" \
  -v crudwithotel-loki-data:/loki \
  grafana/loki:3.6.3 \
  -config.file=/etc/loki/local-config.yaml

podman run --rm -d \
  --name prometheus \
  --network "${network_name}" \
  -v "${ROOT}/prometheus.yml:/etc/prometheus/prometheus.yml:ro" \
  prom/prometheus:v3.2.1 \
  --config.file=/etc/prometheus/prometheus.yml \
  --web.external-url=https://localhost/prometheus/ \
  --web.route-prefix=/prometheus/

podman run --rm -d \
  --name jaeger \
  --network "${network_name}" \
  -e COLLECTOR_OTLP_ENABLED=true \
  -e METRICS_STORAGE_TYPE=prometheus \
  -e PROMETHEUS_SERVER_URL=http://prometheus:9090/prometheus \
  -e PROMETHEUS_QUERY_NORMALIZE_CALLS=true \
  -e PROMETHEUS_QUERY_NORMALIZE_DURATION=true \
  -e QUERY_UI_CONFIG=/etc/jaeger/ui-config.json \
  -v "${ROOT}/ui-config.json:/etc/jaeger/ui-config.json:ro" \
  jaegertracing/all-in-one:1.76.0 \
  --query.base-path=/jaeger

podman run --rm -d \
  --name otel-collector \
  --network "${network_name}" \
  -v "${ROOT}/otel-collector-config.yaml:/etc/otelcol-contrib/config.yaml:ro" \
  -v "${LOG_DIR}:/var/log/crudwithotel:ro" \
  -v "${RUN_DIR}/otelcol:/var/lib/otelcol" \
  -p 127.0.0.1:4317:4317 \
  -p 127.0.0.1:4318:4318 \
  -p 127.0.0.1:8889:8889 \
  otel/opentelemetry-collector-contrib:0.120.0 \
  --config=/etc/otelcol-contrib/config.yaml

podman run --rm -d \
  --name alloy \
  --network "${network_name}" \
  -v "${ROOT}/alloy-config.alloy:/etc/alloy/config.alloy:ro" \
  -v crudwithotel-alloy-data:/var/lib/alloy/data \
  -p 127.0.0.1:12347:12347 \
  grafana/alloy:v1.12.2 \
  run --server.http.listen-addr=0.0.0.0:12345 --storage.path=/var/lib/alloy/data /etc/alloy/config.alloy

podman run --rm -d \
  --name grafana \
  --network "${network_name}" \
  -v "${ROOT}/grafana/provisioning/datasources:/etc/grafana/provisioning/datasources:ro" \
  -v "${ROOT}/grafana/provisioning/dashboards:/etc/grafana/provisioning/dashboards:ro" \
  -v "${ROOT}/grafana/dashboards:/etc/grafana/dashboards:ro" \
  -v crudwithotel-grafana-data:/var/lib/grafana \
  -e GF_SERVER_ROOT_URL=https://localhost/grafana/ \
  -e GF_SERVER_SERVE_FROM_SUB_PATH=true \
  -e GF_AUTH_ANONYMOUS_ENABLED=true \
  -e GF_AUTH_ANONYMOUS_ORG_ROLE=Viewer \
  -e GF_AUTH_DISABLE_LOGIN_FORM=true \
  -e GF_USERS_DEFAULT_THEME=light \
  -e GF_DASHBOARDS_DEFAULT_HOME_DASHBOARD_PATH=/etc/grafana/dashboards/edge-tasks-overview.json \
  grafana/grafana:12.3.3

podman run --rm -d \
  --name observability-gateway \
  --network "${network_name}" \
  -v "${ROOT}/observability-Caddyfile:/etc/caddy/Caddyfile:ro" \
  -v "${LOG_DIR}:/var/log/crudwithotel" \
  -e OBSERVABILITY_GATEWAY_SECRET="$(cat "${RUN_DIR}/observability-gateway-secret")" \
  -p 127.0.0.1:18080:8080 \
  caddy:2.10.2
