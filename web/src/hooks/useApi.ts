import { useCallback, useRef, useState } from "react";
import { useSetAtom } from "jotai";
import { addToastAtom } from "../atoms/ui";
import { getApiUrl, isConfigReady } from "../config";

interface UseApiOptions {
  /** Show toast on error (default: true) */
  showErrorToast?: boolean;
}

interface UseApiReturn<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
  refetch: () => Promise<T | null>;
}

function getBaseUrl(): string {
  if (isConfigReady()) {
    try {
      return getApiUrl();
    } catch {
      // fallback
    }
  }
  return "";
}

/**
 * Custom hook wrapping fetch with error handling and toast notifications.
 *
 * @param path - API path (e.g. "/api/sessions")
 * @param init - Optional RequestInit (method, body, headers, etc.)
 * @param options - Hook options
 */
export function useApi<T = unknown>(
  path: string,
  init?: RequestInit,
  options?: UseApiOptions,
): UseApiReturn<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const addToast = useSetAtom(addToastAtom);
  const showErrorToast = options?.showErrorToast ?? true;

  // Stable reference for init to avoid re-creating refetch
  const initRef = useRef(init);
  initRef.current = init;

  const refetch = useCallback(async (): Promise<T | null> => {
    setLoading(true);
    setError(null);

    try {
      const url = `${getBaseUrl()}${path}`;
      const response = await fetch(url, initRef.current);

      if (!response.ok) {
        let message: string;
        try {
          const body = await response.json();
          message = body.error ?? body.message ?? response.statusText;
        } catch {
          message = response.statusText || `HTTP ${response.status}`;
        }

        const errorMsg =
          response.status >= 500
            ? `Server error: ${message}`
            : `Request failed: ${message}`;

        setError(errorMsg);
        if (showErrorToast) {
          addToast({
            type: "error",
            title: `API Error (${response.status})`,
            message: errorMsg,
          });
        }
        setLoading(false);
        return null;
      }

      const result = (await response.json()) as T;
      setData(result);
      setLoading(false);
      return result;
    } catch (err) {
      const message =
        err instanceof TypeError
          ? "Network error: unable to reach server"
          : err instanceof Error
            ? err.message
            : "Unknown error";

      setError(message);
      if (showErrorToast) {
        addToast({
          type: "error",
          title: "Network Error",
          message,
        });
      }
      setLoading(false);
      return null;
    }
  }, [path, showErrorToast, addToast]);

  return { data, loading, error, refetch };
}
