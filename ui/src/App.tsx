import { useEffect, useState } from "react";
import { getCoreVersion } from "./core/api";

// Placeholder shell. Real screens come with F08; visible text will move to translation keys (T-08.07).
export function App() {
  const [version, setVersion] = useState<string>();

  useEffect(() => {
    getCoreVersion().then(setVersion, () => setVersion(undefined));
  }, []);

  return (
    <main>
      <h1>Limpador</h1>
      {version && <p data-testid="core-version">v{version}</p>}
    </main>
  );
}
