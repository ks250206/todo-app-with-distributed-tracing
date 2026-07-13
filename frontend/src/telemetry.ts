import { diag, DiagConsoleLogger, DiagLogLevel, type Span } from "@opentelemetry/api";
import { OTLPTraceExporter } from "@opentelemetry/exporter-trace-otlp-http";
import { registerInstrumentations } from "@opentelemetry/instrumentation";
import { FetchInstrumentation } from "@opentelemetry/instrumentation-fetch";
import { resourceFromAttributes } from "@opentelemetry/resources";
import { BatchSpanProcessor, WebTracerProvider } from "@opentelemetry/sdk-trace-web";
import {
  ATTR_HTTP_ROUTE,
  ATTR_SERVICE_NAME,
  ATTR_SERVICE_VERSION,
} from "@opentelemetry/semantic-conventions";
import { normalizeApiRoute } from "./http-route";

declare global {
  var __todoFrontendTelemetryInitialized: boolean | undefined;
}

function requestMethod(request: Request | RequestInit): string {
  if (request instanceof Request) return request.method.toUpperCase();
  return (request.method ?? "GET").toUpperCase();
}

function applyFetchRouteAttributes(
  span: Span,
  request: Request | RequestInit,
  rawUrl: string,
): void {
  const path = new URL(rawUrl, window.location.origin).pathname;
  const route = normalizeApiRoute(path);
  const method = requestMethod(request);
  span.setAttribute(ATTR_HTTP_ROUTE, route);
  span.updateName(`${method} ${route}`);
}

export function initializeTelemetry(): void {
  if (globalThis.__todoFrontendTelemetryInitialized) return;

  if (import.meta.env.DEV) diag.setLogger(new DiagConsoleLogger(), DiagLogLevel.WARN);

  const provider = new WebTracerProvider({
    resource: resourceFromAttributes({
      [ATTR_SERVICE_NAME]: "todo-frontend",
      [ATTR_SERVICE_VERSION]: "0.1.0",
    }),
    spanProcessors: [new BatchSpanProcessor(new OTLPTraceExporter({ url: "/otel/v1/traces" }))],
  });
  provider.register();

  registerInstrumentations({
    tracerProvider: provider,
    instrumentations: [
      new FetchInstrumentation({
        ignoreUrls: [/\/otel\/v1\/traces/, /\/faro\/collect/],
        propagateTraceHeaderCorsUrls: [/^https:\/\/localhost\/api\//, /^\/api\//],
        clearTimingResources: true,
        applyCustomAttributesOnSpan: (span, request, result) => {
          if (!("url" in result) || typeof result.url !== "string") return;
          applyFetchRouteAttributes(span, request, result.url);
        },
      }),
    ],
  });

  globalThis.__todoFrontendTelemetryInitialized = true;
}
