import { save } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useMemo, useState } from "react";

import {
  DropZone,
  ImagePreview,
  imageDetail,
  useImageSource,
} from "../../components/ImagePicker";
import { ToolShell } from "../../components/Panels";
import {
  Banner,
  Button,
  NumberField,
  Select,
  TextField,
  Toggle,
} from "../../components/ui";
import type {
  FitMode,
  ImageFormatInfo,
  ImageKindId,
  ProcessResult,
  ResizeOptions,
  SaveImageOptions,
} from "../../lib/images";
import {
  DEFAULT_RESIZE,
  DEFAULT_SAVE,
  formatBytes,
  listImageFormats,
  processImage,
  saveProcessedImage,
} from "../../lib/images";
import { ipcErrorMessage } from "../../lib/ipc";
import type { PanelProps } from "../panels";

/** Qué controles enseña el panel según la herramienta que lo abre. */
const FOCUS: Record<string, "format" | "resize" | "compress"> = {
  "convert-image": "format",
  "image-resize": "resize",
  "image-compress": "compress",
};

export function ImagePanel({ tool }: PanelProps) {
  const focus = FOCUS[tool.id] ?? "format";

  const { image, error: sourceError, dragging, pick } = useImageSource();
  const [formats, setFormats] = useState<ImageFormatInfo[]>([]);
  const [to, setTo] = useState<ImageKindId>("png");
  const [resize, setResize] = useState<ResizeOptions>(DEFAULT_RESIZE);
  const [saveOptions, setSaveOptions] = useState<SaveImageOptions>(DEFAULT_SAVE);
  const [result, setResult] = useState<ProcessResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    listImageFormats().then(setFormats).catch(() => setFormats([]));
  }, []);

  const writable = useMemo(
    () => formats.filter((format) => format.canWrite),
    [formats],
  );

  // Al cargar una imagen, el destino por defecto es su mismo formato: quien
  // entra a redimensionar o comprimir no quiere cambiarlo de tipo sin pedirlo.
  useEffect(() => {
    if (image && focus !== "format") setTo(image.kind);
  }, [image, focus]);

  const request = useMemo(
    () =>
      image
        ? {
            path: image.path,
            to,
            resize,
            save: saveOptions,
            svgWidth: null,
          }
        : null,
    [image, to, resize, saveOptions],
  );

  useEffect(() => {
    if (!request) {
      setResult(null);
      return;
    }
    let cancelled = false;
    setBusy(true);
    processImage(request)
      .then((value) => {
        if (cancelled) return;
        setResult(value);
        setError(null);
      })
      .catch((cause) => {
        if (cancelled) return;
        setResult(null);
        setError(ipcErrorMessage(cause));
      })
      .finally(() => {
        if (!cancelled) setBusy(false);
      });
    return () => {
      cancelled = true;
    };
  }, [request]);

  const handleSave = useCallback(async () => {
    if (!request || !image) return;
    const format = formats.find((candidate) => candidate.id === to);
    const extension = format?.defaultExtension ?? to;
    const base = image.path.split(/[\\/]/).pop()?.replace(/\.[^.]+$/, "") ?? "imagen";

    try {
      const destination = await save({
        defaultPath: `${base}.${extension}`,
        filters: [{ name: format?.label ?? to, extensions: [extension] }],
      });
      if (!destination) return;
      await saveProcessedImage(request, destination, true);
      setNotice(`Guardado en ${destination}`);
      setTimeout(() => setNotice(null), 3000);
    } catch (cause) {
      setError(ipcErrorMessage(cause));
    }
  }, [request, image, formats, to]);

  const targetFormat = formats.find((format) => format.id === to);
  const set = <K extends keyof ResizeOptions>(key: K, value: ResizeOptions[K]) =>
    setResize((previous) => ({ ...previous, [key]: value }));

  const savings =
    result && result.originalBytes > 0
      ? Math.round((1 - result.bytes / result.originalBytes) * 100)
      : null;

  return (
    <ToolShell
      error={error ?? sourceError}
      notice={notice}
      toolbar={
        <>
          <Select
            label="Formato"
            value={to}
            onChange={(event) => setTo(event.target.value as ImageKindId)}
          >
            {writable.map((format) => (
              <option key={format.id} value={format.id}>
                {format.label}
              </option>
            ))}
          </Select>

          {focus !== "format" && (
            <>
              <NumberField
                label="Ancho"
                value={resize.width ?? 0}
                min={0}
                max={20000}
                onChange={(width) => set("width", width || null)}
              />
              <NumberField
                label="Alto"
                value={resize.height ?? 0}
                min={0}
                max={20000}
                onChange={(height) => set("height", height || null)}
              />
              <Select
                label="Encaje"
                value={resize.fit}
                onChange={(event) => set("fit", event.target.value as FitMode)}
              >
                <option value="contain">Contener</option>
                <option value="cover">Cubrir y recortar</option>
                <option value="stretch">Estirar</option>
              </Select>
              <Toggle
                label="No agrandar"
                checked={resize.noUpscale}
                onChange={(value) => set("noUpscale", value)}
              />
              <Select
                label="Girar"
                value={String(resize.rotate)}
                onChange={(event) => set("rotate", Number(event.target.value))}
              >
                <option value="0">0°</option>
                <option value="90">90°</option>
                <option value="180">180°</option>
                <option value="270">270°</option>
              </Select>
              <Toggle
                label="Espejo ↔"
                checked={resize.flipHorizontal}
                onChange={(value) => set("flipHorizontal", value)}
              />
              <Toggle
                label="Espejo ↕"
                checked={resize.flipVertical}
                onChange={(value) => set("flipVertical", value)}
              />
            </>
          )}

          {targetFormat?.lossy && (
            <NumberField
              label="Calidad"
              value={saveOptions.quality}
              min={1}
              max={100}
              onChange={(quality) =>
                setSaveOptions((previous) => ({ ...previous, quality }))
              }
            />
          )}

          {targetFormat && !targetFormat.supportsAlpha && image?.hasAlpha && (
            <TextField
              label="Fondo"
              width="w-20"
              title="Color sobre el que se aplana la transparencia"
              value={saveOptions.background}
              onChange={(background) =>
                setSaveOptions((previous) => ({ ...previous, background }))
              }
            />
          )}

          <div className="ml-auto flex items-center gap-2">
            <Button onClick={pick}>Cambiar imagen</Button>
            <Button variant="primary" onClick={handleSave} disabled={!result || busy}>
              Guardar como
            </Button>
          </div>
        </>
      }
    >
      <div className="flex h-full flex-col gap-3 p-4">
        {targetFormat?.limitation && (
          <Banner tone="info">{targetFormat.limitation}</Banner>
        )}

        {targetFormat && !targetFormat.supportsAlpha && image?.hasAlpha && (
          <Banner tone="warn">
            {targetFormat.label} no admite transparencia: los píxeles
            transparentes se aplanarán sobre {saveOptions.background}.
          </Banner>
        )}

        {result && (
          <Banner tone="info">
            {formatBytes(result.originalBytes)} → {formatBytes(result.bytes)}
            {savings !== null &&
              (savings > 0
                ? ` · ${savings} % menos`
                : savings < 0
                  ? ` · ${-savings} % más`
                  : " · mismo peso")}
            {" · "}
            {result.width}×{result.height} píxeles
          </Banner>
        )}

        {!image ? (
          <DropZone dragging={dragging} onPick={pick}>
            <p className="text-xs text-mist-500">
              PNG, JPEG, WebP, GIF, BMP, TIFF, ICO y SVG
            </p>
          </DropZone>
        ) : (
          <div className="grid min-h-0 flex-1 grid-cols-2 gap-4">
            <ImagePreview
              source={image.preview}
              label={`Original · ${image.label}`}
              detail={imageDetail(image)}
            />
            {result && (
              <ImagePreview
                source={result.preview}
                label={`Resultado · ${targetFormat?.label ?? to}`}
                detail={imageDetail(result)}
              />
            )}
          </div>
        )}
      </div>
    </ToolShell>
  );
}
