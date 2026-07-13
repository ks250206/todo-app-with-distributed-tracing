import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { BrowserRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, test, vi } from "vite-plus/test";
import App from "./App";
import { useUiStore } from "./store";

function response(body: unknown, status = 200) {
  return new Response(status === 204 ? null : JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function renderApp() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </QueryClientProvider>,
  );
}

describe("App", () => {
  beforeEach(() => {
    window.history.replaceState({}, "", "/");
    useUiStore.setState({ filter: "all", authMode: "login" });
  });
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  test("shows the sign-in form for a signed-out user", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValueOnce(response({}, 401)).mockResolvedValueOnce(response({}, 401)),
    );
    renderApp();
    expect(await screen.findByRole("heading", { name: "Welcome back" })).toBeInTheDocument();
    expect(screen.getByLabelText("Email address")).toBeInTheDocument();
  });

  test("filters completed tasks", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValueOnce(response({ id: 7 }))
        .mockResolvedValueOnce(
          response([
            { id: 1, title: "Open task", completed: false, created_at: 1, updated_at: 1 },
            { id: 2, title: "Done task", completed: true, created_at: 1, updated_at: 1 },
          ]),
        ),
    );
    renderApp();
    expect(await screen.findByText("Open task")).toBeInTheDocument();
    await waitFor(() => expect(window.location.pathname).toBe("/dashboard/"));
    await userEvent.click(screen.getByRole("button", { name: "done" }));
    await waitFor(() => expect(screen.queryByText("Open task")).not.toBeInTheDocument());
    expect(screen.getByText("Done task")).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /Jaeger/ })).not.toBeInTheDocument();
  });

  test("shows observability links to an admin", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValueOnce(response({ id: 1, email: "admin@example.test", role: "admin" }))
        .mockResolvedValueOnce(response([])),
    );
    renderApp();
    expect(await screen.findByRole("link", { name: /Jaeger/ })).toHaveAttribute(
      "href",
      "/jaeger/monitor",
    );
    expect(screen.getByRole("link", { name: /Prometheus/ })).toHaveAttribute(
      "href",
      "/prometheus/",
    );
    expect(screen.getByRole("link", { name: /Grafana/ })).toHaveAttribute("href", "/grafana/");
  });

  test("redirects to the login page after logout", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValueOnce(response({ id: 7, role: "user" }))
        .mockResolvedValueOnce(response([]))
        .mockResolvedValueOnce(response(null, 204)),
    );
    renderApp();
    await userEvent.click(await screen.findByRole("button", { name: "Sign out" }));
    await waitFor(() => expect(window.location.pathname).toBe("/"));
    expect(await screen.findByRole("heading", { name: "Welcome back" })).toBeInTheDocument();
  });

  test("returns to login when refreshing an expired session fails", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValueOnce(response({ id: 7, role: "user" }))
        .mockResolvedValueOnce(response({}, 401))
        .mockResolvedValueOnce(response({}, 401)),
    );
    renderApp();
    expect(await screen.findByRole("heading", { name: "Welcome back" })).toBeInTheDocument();
    await waitFor(() => expect(window.location.pathname).toBe("/"));
  });
});
