export function toErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message || String(error);
  if (typeof error === "string") return error;
  if (error == null) return "unknown";
  if (typeof error === "object" && "message" in error && typeof (error as { message?: unknown }).message === "string") {
    return (error as { message: string }).message;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

export function formatI18nError(
  translate: (key: string, params?: Record<string, unknown>) => string,
  key: string,
  error: unknown,
): string {
  return translate(key, { err: toErrorMessage(error) });
}
