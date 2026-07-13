const TODO_ID_ROUTE = /^\/api\/todos\/[^/]+$/;

/** Normalize API paths so Monitor/spanmetrics aggregate by route, not by resource id. */
export function normalizeApiRoute(path: string): string {
  const pathname = path.split("?")[0] ?? path;
  if (TODO_ID_ROUTE.test(pathname)) return "/api/todos/{id}";
  return pathname;
}
