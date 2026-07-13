import { describe, expect, test } from "vite-plus/test";
import { sanitizeTelemetryUrl } from "./frontend-observability";

describe("frontend observability", () => {
  test("removes query parameters and fragments from telemetry URLs", () => {
    expect(sanitizeTelemetryUrl("https://localhost/dashboard/?token=secret#todo")).toBe(
      "/dashboard/",
    );
  });
});
