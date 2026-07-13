import { TransportItemType } from "@grafana/faro-web-sdk";
import { describe, expect, test } from "vite-plus/test";
import { sanitizeItem, sanitizeTelemetryUrl } from "./frontend-observability";

describe("frontend observability", () => {
  test("removes query parameters and fragments from telemetry URLs", () => {
    expect(sanitizeTelemetryUrl("https://localhost/dashboard/?token=secret#todo")).toBe(
      "/dashboard/",
    );
  });

  test("sanitizes performance event resource URLs before send", () => {
    const item = sanitizeItem({
      type: TransportItemType.EVENT,
      meta: {
        page: { url: "https://localhost/dashboard/?session=1" },
        browser: { userAgent: "secret-agent" },
        user: { email: "user@example.test" },
      },
      payload: {
        name: "faro.performance.resource",
        timestamp: "2026-01-01T00:00:00.000Z",
        attributes: {
          name: "https://localhost/api/todos/42?token=secret",
          initiatorType: "fetch",
        },
      },
    });

    expect(item.meta.page?.url).toBe("/dashboard/");
    expect(item.meta.browser?.userAgent).toBeUndefined();
    expect(item.meta.user).toBeUndefined();
    expect(item.payload).toMatchObject({
      attributes: {
        name: "/api/todos/42",
        initiatorType: "fetch",
      },
    });
  });
});
