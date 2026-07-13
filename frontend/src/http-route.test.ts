import { describe, expect, test } from "vite-plus/test";
import { normalizeApiRoute } from "./http-route";

describe("normalizeApiRoute", () => {
  test("keeps static API paths", () => {
    expect(normalizeApiRoute("/api/me")).toBe("/api/me");
    expect(normalizeApiRoute("/api/todos")).toBe("/api/todos");
    expect(normalizeApiRoute("/api/auth/login")).toBe("/api/auth/login");
  });

  test("templates todo ids and strips query strings", () => {
    expect(normalizeApiRoute("/api/todos/42")).toBe("/api/todos/{id}");
    expect(normalizeApiRoute("/api/todos/42?x=1")).toBe("/api/todos/{id}");
  });
});
