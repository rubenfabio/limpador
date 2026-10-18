import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { useState } from "react";
import { SelectionProvider } from "../selection/Selection";
import { createAppRouter } from "./router";

/** Everything is local IPC: failures are real, so they are not retried. */
export function createQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false, staleTime: Number.POSITIVE_INFINITY } },
  });
}

/** The whole app: data, selection and screens. `initialPath` exists for tests. */
export function AppRoot({ initialPath = "/" }: { initialPath?: string }) {
  const [queryClient] = useState(createQueryClient);
  const [router] = useState(() => createAppRouter(initialPath));
  return (
    <QueryClientProvider client={queryClient}>
      <SelectionProvider>
        <RouterProvider router={router} />
      </SelectionProvider>
    </QueryClientProvider>
  );
}
