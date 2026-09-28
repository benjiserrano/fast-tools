import { save } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useState } from "react";

import { CopyButton, ToolShell } from "../../components/Panels";
import { Banner, Button, NumberField, Select, TextField } from "../../components/ui";
import type { CorrectionId, CorrectionInfo, QrOptions, QrResult } from "../../lib/images";
import {
  DEFAULT_QR,
  formatBytes,
  generateQr,
  qrCorrectionLevels,
  saveQr,
} from "../../lib/images";
import { ipcErrorMessage } from "../../lib/ipc";

export function QrPanel() {
  const [content, setContent] = useState("https://fast-tools.test");
  const [options, setOptions] = useState<QrOptions>(DEFAULT_QR);
  const [levels, setLevels] = useState<CorrectionInfo[]>([]);
  const [result, setResult] = useState<QrResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    qrCorrectionLevels()
      .then(setLevels)
      .catch(() => setLevels([]));
  }, []);

  useEffect(() => {
    if (!content) {
      setResult(null);
      setError(null);
      return;
    }
    let cancelled = false;
    generateQr(content, options)
      .then((value) => {
        if (cancelled) return;
        setResult(value);
        setError(null);
      })
      .catch((cause) => {
        if (cancelled) return;
        setResult(null);
        setError(ipcErrorMessage(cause));
      });
    return () => {
      cancelled = true;
    };
  }, [content, options]);

  const set = <K extends keyof QrOptions>(key: K, value: QrOptions[K]) =>
    setOptions((previous) => ({ ...previous, [key]: value }));

  const handleSave = useCallback(
    async (extension: "png" | "svg") => {
      try {
        const destination = await save({
          defaultPath: `codigo-qr.${extension}`,
          filters: [{ name: extension.toUpperCase(), extensions: [extension] }],
        });
        if (!destination) return;
        await saveQr(content, options, destination, true);
        setNotice(`Guardado en ${destination}`);
        setTimeout(() => setNotice(null), 3000);
      } catch (cause) {
        setError(ipcErrorMessage(cause));
      }
    },
    [content, options],
  );

  const level = levels.find((candidate) => candidate.id === options.correction);
  const lowContrastWarning =
    options.foreground.toLowerCase() === options.background.toLowerCase();

  return (
    <ToolShell
      error={error}
      notice={notice ?? level?.note ?? null}
      toolbar={
        <>
          <Select
            label="Corrección"
            value={options.correction}
            onChange={(event) =>
              set("correction", event.target.value as CorrectionId)
            }
          >
            {levels.map((candidate) => (
              <option key={candidate.id} value={candidate.id}>
                {candidate.label}
              </option>
            ))}
          </Select>
          <NumberField
            label="Tamaño PNG"
            value={options.size}
            min={64}
            max={4096}
            onChange={(size) => set("size", size)}
          />
          <NumberField
            label="Margen"
            value={options.margin}
            min={0}
            max={16}
            onChange={(margin) => set("margin", margin)}
          />
          <TextField
            label="Color"
            width="w-20"
            value={options.foreground}
            onChange={(value) => set("foreground", value)}
          />
          <TextField
            label="Fondo"
            width="w-20"
            value={options.background}
            onChange={(value) => set("background", value)}
          />
          <div className="ml-auto flex items-center gap-2">
            <CopyButton text={result?.svg ?? ""} label="Copiar SVG" />
            <Button onClick={() => handleSave("svg")} disabled={!result}>
              Guardar SVG
            </Button>
            <Button
              variant="primary"
              onClick={() => handleSave("png")}
              disabled={!result}
            >
              Guardar PNG
            </Button>
          </div>
        </>
      }
    >
      <div className="grid h-full grid-cols-2 gap-4 p-4">
        <div className="flex min-h-0 flex-col gap-3">
          <label className="flex min-h-0 flex-1 flex-col gap-2">
            <span className="text-[11px] uppercase tracking-wider text-mist-500">
              Contenido
            </span>
            <textarea
              value={content}
              onChange={(event) => setContent(event.target.value)}
              placeholder="Una URL, un texto, datos de contacto…"
              className="selectable min-h-0 flex-1 resize-none rounded-lg border border-ink-800 bg-ink-900 p-3 font-mono text-sm text-mist-200 outline-none focus:border-ink-600"
            />
          </label>

          {options.margin < 4 && (
            <Banner tone="warn">
              Con menos de 4 módulos de margen, muchos lectores no encuentran el
              código. La especificación pide 4 como mínimo.
            </Banner>
          )}

          {lowContrastWarning && (
            <Banner tone="error">
              El color y el fondo son el mismo: el código será invisible.
            </Banner>
          )}
        </div>

        <div className="flex min-h-0 flex-col">
          <h3 className="px-1 pb-2 text-[11px] uppercase tracking-wider text-mist-500">
            Vista previa
            {result && (
              <span className="ml-2 font-mono normal-case text-mist-400">
                PNG {formatBytes(result.bytes)}
              </span>
            )}
          </h3>
          <div className="flex min-h-0 flex-1 items-center justify-center rounded-lg border border-ink-800 bg-ink-900 p-6">
            {result ? (
              <img
                src={result.preview}
                alt="Código QR generado"
                className="max-h-full max-w-full object-contain"
              />
            ) : (
              <p className="text-xs text-mist-500">
                Escribe el contenido del código.
              </p>
            )}
          </div>
        </div>
      </div>
    </ToolShell>
  );
}
