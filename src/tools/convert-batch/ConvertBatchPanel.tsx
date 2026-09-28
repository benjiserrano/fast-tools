import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useCallback, useEffect, useMemo, useState } from "react";

import { Banner, Button, Select, Toggle, Toolbar } from "../../components/ui";
import type { FileOutcome, FormatId, FormatInfo } from "../../lib/convert";
import {
  baseName,
  convertFiles,
  DEFAULT_OPTS,
  listFormats,
} from "../../lib/convert";
import { ipcErrorMessage } from "../../lib/ipc";

export function ConvertBatchPanel() {
  const [formats, setFormats] = useState<FormatInfo[]>([]);
  const [paths, setPaths] = useState<string[]>([]);
  const [to, setTo] = useState<FormatId>("json");
  const [outputDir, setOutputDir] = useState<string | null>(null);
  const [overwrite, setOverwrite] = useState(false);
  const [outcomes, setOutcomes] = useState<FileOutcome[] | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);

  useEffect(() => {
    listFormats().then(setFormats).catch(() => setFormats([]));
  }, []);

  const knownExtensions = useMemo(
    () => new Set(formats.flatMap((format) => format.extensions)),
    [formats],
  );

  const addPaths = useCallback(
    (incoming: string[]) => {
      setOutcomes(null);
      setPaths((previous) => {
        const merged = new Set(previous);
        for (const path of incoming) merged.add(path);
        return [...merged];
      });
      // Avisa de lo que se ha añadido pero no se va a poder convertir, en vez
      // de descartarlo en silencio y dejar al usuario esperando un resultado.
      const unknown = incoming.filter((path) => {
        const extension = path.split(".").pop()?.toLowerCase() ?? "";
        return !knownExtensions.has(extension);
      });
      setError(
        unknown.length > 0
          ? `Sin formato reconocido: ${unknown.map(baseName).join(", ")}`
          : null,
      );
    },
    [knownExtensions],
  );

  // Arrastrar y soltar sobre la ventana.
  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        setDragging(true);
      } else if (event.payload.type === "drop") {
        setDragging(false);
        addPaths(event.payload.paths);
      } else {
        setDragging(false);
      }
    });
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, [addPaths]);

  const handleAdd = useCallback(async () => {
    const selected = await open({
      multiple: true,
      filters: [
        {
          name: "Datos estructurados",
          extensions: formats.flatMap((format) => format.extensions),
        },
      ],
    });
    if (Array.isArray(selected)) addPaths(selected);
  }, [formats, addPaths]);

  const handlePickFolder = useCallback(async () => {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") setOutputDir(selected);
  }, []);

  const handleConvert = useCallback(async () => {
    if (paths.length === 0) return;
    setRunning(true);
    setError(null);
    try {
      setOutcomes(await convertFiles(paths, to, outputDir, overwrite, DEFAULT_OPTS));
    } catch (cause) {
      setError(ipcErrorMessage(cause));
    } finally {
      setRunning(false);
    }
  }, [paths, to, outputDir, overwrite]);

  const failed = outcomes?.filter((outcome) => outcome.error).length ?? 0;
  const succeeded = (outcomes?.length ?? 0) - failed;

  return (
    <div className="flex h-full flex-col">
      <Toolbar>
        <Select
          label="Convertir a"
          value={to}
          onChange={(event) => {
            setTo(event.target.value as FormatId);
            setOutcomes(null);
          }}
        >
          {formats.map((format) => (
            <option key={format.id} value={format.id}>
              {format.label}
            </option>
          ))}
        </Select>

        <Button onClick={handlePickFolder} title="Dónde dejar los resultados">
          {outputDir ? `Salida: ${baseName(outputDir)}` : "Salida: junto al original"}
        </Button>
        {outputDir && (
          <Button onClick={() => setOutputDir(null)} title="Volver al valor por defecto">
            ✕
          </Button>
        )}

        <Toggle
          label="Sobrescribir"
          title="Sin esto, los archivos que ya existan se saltan y se informa de ello"
          checked={overwrite}
          onChange={setOverwrite}
        />

        <div className="ml-auto flex items-center gap-2">
          <Button onClick={handleAdd}>Añadir archivos</Button>
          {paths.length > 0 && (
            <Button
              onClick={() => {
                setPaths([]);
                setOutcomes(null);
                setError(null);
              }}
            >
              Vaciar
            </Button>
          )}
          <Button
            variant="primary"
            onClick={handleConvert}
            disabled={paths.length === 0 || running}
          >
            {running ? "Convirtiendo…" : `Convertir ${paths.length || ""}`}
          </Button>
        </div>
      </Toolbar>

      <div className="flex min-h-0 flex-1 flex-col gap-3 p-4">
        {error && <Banner tone="warn">{error}</Banner>}

        {outcomes && (
          <Banner tone={failed > 0 ? "warn" : "info"}>
            {succeeded} convertido{succeeded === 1 ? "" : "s"}
            {failed > 0 && ` · ${failed} con error`}
          </Banner>
        )}

        <div
          className={`flex min-h-0 flex-1 flex-col rounded-lg border border-dashed transition-colors ${
            dragging ? "border-bolt-500 bg-bolt-500/5" : "border-ink-700"
          }`}
        >
          {paths.length === 0 ? (
            <div className="flex flex-1 flex-col items-center justify-center gap-2 text-center">
              <p className="text-sm text-mist-400">
                Arrastra archivos o una carpeta aquí
              </p>
              <p className="text-xs text-mist-500">
                El formato de cada archivo se detecta por su extensión
              </p>
            </div>
          ) : (
            <ul className="min-h-0 flex-1 overflow-y-auto p-2">
              {paths.map((path) => {
                const outcome = outcomes?.find((item) => item.source === path);
                return (
                  <li
                    key={path}
                    className="flex items-center gap-3 rounded-md px-2 py-1.5 text-xs hover:bg-ink-900"
                  >
                    <span className="min-w-0 flex-1 truncate text-mist-200" title={path}>
                      {baseName(path)}
                    </span>
                    {outcome?.error && (
                      <span
                        className="selectable max-w-[55%] truncate text-danger-400"
                        title={outcome.error}
                      >
                        {outcome.error}
                      </span>
                    )}
                    {outcome?.output && (
                      <span className="truncate text-bolt-400" title={outcome.output}>
                        → {baseName(outcome.output)}
                      </span>
                    )}
                    {!outcome && (
                      <button
                        type="button"
                        onClick={() =>
                          setPaths((previous) => previous.filter((item) => item !== path))
                        }
                        className="text-mist-500 hover:text-danger-400"
                        title="Quitar de la lista"
                      >
                        ✕
                      </button>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
}
