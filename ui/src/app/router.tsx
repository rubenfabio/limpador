// Screens and navigation (ADR-0020): code-based routes, in-memory history (no URL bar).
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import { OnboardingFlow } from "../onboarding/OnboardingFlow";
import { Painel } from "../screens/Painel";
import { Ranking } from "../screens/Ranking";
import { Resumo } from "../screens/Resumo";
import { Layout } from "./Layout";

const rootRoute = createRootRoute({ component: Layout });

const painelRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: Painel,
});

const rankingRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "ranking",
  component: Ranking,
});

const resumoRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "resumo",
  component: Resumo,
});

const configurarRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "configurar",
  component: OnboardingFlow,
});

export const routeTree = rootRoute.addChildren([
  painelRoute,
  rankingRoute,
  resumoRoute,
  configurarRoute,
]);

export function createAppRouter(initialPath = "/") {
  return createRouter({
    routeTree,
    history: createMemoryHistory({ initialEntries: [initialPath] }),
  });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof createAppRouter>;
  }
}
