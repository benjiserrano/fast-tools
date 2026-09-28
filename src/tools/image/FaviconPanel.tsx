import { save } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useState } from "react";

import {
  DropZone,
  ImagePreview,
  imageDetail,
  useImageSource,
} from "../../components/ImagePicker";
import { ToolShell } from "../../components/Panels";
import { Banner, Button } from "../../components/ui";
import { faviconSizes, generateFavicon } from "../../lib/images";
import { ipcErrorMessage } from "../../lib/ipc";

export function FaviconPanel() {
  // Se rasteriza el SVG al mayor tamaño posible para que ninguna resolución
  // salga de ampliar otra más pequeña.
  const { image, error: sourceError, dragging, pick } = useImageSource(256);

  const [available, setAvailable] = useState<number[]>([]);
  const [selected, setSelected] = useState<number[]>([16, 32, 48]);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    faviconSizes()
      .then(setAvailable)
      .catch(() => setAvailable([16, 32, 48, 64, 128, 256]));
  }, []);

  const toggle = (size: number) =>
    setSelected((previous) =>
      previous.includes(size)
        ? previous.filter((item) => item !== size)
        : [...previous, size].sort((a, b) => a - b),
    );

  const handleSave = useCallback(async () => {
    if (!image) return;
    try {
      const destination = await save({
        defaultPath: "favicon.ico",
        filters: [{ name: "Icono", extensions: ["ico"] }],
      });
      if (!destination) return;

      await generateFavicon(image.path, selected, destination, true);
      setNotice(`Guardado en ${destination}`);
      setTimeout(() => setNotice(null), 3000);
    } catch (cause) {
      setError(ipcErrorMessage(cause));
    }
  }, [image, selected]);

  const tooSmall = image
    ? selected.filter((size) => size > Math.min(image.width, image.height))
    : [];

  return (
    <ToolShell
      error={error ?? sourceError}
      notice={notice}
      toolbar={
        <>
          <span className="text-xs text-mist-500">Tamaños</span>
          <div className="flex flex-wrap gap-1.5">
            {available.map((size) => (
              <button
                key={size}
                type="button"
                onClick={() => toggle(size)}
                className={`rounded-md px-2 py-1 font-mono text-xs transition-colors ${
                  selected.includes(size)
                    ? "bg-bolt-500/15 text-bolt-400"
                    : "border border-ink-700 text-mist-500 hover:text-mist-200"
                }`}
              >
                {size}
              </button>
            ))}
          </div>
          <div className="ml-auto flex items-center gap-2">
            {image && <Button onClick={pick}>Cambiar imagen</Button>}
            <Button
              variant="primary"
              onClick={handleSave}
              disabled={!image || selected.length === 0}
            >
              Guardar .ico
            </Button>
          </div>
        </>
      }
    >
      <div className="flex h-full flex-col gap-3 p-4">
        <Banner tone="info">
          Un solo archivo .ico con todas las resoluciones dentro. 16 y 32 son los
          que usa la pestaña del navegador; 48 y 256, los accesos directos de
          Windows.
        </Banner>

        {tooSmall.length > 0 && (
          <Banner tone="warn">
            La imagen mide {image?.width}×{image?.height}: los tamaños{" "}
            {tooSmall.join(", ")} saldrán de ampliarla y se verán borrosos. Parte
            de un original cuadrado de 512 píxeles o de un SVG.
          </Banner>
        )}

        {!image ? (
          <DropZone dragging={dragging} onPick={pick}>
            <p className="text-xs text-mist-500">
              Lo ideal es un SVG o un PNG cuadrado grande
            </p>
          </DropZone>
        ) : (
          <div className="grid min-h-0 flex-1 grid-cols-2 gap-4">
            <ImagePreview
              source={image.preview}
              label={`Origen · ${image.label}`}
              detail={imageDetail(image)}
            />

            <div className="flex min-h-0 flex-col">
              <h3 className="px-1 pb-2 text-[11px] uppercase tracking-wider text-mist-500">
                Cómo se verá
              </h3>
              <div className="flex flex-wrap items-end gap-6 overflow-y-auto rounded-lg border border-ink-800 p-4">
                {selected.map((size) => (
                  <div key={size} className="flex flex-col items-center gap-2">
                    <img
                      src={image.preview}
                      alt={`${size} píxeles`}
                      width={size}
                      height={size}
                      style={{ width: size, height: size }}
                      className="rounded-sm object-cover"
                    />
                    <span className="font-mono text-[11px] text-mist-500">
                      {size}
                    </span>
                  </div>
                ))}
                {selected.length === 0 && (
                  <p className="text-xs text-mist-500">
                    Elige al menos un tamaño arriba.
                  </p>
                )}
              </div>
            </div>
          </div>
        )}
      </div>
    </ToolShell>
  );
}
