import type {
  EventEvent,
  ExceptionEvent,
  Faro,
  MeasurementEvent,
  TransportItem,
} from "@grafana/faro-web-sdk";

let faroInstance: Faro | undefined;

export function sanitizeTelemetryUrl(rawUrl: string | undefined): string | undefined {
  if (!rawUrl) return undefined;
  try {
    return new URL(rawUrl, window.location.origin).pathname;
  } catch {
    return rawUrl.split(/[?#]/, 1)[0];
  }
}

function looksLikeUrl(value: string): boolean {
  return /^(?:[a-z][a-z0-9+.-]*:)?\/\//i.test(value) || value.includes("?") || value.includes("#");
}

function sanitizeAttributeValue(key: string, value: string): string {
  if (key === "name" || /url/i.test(key) || looksLikeUrl(value)) {
    return sanitizeTelemetryUrl(value) ?? value.split(/[?#]/, 1)[0] ?? value;
  }
  return value;
}

function sanitizeAttributes(
  attributes: Record<string, string> | undefined,
): Record<string, string> | undefined {
  if (!attributes) return undefined;
  return Object.fromEntries(
    Object.entries(attributes).map(([key, value]) => [key, sanitizeAttributeValue(key, value)]),
  );
}

function sanitizeException(payload: ExceptionEvent): ExceptionEvent {
  return {
    ...payload,
    value: payload.value ? payload.value.slice(0, 2_000) : payload.value,
    stacktrace: payload.stacktrace
      ? {
          ...payload.stacktrace,
          frames: payload.stacktrace.frames?.map((frame) => ({
            ...frame,
            filename: sanitizeTelemetryUrl(frame.filename) ?? frame.filename,
          })),
        }
      : payload.stacktrace,
  };
}

function sanitizeEvent(payload: EventEvent): EventEvent {
  return {
    ...payload,
    attributes: sanitizeAttributes(payload.attributes),
  };
}

function sanitizeMeasurement(payload: MeasurementEvent): MeasurementEvent {
  return {
    ...payload,
    context: sanitizeAttributes(payload.context),
  };
}

export function sanitizeItem(item: TransportItem): TransportItem {
  const page = item.meta.page
    ? { ...item.meta.page, url: sanitizeTelemetryUrl(item.meta.page.url) }
    : undefined;
  const browser = item.meta.browser ? { ...item.meta.browser, userAgent: undefined } : undefined;
  const meta = {
    ...item.meta,
    user: undefined,
    page,
    browser,
  };

  switch (item.type) {
    case "exception":
      return { ...item, meta, payload: sanitizeException(item.payload as ExceptionEvent) };
    case "event":
      return { ...item, meta, payload: sanitizeEvent(item.payload as EventEvent) };
    case "measurement":
      return { ...item, meta, payload: sanitizeMeasurement(item.payload as MeasurementEvent) };
    default:
      return { ...item, meta };
  }
}

async function initializeFrontendLogs(): Promise<void> {
  const { getWebInstrumentations, initializeFaro } = await import("@grafana/faro-web-sdk");
  faroInstance = initializeFaro({
    url: "/faro/collect",
    app: {
      name: "todo-frontend",
      namespace: "edge-tasks",
      version: "0.1.0",
      environment: import.meta.env.MODE,
    },
    instrumentations: [
      ...getWebInstrumentations({
        captureConsole: false,
        enablePerformanceInstrumentation: true,
      }),
    ],
    beforeSend: sanitizeItem,
    ignoreUrls: [/\/faro\/collect/, /\/otel\/v1\/traces/],
    pageTracking: {
      generatePageId: (location) => location.pathname,
    },
    preventGlobalExposure: true,
    sessionTracking: {
      enabled: true,
      persistent: false,
      samplingRate: 1,
    },
    trackGeolocation: false,
  });
}

export function scheduleFrontendLogs(): void {
  if (import.meta.env.MODE === "test") return;
  const initialize = () => {
    void initializeFrontendLogs().catch(() => {
      console.warn("frontend observability initialization failed");
    });
  };
  if ("requestIdleCallback" in window) {
    window.requestIdleCallback(initialize, { timeout: 2_000 });
  } else {
    globalThis.setTimeout(initialize, 0);
  }
}

export function reportFrontendError(error: unknown, operation: string): void {
  if (!faroInstance) return;
  const reportable = error instanceof Error ? error : new Error("frontend operation failed");
  faroInstance.api.pushError(reportable, {
    context: { operation },
  });
}
