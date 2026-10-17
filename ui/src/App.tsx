import { useEffect, useState } from "react";
import { getCoreVersion } from "./core/api";

// Placeholder shell. Real screens come with F08; visible text will move to translation keys (T-08.07).
export function App() {
  const [version, setVersion] = useState<string>();

  useEffect(
    () => {
      getCoreVersion()
        .then(setVersion)
        .catch((error: unknown) => console.error("Falha ao consultar a versão do núcleo", error));
    },
    // Stryker disable next-line ArrayDeclaration: qualquer lista constante roda o efeito uma vez só, então o mutante é equivalente
    [],
  );

  return (
    <main>
      <h1>Limpador</h1>
      {version && <p data-testid="core-version">v{version}</p>}
    </main>
  );
}
