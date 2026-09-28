import { Suspense } from "react";

import type { Tool } from "../lib/types";
import { CATEGORY_LABEL } from "../lib/types";
import { PANELS } from "../tools/panels";

export function ToolView({ tool }: { tool: Tool }) {
  const Panel = PANELS[tool.id];

  return (
    <div className="flex h-full flex-col">
      <header className="border-b border-ink-800 px-6 py-4">
        <div className="flex items-baseline gap-3">
          <h1 className="text-lg font-semibold text-mist-50">{tool.name}</h1>
          <span className="text-xs uppercase tracking-wider text-mist-500">
            {CATEGORY_LABEL[tool.category]}
          </span>
        </div>
        <p className="mt-1 text-sm text-mist-400">{tool.summary}</p>
      </header>

      <div className="min-h-0 flex-1">
        {Panel ? (
          <Suspense fallback={<Loading />}>
            <Panel tool={tool} />
          </Suspense>
        ) : (
          <NotYetBuilt tool={tool} />
        )}
      </div>
    </div>
  );
}

function Loading() {
  return (
    <div className="flex h-full items-center justify-center">
      <p className="text-xs text-mist-500">Cargando…</p>
    </div>
  );
}

function NotYetBuilt({ tool }: { tool: Tool }) {
  return (
    <div className="flex h-full items-center justify-center p-8">
      <div className="max-w-md text-center">
        <p className="font-mono text-3xl text-ink-600">Fase {tool.phase}</p>
        <p className="mt-3 text-sm text-mist-400">
          Esta herramienta todavía no está implementada.
        </p>
        <p className="mt-1 text-xs text-mist-500">
          Se ejecutará en{" "}
          {tool.runtime === "native"
            ? "el núcleo Rust"
            : "el proceso de la interfaz"}
          .
        </p>
      </div>
    </div>
  );
}
