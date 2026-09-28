import { useState } from "react";

import { ipcErrorMessage, revealDataDir } from "../lib/ipc";
import type { AppInfo } from "../lib/types";
import { STORAGE_LABEL } from "../lib/types";

export function StatusBar({
  info,
  toolCount,
}: {
  info: AppInfo | null;
  toolCount: number;
}) {
  const [revealError, setRevealError] = useState<string | null>(null);

  if (!info) {
    return (
      <footer className="h-7 shrink-0 border-t border-ink-800 bg-ink-900" />
    );
  }

  const { storage } = info;
  const degraded = storage.mode !== "portable";

  return (
    <footer className="flex h-7 shrink-0 items-center gap-3 border-t border-ink-800 bg-ink-900 px-3 text-[11px] text-mist-500">
      <span
        className={`rounded px-1.5 py-0.5 font-medium ${
          degraded
            ? "bg-warn-400/15 text-warn-400"
            : "bg-bolt-500/15 text-bolt-400"
        }`}
        title={storage.degradedReason ?? "Los datos se guardan junto al ejecutable"}
      >
        {STORAGE_LABEL[storage.mode]}
      </span>

      <button
        type="button"
        onClick={() => {
          setRevealError(null);
          revealDataDir().catch((cause) =>
            setRevealError(ipcErrorMessage(cause)),
          );
        }}
        title="Abrir la carpeta de datos en el explorador"
        className="selectable truncate font-mono transition-colors hover:text-mist-200"
      >
        {storage.root}
      </button>

      {degraded && storage.degradedReason && (
        <span className="truncate text-warn-400" title={storage.degradedReason}>
          {storage.degradedReason}
        </span>
      )}

      {revealError && (
        <span className="truncate text-danger-400">{revealError}</span>
      )}

      <span className="ml-auto shrink-0 font-mono">
        {toolCount} herramientas · v{info.version}
      </span>
    </footer>
  );
}
