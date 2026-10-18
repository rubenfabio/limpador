import { render } from "@testing-library/react";
import { AppRoot } from "../app/Providers";
import { type Call, simulateCore } from "./core-mock";

/** Renders the whole app at `path` with a simulated core; returns the calls made. */
export function renderApp(path: string, responses: Record<string, unknown>): Call[] {
  const calls = simulateCore({ core_version: "0.1.0", ...responses });
  render(<AppRoot initialPath={path} />);
  return calls;
}
