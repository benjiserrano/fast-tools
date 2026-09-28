import { useMemo } from "react";

import type { Tool } from "../lib/types";
import { CATEGORY_LABEL, CATEGORY_ORDER } from "../lib/types";

export function Welcome({
  tools,
  version,
}: {
  tools: Tool[];
  version: string | null;
}) {
  const counts = useMemo(() => {
    const ready = tools.filter((tool) => tool.status === "ready").length;
    return { ready, total: tools.length };
  }, [tools]);

  return (
    <div className="flex h-full flex-col items-center justify-center p-10">
      <h1 className="text-2xl font-semibold tracking-tight text-mist-50">
        Fast tools
      </h1>
      <p className="mt-2 max-w-md text-center text-sm text-mist-400">
        Utilidades de desarrollo que funcionan en local. Nada de lo que pegues
        aquí sale de esta máquina.
      </p>

      <p className="mt-6 text-sm text-mist-500">
        Pulsa{" "}
        <kbd className="rounded border border-ink-700 bg-ink-900 px-1.5 py-0.5 font-mono text-xs text-mist-400">
          Ctrl Alt T
        </kbd>{" "}
        para buscar una herramienta
      </p>

      <div className="mt-8 grid w-full max-w-lg grid-cols-5 gap-2">
        {CATEGORY_ORDER.map((category) => (
          <div
            key={category}
            className="rounded-lg border border-ink-800 bg-ink-900 px-2 py-3 text-center"
          >
            <p className="font-mono text-lg text-mist-200">
              {tools.filter((tool) => tool.category === category).length}
            </p>
            <p className="mt-1 text-[10px] leading-tight text-mist-500">
              {CATEGORY_LABEL[category]}
            </p>
          </div>
        ))}
      </div>

      <p className="mt-6 font-mono text-xs text-mist-500">
        {counts.ready} de {counts.total} herramientas disponibles
        {version ? ` · v${version}` : ""}
      </p>
    </div>
  );
}
