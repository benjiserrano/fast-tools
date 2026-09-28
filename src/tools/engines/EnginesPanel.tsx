import { useCallback, useEffect, useState } from "react";

import { FieldRow, ToolShell } from "../../components/Panels";
import { Banner, Button } from "../../components/ui";
import type { EngineStatus, VerifyEngineResult } from "../../lib/engines";
import {
  formatEngineSize,
  installEngine,
  listEngines,
  onEngineProgress,
  removeEngine,
  SOURCE_LABEL,
  verifyEngine,
} from "./helpers";
import { ipcErrorMessage } from "../../lib/ipc";

export function EnginesPanel() {
  const [engines, setEngines] = useState<EngineStatus[]>([]);
  const [progress, setProgress] = useState<Record<string, number>>({});
  const [installing, setInstalling] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<EngineStatus | null>(null);
  const [verdicts, setVerdicts] = useState<Record<string, VerifyEngineResult>>({});
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(() => {
    listEngines()
      .then(setEngines)
      .catch((cause) => setError(ipcErrorMessage(cause)));
  }, []);

  useEffect(refresh, [refresh]);

  useEffect(() => {
    const unlisten = onEngineProgress((event) => {
      setProgress((previous) => ({
        ...previous,
        [event.engine]: event.total > 0 ? event.downloaded / event.total : 0,
      }));
    });
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, []);

  const confirmInstall = useCallback(async () => {
    if (!confirming) return;
    const id = confirming.id;
    setConfirming(null);
    setInstalling(id);
    setError(null);
    try {
      const updated = await installEngine(id);
      setEngines((previous) =>
        previous.map((engine) => (engine.id === id ? updated : engine)),
      );
    } catch (cause) {
      setError(ipcErrorMessage(cause));
    } finally {
      setInstalling(null);
      setProgress((previous) => {
        const next = { ...previous };
        delete next[id];
        return next;
      });
    }
  }, [confirming]);

  const handleRemove = useCallback(async (id: string) => {
    try {
      const updated = await removeEngine(id);
      setEngines((previous) =>
        previous.map((engine) => (engine.id === id ? updated : engine)),
      );
      setVerdicts((previous) => {
        const next = { ...previous };
        delete next[id];
        return next;
      });
    } catch (cause) {
      setError(ipcErrorMessage(cause));
    }
  }, []);

  const handleVerify = useCallback(async (id: string) => {
    try {
      const verdict = await verifyEngine(id);
      setVerdicts((previous) => ({ ...previous, [id]: verdict }));
    } catch (cause) {
      setError(ipcErrorMessage(cause));
    }
  }, []);

  return (
    <ToolShell
      error={error}
      toolbar={
        <>
          <span className="text-[11px] text-mist-500">
            Programas de terceros que fast-tools descarga bajo demanda. Nada se
            baja sin que lo pidas.
          </span>
          <div className="ml-auto">
            <Button onClick={refresh}>Actualizar</Button>
          </div>
        </>
      }
    >
      <div className="space-y-4 overflow-y-auto p-4">
        {engines.map((engine) => (
          <article
            key={engine.id}
            className="rounded-lg border border-ink-800 bg-ink-900"
          >
            <header className="flex flex-wrap items-center gap-3 border-b border-ink-800 px-4 py-3">
              <h2 className="font-semibold text-mist-50">{engine.name}</h2>
              <span
                className={`rounded px-1.5 py-0.5 text-[11px] font-medium ${
                  engine.available
                    ? "bg-bolt-500/15 text-bolt-400"
                    : "bg-ink-800 text-mist-500"
                }`}
              >
                {engine.available
                  ? engine.source
                    ? SOURCE_LABEL[engine.source]
                    : "Disponible"
                  : "No instalado"}
              </span>
              {engine.version && (
                <span className="font-mono text-[11px] text-mist-500">
                  v{engine.version}
                </span>
              )}

              <div className="ml-auto flex items-center gap-2">
                {engine.available && engine.source === "managed" && (
                  <>
                    <Button onClick={() => handleVerify(engine.id)}>
                      Verificar integridad
                    </Button>
                    <Button onClick={() => handleRemove(engine.id)}>Quitar</Button>
                  </>
                )}
                {!engine.available && engine.downloadable && (
                  <Button
                    variant="primary"
                    onClick={() => setConfirming(engine)}
                    disabled={installing !== null}
                  >
                    {installing === engine.id ? "Instalando…" : "Instalar"}
                  </Button>
                )}
              </div>
            </header>

            <div className="px-4 py-3 text-sm text-mist-400">
              <p>{engine.description}</p>
              <p className="mt-1 text-xs text-mist-500">
                Desbloquea: {engine.enables}
              </p>
            </div>

            {installing === engine.id && (
              <div className="px-4 pb-3">
                <div className="h-1.5 overflow-hidden rounded-full bg-ink-800">
                  <div
                    className="h-full bg-bolt-500 transition-[width] duration-200"
                    style={{
                      width: `${Math.round((progress[engine.id] ?? 0) * 100)}%`,
                    }}
                  />
                </div>
                <p className="mt-1 font-mono text-[11px] text-mist-500">
                  {Math.round((progress[engine.id] ?? 0) * 100)} % ·
                  verificando el hash al terminar
                </p>
              </div>
            )}

            {!engine.available && !engine.downloadable && engine.noDownloadReason && (
              <div className="px-4 pb-3">
                <Banner tone="info">{engine.noDownloadReason}</Banner>
              </div>
            )}

            {verdicts[engine.id] && (
              <div className="px-4 pb-3">
                <Banner tone={verdicts[engine.id]!.ok ? "info" : "error"}>
                  <span className="whitespace-pre-wrap">
                    {verdicts[engine.id]!.message}
                  </span>
                </Banner>
              </div>
            )}

            {engine.path && (
              <div className="border-t border-ink-800">
                <FieldRow label="Ruta" value={engine.path} tone="muted" />
                {engine.diskBytes !== null && engine.diskBytes > 0 && (
                  <FieldRow
                    label="Ocupa"
                    value={formatEngineSize(engine.diskBytes)}
                    tone="muted"
                  />
                )}
              </div>
            )}
          </article>
        ))}
      </div>

      {confirming && (
        <ConsentDialog
          engine={confirming}
          onCancel={() => setConfirming(null)}
          onAccept={confirmInstall}
        />
      )}
    </ToolShell>
  );
}

/**
 * Antes de descargar nada hay que enseñar qué se va a traer y de dónde.
 *
 * Es la regla de consentimiento explícito: el usuario ve el nombre, la versión,
 * el tamaño, el dominio de origen y la licencia antes de que salga una sola
 * petición a la red.
 */
function ConsentDialog({
  engine,
  onCancel,
  onAccept,
}: {
  engine: EngineStatus;
  onCancel: () => void;
  onAccept: () => void;
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-ink-950/70 p-6 backdrop-blur-sm">
      <div className="w-[min(34rem,92vw)] rounded-xl border border-ink-700 bg-ink-900 shadow-2xl">
        <header className="border-b border-ink-800 px-5 py-4">
          <h2 className="font-semibold text-mist-50">
            Descargar {engine.name}
          </h2>
          <p className="mt-1 text-xs text-mist-400">
            Es un programa de terceros. Se descargará y se comprobará su hash
            SHA-256 antes de ejecutar nada.
          </p>
        </header>

        <div className="border-b border-ink-800">
          <FieldRow label="Versión" value={engine.downloadVersion ?? "—"} />
          <FieldRow
            label="Tamaño"
            value={
              engine.downloadSize ? formatEngineSize(engine.downloadSize) : "—"
            }
          />
          <FieldRow label="Origen" value={engine.downloadOrigin ?? "—"} />
          <FieldRow label="Licencia" value={engine.license ?? "—"} tone="muted" />
        </div>

        <footer className="flex items-center justify-end gap-2 px-5 py-4">
          <Button onClick={onCancel}>Cancelar</Button>
          <Button variant="primary" onClick={onAccept}>
            Descargar e instalar
          </Button>
        </footer>
      </div>
    </div>
  );
}
