import { useEffect, useState, type FormEvent } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Navigate, Route, Routes, useNavigate } from "react-router-dom";
import {
  ApiError,
  api,
  setUnauthorizedHandler,
  tracedMutation,
  type Session,
  type Todo,
} from "./api";
import { useUiStore, type TodoFilter } from "./store";

const sessionKey = ["session"] as const;
const todosKey = ["todos"] as const;

function Logo() {
  return (
    <div className="flex items-center gap-2.5 font-semibold tracking-tight">
      <span className="grid size-8 place-items-center rounded-lg bg-orange-500 text-white shadow-sm shadow-orange-200">
        ◇
      </span>
      <span>Edge Tasks</span>
    </div>
  );
}

function AuthScreen() {
  const queryClient = useQueryClient();
  const mode = useUiStore((state) => state.authMode);
  const setMode = useUiStore((state) => state.setAuthMode);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const auth = useMutation({
    mutationFn: () =>
      tracedMutation(`auth.${mode}`, () =>
        mode === "login" ? api.login(email, password) : api.register(email, password),
      ),
    onSuccess: (session) => queryClient.setQueryData(sessionKey, session),
  });

  function submit(event: FormEvent) {
    event.preventDefault();
    auth.mutate();
  }

  return (
    <main className="grid min-h-screen bg-[#f7f7f5] lg:grid-cols-[1.08fr_.92fr]">
      <section className="hidden overflow-hidden bg-[#0b1f33] p-12 text-white lg:flex lg:flex-col">
        <Logo />
        <div className="my-auto max-w-xl">
          <p className="mb-5 text-xs font-semibold uppercase tracking-[.24em] text-orange-400">
            Observe every action
          </p>
          <h1 className="text-5xl font-semibold leading-[1.08] tracking-[-.04em]">
            Work moves fast.
            <br />
            Your tasks should too.
          </h1>
          <p className="mt-7 max-w-md text-base leading-7 text-slate-300">
            A focused workspace with distributed tracing built in. Every request stays visible from
            click to database.
          </p>
          <div className="mt-12 grid grid-cols-3 gap-3">
            {["Secure session", "Private by design", "OTel traced"].map((item, index) => (
              <div key={item} className="rounded-xl border border-white/10 bg-white/5 p-4">
                <span className="text-orange-400">0{index + 1}</span>
                <p className="mt-5 text-sm text-slate-200">{item}</p>
              </div>
            ))}
          </div>
        </div>
        <p className="text-xs text-slate-500">Local workspace · TLS protected</p>
      </section>

      <section className="flex min-h-screen items-center justify-center p-6 sm:p-12">
        <div className="w-full max-w-md">
          <div className="mb-12 lg:hidden">
            <Logo />
          </div>
          <p className="text-sm font-medium text-orange-600">Edge Tasks Console</p>
          <h2 className="mt-2 text-3xl font-semibold tracking-[-.03em] text-slate-950">
            {mode === "login" ? "Welcome back" : "Create your account"}
          </h2>
          <p className="mt-3 text-sm leading-6 text-slate-500">
            {mode === "login"
              ? "Sign in to manage your private task queue."
              : "Start with a secure, isolated workspace."}
          </p>

          <div className="mt-8 flex rounded-lg bg-slate-200/70 p-1" role="tablist">
            {(["login", "register"] as const).map((item) => (
              <button
                key={item}
                type="button"
                role="tab"
                aria-selected={mode === item}
                onClick={() => setMode(item)}
                className={`flex-1 rounded-md px-3 py-2 text-sm font-medium transition ${mode === item ? "bg-white text-slate-950 shadow-sm" : "text-slate-500 hover:text-slate-800"}`}
              >
                {item === "login" ? "Sign in" : "Register"}
              </button>
            ))}
          </div>

          <form className="mt-7 space-y-5" onSubmit={submit}>
            <label className="block text-sm font-medium text-slate-700">
              Email address
              <input
                type="email"
                required
                autoComplete="email"
                value={email}
                onChange={(event) => setEmail(event.target.value)}
                className="mt-2 w-full rounded-lg border border-slate-300 bg-white px-3.5 py-3 text-slate-950 outline-none transition focus:border-orange-500 focus:ring-3 focus:ring-orange-100"
                placeholder="you@example.com"
              />
            </label>
            <label className="block text-sm font-medium text-slate-700">
              Password
              <input
                type="password"
                required
                minLength={12}
                autoComplete={mode === "login" ? "current-password" : "new-password"}
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                className="mt-2 w-full rounded-lg border border-slate-300 bg-white px-3.5 py-3 text-slate-950 outline-none transition focus:border-orange-500 focus:ring-3 focus:ring-orange-100"
                placeholder="12 characters or more"
              />
            </label>
            {auth.error && (
              <p
                role="alert"
                className="rounded-lg border border-red-200 bg-red-50 px-3.5 py-3 text-sm text-red-700"
              >
                {auth.error instanceof ApiError ? auth.error.message : "Something went wrong"}
              </p>
            )}
            <button
              type="submit"
              disabled={auth.isPending}
              className="w-full rounded-lg bg-orange-500 px-4 py-3 text-sm font-semibold text-white shadow-sm transition hover:bg-orange-600 disabled:cursor-wait disabled:opacity-60"
            >
              {auth.isPending ? "Please wait…" : mode === "login" ? "Sign in" : "Create account"}
            </button>
          </form>
          <p className="mt-6 text-center text-xs leading-5 text-slate-400">
            Session tokens are stored in Secure, HttpOnly cookies.
          </p>
        </div>
      </section>
    </main>
  );
}

function Metric({ label, value, note }: { label: string; value: string | number; note: string }) {
  return (
    <div className="rounded-xl border border-slate-200 bg-white p-5 shadow-[0_1px_2px_rgba(15,23,42,.03)]">
      <p className="text-xs font-medium uppercase tracking-wider text-slate-500">{label}</p>
      <p className="mt-3 text-3xl font-semibold tracking-tight text-slate-950">{value}</p>
      <p className="mt-2 text-xs text-slate-400">{note}</p>
    </div>
  );
}

function TodoRow({ todo }: { todo: Todo }) {
  const queryClient = useQueryClient();
  const update = useMutation({
    mutationFn: () =>
      tracedMutation("todo.toggle", () => api.updateTodo(todo.id, { completed: !todo.completed })),
    onSuccess: (next) =>
      queryClient.setQueryData<Todo[]>(todosKey, (items = []) =>
        items.map((item) => (item.id === next.id ? next : item)),
      ),
  });
  const remove = useMutation({
    mutationFn: () => tracedMutation("todo.delete", () => api.deleteTodo(todo.id)),
    onSuccess: () =>
      queryClient.setQueryData<Todo[]>(todosKey, (items = []) =>
        items.filter((item) => item.id !== todo.id),
      ),
  });

  return (
    <li className="group flex items-center gap-3 border-b border-slate-100 px-4 py-3.5 last:border-0 sm:px-5">
      <button
        type="button"
        aria-label={`Mark ${todo.title} as ${todo.completed ? "open" : "done"}`}
        onClick={() => update.mutate()}
        disabled={update.isPending}
        className={`grid size-5 shrink-0 place-items-center rounded border text-[11px] transition ${todo.completed ? "border-emerald-500 bg-emerald-500 text-white" : "border-slate-300 bg-white text-transparent hover:border-orange-500"}`}
      >
        ✓
      </button>
      <div className="min-w-0 flex-1">
        <p
          className={`truncate text-sm ${todo.completed ? "text-slate-400 line-through" : "font-medium text-slate-800"}`}
        >
          {todo.title}
        </p>
        <p className="mt-1 text-[11px] text-slate-400">
          Task #{todo.id} · updated {new Date(todo.updated_at * 1000).toLocaleDateString()}
        </p>
      </div>
      <span
        className={`hidden rounded-full px-2 py-1 text-[10px] font-semibold uppercase sm:block ${todo.completed ? "bg-emerald-50 text-emerald-700" : "bg-amber-50 text-amber-700"}`}
      >
        {todo.completed ? "Done" : "Open"}
      </span>
      <button
        type="button"
        aria-label={`Delete ${todo.title}`}
        onClick={() => remove.mutate()}
        disabled={remove.isPending}
        className="rounded-md px-2 py-1 text-lg leading-none text-slate-300 opacity-100 transition hover:bg-red-50 hover:text-red-500 sm:opacity-0 sm:group-hover:opacity-100 sm:focus:opacity-100"
      >
        ×
      </button>
    </li>
  );
}

function Dashboard({ session }: { session: Session }) {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const [title, setTitle] = useState("");
  const filter = useUiStore((state) => state.filter);
  const setFilter = useUiStore((state) => state.setFilter);
  const todos = useQuery({ queryKey: todosKey, queryFn: api.listTodos });
  const create = useMutation({
    mutationFn: (nextTitle: string) =>
      tracedMutation("todo.create", () => api.createTodo(nextTitle)),
    onSuccess: (todo) => {
      queryClient.setQueryData<Todo[]>(todosKey, (items = []) => [todo, ...items]);
      setTitle("");
    },
  });
  const logout = useMutation({
    mutationFn: () => tracedMutation("auth.logout", api.logout),
    onSuccess: () => {
      queryClient.setQueryData(sessionKey, null);
      queryClient.removeQueries({ queryKey: todosKey });
      void navigate("/", { replace: true });
    },
  });
  const items = todos.data ?? [];
  const completed = items.filter((todo) => todo.completed).length;
  const visibleItems = items.filter(
    (todo) => filter === "all" || (filter === "done" ? todo.completed : !todo.completed),
  );

  function addTodo(event: FormEvent) {
    event.preventDefault();
    const nextTitle = title.trim();
    if (nextTitle) create.mutate(nextTitle);
  }

  return (
    <div className="min-h-screen bg-[#f7f7f5] text-slate-900">
      <header className="border-b border-slate-200 bg-white">
        <div className="mx-auto flex h-16 max-w-[1440px] items-center justify-between px-4 sm:px-6 lg:px-8">
          <Logo />
          <div className="flex items-center gap-3">
            <div className="hidden items-center gap-2 rounded-full border border-emerald-200 bg-emerald-50 px-3 py-1.5 text-xs font-medium text-emerald-700 sm:flex">
              <span className="size-1.5 rounded-full bg-emerald-500" /> Telemetry online
            </div>
            <button
              type="button"
              onClick={() => logout.mutate()}
              className="rounded-lg border border-slate-200 bg-white px-3 py-2 text-xs font-medium text-slate-600 hover:bg-slate-50"
            >
              Sign out
            </button>
          </div>
        </div>
      </header>

      <div className="mx-auto grid max-w-[1440px] lg:grid-cols-[220px_1fr]">
        <aside className="hidden min-h-[calc(100vh-4rem)] border-r border-slate-200 bg-white px-4 py-6 lg:block">
          <p className="px-3 text-[10px] font-semibold uppercase tracking-[.16em] text-slate-400">
            Workspace
          </p>
          <nav className="mt-3 space-y-1 text-sm">
            <a
              className="flex items-center gap-3 rounded-lg bg-orange-50 px-3 py-2.5 font-medium text-orange-700"
              href="#tasks"
            >
              <span>▣</span> Tasks
            </a>
            {session.role === "admin" && (
              <>
                <a
                  className="flex items-center gap-3 rounded-lg px-3 py-2.5 text-slate-500 hover:bg-slate-50 hover:text-slate-800"
                  href="/jaeger/monitor"
                >
                  <span>⌁</span> Jaeger
                </a>
                <a
                  className="flex items-center gap-3 rounded-lg px-3 py-2.5 text-slate-500 hover:bg-slate-50 hover:text-slate-800"
                  href="/prometheus/"
                >
                  <span>◫</span> Prometheus
                </a>
                <a
                  className="flex items-center gap-3 rounded-lg px-3 py-2.5 text-slate-500 hover:bg-slate-50 hover:text-slate-800"
                  href="/grafana/"
                >
                  <span>◩</span> Grafana
                </a>
              </>
            )}
            <span className="flex items-center gap-3 px-3 py-2.5 text-slate-400">
              <span>⚙</span> Settings
            </span>
          </nav>
          <div className="mt-10 rounded-lg border border-slate-200 bg-slate-50 p-3">
            <p className="text-[10px] font-semibold uppercase tracking-wider text-slate-400">
              Signed in
            </p>
            <p className="mt-2 truncate text-xs font-medium text-slate-700">
              {session.email ?? `Account #${session.id}`}
            </p>
            <p className="mt-1 text-[10px] text-slate-400">Private workspace</p>
          </div>
        </aside>

        <main id="tasks" className="min-w-0 px-4 py-7 sm:px-6 lg:px-10 lg:py-10">
          <div className="flex flex-col justify-between gap-3 sm:flex-row sm:items-end">
            <div>
              <p className="text-xs font-medium text-orange-600">Task management</p>
              <h1 className="mt-1 text-3xl font-semibold tracking-[-.035em] text-slate-950">
                Overview
              </h1>
              <p className="mt-2 text-sm text-slate-500">
                Your private queue, synchronized and observable.
              </p>
            </div>
            <p className="text-xs text-slate-400">Last synced · just now</p>
          </div>

          <section className="mt-7 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <Metric label="Total tasks" value={items.length} note="All tasks in your workspace" />
            <Metric label="Open" value={items.length - completed} note="Still waiting for action" />
            <Metric label="Completed" value={completed} note="Finished successfully" />
            <Metric
              label="Completion"
              value={`${items.length ? Math.round((completed / items.length) * 100) : 0}%`}
              note="Across your current queue"
            />
          </section>

          <section className="mt-6 overflow-hidden rounded-xl border border-slate-200 bg-white shadow-[0_1px_2px_rgba(15,23,42,.03)]">
            <div className="border-b border-slate-200 p-4 sm:p-5">
              <form onSubmit={addTodo} className="flex gap-2">
                <input
                  value={title}
                  onChange={(event) => setTitle(event.target.value)}
                  aria-label="New task title"
                  placeholder="What needs to get done?"
                  className="min-w-0 flex-1 rounded-lg border border-slate-300 px-3.5 py-2.5 text-sm outline-none transition focus:border-orange-500 focus:ring-3 focus:ring-orange-100"
                />
                <button
                  type="submit"
                  disabled={create.isPending || !title.trim()}
                  className="rounded-lg bg-orange-500 px-4 py-2.5 text-sm font-semibold text-white hover:bg-orange-600 disabled:opacity-50"
                >
                  Add task
                </button>
              </form>
              {create.error && (
                <p role="alert" className="mt-2 text-xs text-red-600">
                  Could not add the task. Please try again.
                </p>
              )}
            </div>
            <div className="flex items-center justify-between border-b border-slate-100 px-4 py-3 sm:px-5">
              <h2 className="text-sm font-semibold text-slate-800">Tasks</h2>
              <div className="flex rounded-md bg-slate-100 p-0.5">
                {(["all", "open", "done"] as TodoFilter[]).map((item) => (
                  <button
                    key={item}
                    type="button"
                    onClick={() => setFilter(item)}
                    className={`rounded px-2.5 py-1 text-xs font-medium capitalize ${filter === item ? "bg-white text-slate-800 shadow-sm" : "text-slate-500"}`}
                  >
                    {item}
                  </button>
                ))}
              </div>
            </div>
            {todos.isPending ? (
              <div className="grid place-items-center px-4 py-16 text-sm text-slate-400">
                Loading your workspace…
              </div>
            ) : todos.isError ? (
              <div className="grid place-items-center px-4 py-16 text-center">
                <p className="text-sm text-red-600">Unable to load tasks.</p>
                <button
                  type="button"
                  onClick={() => todos.refetch()}
                  className="mt-3 text-xs font-semibold text-orange-600"
                >
                  Try again
                </button>
              </div>
            ) : visibleItems.length ? (
              <ul>
                {visibleItems.map((todo) => (
                  <TodoRow key={todo.id} todo={todo} />
                ))}
              </ul>
            ) : (
              <div className="grid place-items-center px-4 py-16 text-center">
                <span className="grid size-10 place-items-center rounded-full bg-orange-50 text-orange-500">
                  ✓
                </span>
                <p className="mt-3 text-sm font-medium text-slate-700">No tasks here</p>
                <p className="mt-1 text-xs text-slate-400">Add a task or choose another filter.</p>
              </div>
            )}
          </section>
        </main>
      </div>
    </div>
  );
}

export default function App() {
  const queryClient = useQueryClient();
  useEffect(
    () =>
      setUnauthorizedHandler(() => {
        queryClient.setQueryData(sessionKey, null);
        queryClient.removeQueries({ queryKey: todosKey });
      }),
    [queryClient],
  );
  const session = useQuery({
    queryKey: sessionKey,
    queryFn: async () => {
      try {
        return await api.me();
      } catch (error) {
        if (error instanceof ApiError && error.status === 401) return null;
        throw error;
      }
    },
    retry: false,
  });

  if (session.isPending)
    return (
      <div className="grid min-h-screen place-items-center bg-[#f7f7f5] text-sm text-slate-500">
        Opening workspace…
      </div>
    );
  if (session.isError)
    return (
      <div className="grid min-h-screen place-items-center bg-[#f7f7f5] text-sm text-red-600">
        The service is unavailable.
      </div>
    );
  return (
    <Routes>
      <Route
        path="/"
        element={session.data ? <Navigate to="/dashboard/" replace /> : <AuthScreen />}
      />
      <Route
        path="/dashboard/*"
        element={session.data ? <Dashboard session={session.data} /> : <Navigate to="/" replace />}
      />
      <Route path="*" element={<Navigate to={session.data ? "/dashboard/" : "/"} replace />} />
    </Routes>
  );
}
