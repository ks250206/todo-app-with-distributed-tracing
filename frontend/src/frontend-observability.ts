import type { Faro, TransportItem } from "@grafana/faro-web-sdk";

let faroInstance: Faro | undefined;

export function sanitizeTelemetryUrl(rawUrl: string | undefined): string | undefined {
  if (!rawUrl) return undefined;
  try {
    return new URL(rawUrl, window.location.origin).pathname;
  } catch {
    return rawUrl.split(/[?#]/, 1)[0];
  }
}

function sanitizeItem(item: TransportItem): TransportItem {
  const page = item.meta.page
    ? { ...item.meta.page, url: sanitizeTelemetryUrl(item.meta.page.url) }
    : undefined;
  const browser = item.meta.browser ? { ...item.meta.browser, userAgent: undefined } : undefined;
  return {
    ...item,
    meta: {
      ...item.meta,
      user: undefined,
      page,
      browser,
    },
  };
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
