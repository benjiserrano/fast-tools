/** Selección de imagen compartida por todos los paneles de la Fase 3. */

import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useState, type ReactNode } from "react";

import type { LoadedImage } from "../lib/images";
import { formatBytes, loadImage } from "../lib/images";
import { ipcErrorMessage } from "../lib/ipc";
import { Button } from "./ui";

const IMAGE_EXTENSIONS = [
  "png",
  "jpg",
  "jpeg",
  "webp",
  "gif",
  "bmp",
  "tif",
  "tiff",
  "ico",
  "svg",
];

export function useImageSource(svgWidth: number | null = null) {
  const [image, setImage] = useState<LoadedImage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);

  const accept = useCallback(
    async (path: string) => {
      try {
        setImage(await loadImage(path, svgWidth));
        setError(null);
      } catch (cause) {
        setImage(null);
        setError(ipcErrorMessage(cause));
      }
    },
    [svgWidth],
  );

  const pick = useCallback(async () => {
    const path = await open({
      multiple: false,
      filters: [{ name: "Imágenes", extensions: IMAGE_EXTENSIONS }],
    });
    if (typeof path === "string") await accept(path);
  }, [accept]);

  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        setDragging(true);
        return;
      }
      setDragging(false);
      if (event.payload.type === "drop") {
        const first = event.payload.paths[0];
        if (first) void accept(first);
      }
    });
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, [accept]);

  return { image, error, dragging, pick, setError, reload: accept };
}

export function DropZone({
  dragging,
  onPick,
  children,
}: {
  dragging: boolean;
  onPick: () => void;
  children?: ReactNode;
}) {
  return (
    <div
      className={`flex h-full flex-col items-center justify-center gap-3 rounded-lg border border-dashed transition-colors ${
        dragging ? "border-bolt-500 bg-bolt-500/5" : "border-ink-700"
      }`}
    >
      <p className="text-sm text-mist-400">Arrastra una imagen aquí</p>
      {children}
      <Button variant="primary" onClick={onPick}>
        Elegir archivo
      </Button>
    </div>
  );
}

export function ImagePreview({
  source,
  label,
  detail,
}: {
  source: string;
  label: string;
  detail?: string;
}) {
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="flex items-baseline gap-2 px-1 pb-2">
        <h3 className="text-[11px] uppercase tracking-wider text-mist-500">
          {label}
        </h3>
        {detail && <span className="font-mono text-[11px] text-mist-400">{detail}</span>}
      </header>
      {/* El tablero de ajedrez deja ver dónde hay transparencia; sobre un fondo
          liso, un PNG con alfa y otro sin él se ven igual. */}
      <div
        className="flex min-h-0 flex-1 items-center justify-center overflow-hidden rounded-lg border border-ink-800"
        style={{
          backgroundImage:
            "repeating-conic-gradient(#1f2f4d 0% 25%, #16223a 0% 50%)",
          backgroundSize: "16px 16px",
        }}
      >
        <img
          src={source}
          alt={label}
          className="max-h-full max-w-full object-contain"
        />
      </div>
    </div>
  );
}

export function imageDetail(image: {
  width: number;
  height: number;
  bytes: number;
}): string {
  return `${image.width}×${image.height} · ${formatBytes(image.bytes)}`;
}
