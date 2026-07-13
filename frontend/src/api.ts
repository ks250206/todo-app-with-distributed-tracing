import { SpanKind, SpanStatusCode, trace } from "@opentelemetry/api";
import { reportFrontendError } from "./frontend-observability";

export type Session = { id: number; email?: string; role?: "admin" | "user" };
export type Todo = {
  id: number;
  title: string;
  completed: boolean;
  created_at: number;
  updated_at: number;
};

export class ApiError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

let refreshRequest: Promise<boolean> | undefined;
let unauthorizedHandler: (() => void) | undefined;

export function setUnauthorizedHandler(handler: () => void): () => void {
  unauthorizedHandler = handler;
  return () => {
    if (unauthorizedHandler === handler) unauthorizedHandler = undefined;
  };
}

async function refreshSession(): Promise<boolean> {
  refreshRequest ??= fetch("/api/auth/refresh", {
    method: "POST",
    credentials: "include",
  })
    .then((response) => {
      if (response.ok) return true;
      if (response.status === 401) return false;
      throw new ApiError(response.status, `Session refresh failed (${response.status})`);
    })
    .finally(() => {
      refreshRequest = undefined;
    });
  return refreshRequest;
}

async function request<T>(path: string, init: RequestInit = {}, retry = true): Promise<T> {
  const headers = new Headers(init.headers);
  if (init.body) headers.set("content-type", "application/json");
  const response = await fetch(path, {
    ...init,
    credentials: "include",
    headers,
  });

  if (response.status === 401 && retry && !path.startsWith("/api/auth/")) {
    if (await refreshSession()) return request<T>(path, init, false);
  }
  if (response.status === 401 && !path.startsWith("/api/auth/")) unauthorizedHandler?.();
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as { error?: string } | null;
    throw new ApiError(response.status, body?.error ?? `Request failed (${response.status})`);
  }
  if (response.status === 204) return undefined as T;
  return response.json() as Promise<T>;
}

export function tracedMutation<T>(name: string, operation: () => Promise<T>): Promise<T> {
  return trace
    .getTracer("todo-frontend")
    .startActiveSpan(name, { kind: SpanKind.INTERNAL }, async (span) => {
      try {
        return await operation();
      } catch (error) {
        span.recordException(error instanceof Error ? error : String(error));
        span.setStatus({
          code: SpanStatusCode.ERROR,
          message: error instanceof Error ? error.message : String(error),
        });
        reportFrontendError(error, name);
        throw error;
      } finally {
        span.end();
      }
    });
}

export const api = {
  me: () => request<Session>("/api/me"),
  login: (email: string, password: string) =>
    request<Session>("/api/auth/login", {
      method: "POST",
      body: JSON.stringify({ email, password }),
    }),
  register: (email: string, password: string) =>
    request<Session>("/api/auth/register", {
      method: "POST",
      body: JSON.stringify({ email, password }),
    }),
  logout: () => request<void>("/api/auth/logout", { method: "POST" }),
  listTodos: () => request<Todo[]>("/api/todos"),
  createTodo: (title: string) =>
    request<Todo>("/api/todos", { method: "POST", body: JSON.stringify({ title }) }),
  updateTodo: (id: number, input: { title?: string; completed?: boolean }) =>
    request<Todo>(`/api/todos/${id}`, { method: "PATCH", body: JSON.stringify(input) }),
  deleteTodo: (id: number) => request<void>(`/api/todos/${id}`, { method: "DELETE" }),
};
