import { runSteps } from "./run-steps.mjs";
import { groups } from "./steps.mjs";

const name = process.argv[2];
const steps = groups[name];
if (!steps) {
  console.error(`Grupo desconhecido: ${name}. Use um de: ${Object.keys(groups).join(", ")}`);
  process.exit(2);
}
process.exit(await runSteps(steps));
