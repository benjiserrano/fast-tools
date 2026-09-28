import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useMemo, useState } from "react";

import { ToolShell } from "../../components/Panels";
import { Banner, Button, NumberField, Select, Toggle } from "../../components/ui";
import { baseName } from "../../lib/convert";
import { ipcErrorMessage } from "../../lib/ipc";
import { navigateToTool } from "../../lib/navigation";
import type {
  DocFormatInfo,
  EngineStatus,
  FileOutcome,
  MediaFormatInfo,
  MediaOptions,
} from "./helpers";
import {
  convertDocuments,
  convertMediaFiles,
  DEFAULT_MEDIA_OPTIONS,
  listDocFormats,
  listEngines,
  listMediaFormats,
} from "./helpers";

const MEDIA_EXTENSIONS = [
  "mp4",
  "mkv",
  "webm",
  "mov",
  "avi",
  "mp3",
  "wav",
  "flac",
  "ogg",
  "m4a",
  "aac",
];

export function DocPanel() {
  return <ExternalBatch mode="doc" />;
}

export function MediaPanel() {
  return <ExternalBatch mode="media" />;
}

function ExternalBatch({ mode }: { mode: "doc" | "media" }) {
  const [engines, setEngines] = useState<EngineStatus[]>([]);
  const [docFormats, setDocFormats] = useState<DocFormatInfo[]>([]);
  const [mediaFormats, setMediaFormats] = useState<MediaFormatInfo[]>([]);
  const [paths, setPaths] = useState<string[]>([]);
  const [to, setTo] = useState(mode === "doc" ? "pdf" : "mp4");
  const [outputDir, setOutputDir] = useState<string | null>(null);
  const [overwrite, setOverwrite] = useState(false);
  const [options, setOptions] = useState<MediaOptions>(DEFAULT_MEDIA_OPTIONS);
  const [outcomes, setOutcomes] = useState<FileOutcome[] | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);

  useEffect(() => {
    listEngines().then(setEngines).catch(() => setEngines([]));
    if (mode === "doc") {
      listDocFormats().then(setDocFormats).catch(() => setDocFormats([]));
    } else {
      listMediaFormats().then(setMediaFormats).catch(() => setMediaFormats([]));
    }
  }, [mode]);

  const addPaths = useCallback((incoming: string[]) => {
    setOutcomes(null);
    setPaths((previous) => [...new Set([...previous, ...incoming])]);
  }, []);

  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        setDragging(true);
        return;
      }
      setDragging(false);
      if (event.payload.type === "drop") addPaths(event.payload.paths);
    });
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, [addPaths]);

  // Qué motor hace falta para el destino elegido, y si está.
  const requiredEngineId =
    mode === "doc"
      ? (docFormats.find((format) => format.id === to)?.engine ?? "pandoc")
      : "ffmpeg";
  const requiredEngine = engines.find((engine) => engine.id === requiredEngineId);
  const engineReady = requiredEngine?.available ?? false;

  const readableExtensions = useMemo(
    () =>
      mode === "doc"
        ? docFormats.filter((format) => format.canRead).flatMap((f) => f.extensions)
        : MEDIA_EXTENSIONS,
    [mode, docFormats],
  );

  const handleAdd = useCallback(async () => {
    const selected = await open({
      multiple: true,
      filters: [
        {
          name: mode === "doc" ? "Documentos" : "Audio y vídeo",
          extensions: readableExtensions,
        },
      ],
    });
    if (Array.isArray(selected)) addPaths(selected);
  }, [mode, readableExtensions, addPaths]);

  const handleRun = useCallback(async () => {
    if (paths.length === 0) return;
    setRunning(true);
    setError(null);
    try {
      setOutcomes(
        mode === "doc"
          ? await convertDocuments(paths, to, outputDir, overwrite)
          : await convertMediaFiles(paths, to, outputDir, overwrite, options),
      );
    } catch (cause) {
      setError(ipcErrorMessage(cause));
    } finally {
      setRunning(false);
    }
  }, [mode, paths, to, outputDir, overwrite, options]);

  const selectedDoc = docFormats.find((format) => format.id === to);
  const selectedMedia = mediaFormats.find((format) => format.id === to);
  const failed = outcomes?.filter((outcome) => outcome.error).length ?? 0;
  const succeeded = (outcomes?.length ?? 0) - failed;

  return (
    <ToolShell
      error={error}
      toolbar={
        <>
          <Select
            label="Convertir a"
            value={to}
            onChange={(event) => {
              setTo(event.target.value);
              setOutcomes(null);
            }}
          >
            {mode === "doc"
              ? docFormats
                  .filter((format) => format.canWrite)
                  .map((format) => (
                    <option key={format.id} value={format.id}>
                      {format.label}
                    </option>
                  ))
              : mediaFormats.map((format) => (
                  <option key={format.id} value={format.id}>
                    {format.label}
                  </option>
                ))}
          </Select>

          {mode === "media" && (
            <>
              {!selectedMedia?.audioOnly && (
                <NumberField
                  label="Calidad vídeo"
                  value={options.videoQuality}
                  min={0}
                  max={51}
                  onChange={(videoQuality) =>
                    setOptions((previous) => ({ ...previous, videoQuality }))
                  }
                />
              )}
              <NumberField
                label="Audio kbit/s"
                value={options.audioBitrate}
                min={32}
                max={512}
                onChange={(audioBitrate) =>
                  setOptions((previous) => ({ ...previous, audioBitrate }))
                }
              />
            </>
          )}

          <Button
            onClick={async () => {
              const selected = await open({ directory: true, multiple: false });
              if (typeof selected === "string") setOutputDir(selected);
            }}
          >
            {outputDir ? `Salida: ${baseName(outputDir)}` : "Salida: junto al original"}
          </Button>
          {outputDir && <Button onClick={() => setOutputDir(null)}>✕</Button>}

          <Toggle label="Sobrescribir" checked={overwrite} onChange={setOverwrite} />

          <div className="ml-auto flex items-center gap-2">
            <Button onClick={handleAdd}>Añadir archivos</Button>
            {paths.length > 0 && (
              <Button
                onClick={() => {
                  setPaths([]);
                  setOutcomes(null);
                }}
              >
                Vaciar
              </Button>
            )}
            <Button
              variant="primary"
              onClick={handleRun}
              disabled={paths.length === 0 || running || !engineReady}
            >
              {running ? "Convirtiendo…" : `Convertir ${paths.length || ""}`}
            </Button>
          </div>
        </>
      }
    >
      <div className="flex h-full flex-col gap-3 p-4">
        {!engineReady && requiredEngine && (
          <Banner tone="warn">
            <div className="flex flex-wrap items-center gap-3">
              <span>
                Esto necesita <strong>{requiredEngine.name}</strong>, que no está
                instalado. {requiredEngine.enables}
              </span>
              <Button onClick={() => navigateToTool("engines")}>
                Ir a motores
              </Button>
            </div>
          </Banner>
        )}

        {mode === "media" && options.videoQuality < 18 && !selectedMedia?.audioOnly && (
          <Banner tone="info">
            Por debajo de 18 la diferencia de calidad deja de notarse y el archivo
            crece mucho. 23 es el valor por defecto de FFmpeg.
          </Banner>
        )}

        {selectedDoc?.limitation && <Banner tone="info">{selectedDoc.limitation}</Banner>}

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
              <p className="text-sm text-mist-400">Arrastra archivos aquí</p>
              <p className="text-xs text-mist-500">
                {mode === "doc"
                  ? "Markdown, HTML, DOCX, ODT, RST, LaTeX y EPUB"
                  : "MP4, MKV, WebM, MOV, MP3, WAV, FLAC y OGG"}
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
    </ToolShell>
  );
}
