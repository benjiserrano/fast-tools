import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import { appInfo, inTauri, ipcErrorMessage, listTools } from "../lib/ipc";
import { onNavigateToTool } from "../lib/navigation";
import type { AppInfo, Tool } from "../lib/types";
import { CommandPalette } from "./CommandPalette";
import { Sidebar } from "./Sidebar";
import { StatusBar } from "./StatusBar";
import { TitleBar } from "./TitleBar";
import { ToolView } from "./ToolView";
import { Welcome } from "./Welcome";

const OPEN_SEARCH_EVENT = "open-search";

export function App() {
  const [tools, setTools] = useState<Tool[]>([]);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [paletteOpen, setPaletteOpen] = useState(false);

  useEffect(() => {
    if (!inTauri()) {
      setError(
        "Esta interfaz necesita la ventana de Fast tools. Ejecuta «pnpm dev» en vez de abrir Vite en el navegador.",
      );
      return;
    }
    Promise.all([listTools(), appInfo()])
      .then(([loadedTools, loadedInfo]) => {
        setTools(loadedTools);
        setInfo(loadedInfo);
      })
      .catch((cause) => setError(ipcErrorMessage(cause)));
  }, []);

  // El backend restaura la ventana y emite este evento desde Ctrl+Alt+T o la bandeja.
  useEffect(() => {
    if (!inTauri()) return;

    let active = true;
    let stopListening: (() => void) | undefined;

    void listen(OPEN_SEARCH_EVENT, () => setPaletteOpen(true))
      .then((unlisten) => {
        if (active) stopListening = unlisten;
        else unlisten();
      })
      .catch(() => {
        // Vitest/browser preview lack Tauri event IPC; app remains usable.
      });

    return () => {
      active = false;
      stopListening?.();
    };
  }, []);

  // Ctrl+K abre y cierra la paleta desde cualquier punto de la app.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen((open) => !open);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const selected = useMemo(
    () => tools.find((tool) => tool.id === selectedId) ?? null,
    [tools, selectedId],
  );

  const select = useCallback((id: string) => {
    setSelectedId(id);
    setPaletteOpen(false);
  }, []);

  // Los paneles pueden pedir saltar a otra herramienta; por ejemplo, el de
  // documentos manda a la pantalla de motores cuando falta Pandoc.
  useEffect(() => onNavigateToTool(select), [select]);

  if (error) {
    return (
      <div className="flex h-full items-center justify-center p-8">
        <div className="selectable max-w-lg rounded-lg border border-danger-400/40 bg-danger-400/10 p-6 text-sm">
          <p className="mb-2 font-semibold text-danger-400">
            No se pudo iniciar la interfaz
          </p>
          <p className="text-mist-400">{error}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <TitleBar />

      <div className="flex min-h-0 flex-1">
        <Sidebar
          tools={tools}
          selectedId={selectedId}
          onSelect={select}
          onOpenPalette={() => setPaletteOpen(true)}
        />
        <main className="min-w-0 flex-1 overflow-y-auto bg-ink-950">
          {selected ? (
            <ToolView tool={selected} />
          ) : (
            <Welcome tools={tools} version={info?.version ?? null} />
          )}
        </main>
      </div>

      <StatusBar info={info} toolCount={tools.length} />

      <CommandPalette
        open={paletteOpen}
        tools={tools}
        onOpenChange={setPaletteOpen}
        onSelect={select}
      />
    </div>
  );
}
