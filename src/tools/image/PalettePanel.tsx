import { useEffect, useState } from "react";

import {
  DropZone,
  ImagePreview,
  imageDetail,
  useImageSource,
} from "../../components/ImagePicker";
import { CopyButton, ToolShell } from "../../components/Panels";
import { Button, NumberField } from "../../components/ui";
import type { PaletteColor } from "../../lib/images";
import { extractPalette } from "../../lib/images";
import { ipcErrorMessage } from "../../lib/ipc";
import { contrastRatio, parseHex } from "../../lib/web-tools/color";

export function PalettePanel() {
  const { image, error: sourceError, dragging, pick } = useImageSource();
  const [count, setCount] = useState(6);
  const [colors, setColors] = useState<PaletteColor[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!image) {
      setColors([]);
      return;
    }
    let cancelled = false;
    extractPalette(image.path, count)
      .then((value) => {
        if (cancelled) return;
        setColors(value);
        setError(null);
      })
      .catch((cause) => {
        if (cancelled) return;
        setColors([]);
        setError(ipcErrorMessage(cause));
      });
    return () => {
      cancelled = true;
    };
  }, [image, count]);

  const asCss = colors
    .map((color, index) => `  --color-${index + 1}: ${color.hex};`)
    .join("\n");

  return (
    <ToolShell
      error={error ?? sourceError}
      toolbar={
        <>
          <NumberField
            label="Colores"
            value={count}
            min={2}
            max={16}
            onChange={setCount}
          />
          <div className="ml-auto flex items-center gap-2">
            {image && <Button onClick={pick}>Cambiar imagen</Button>}
            <CopyButton
              text={colors.map((color) => color.hex).join("\n")}
              label="Copiar HEX"
            />
            <CopyButton text={`:root {\n${asCss}\n}`} label="Copiar CSS" />
          </div>
        </>
      }
    >
      <div className="flex h-full flex-col gap-3 p-4">
        {!image ? (
          <DropZone dragging={dragging} onPick={pick}>
            <p className="text-xs text-mist-500">
              Se extraen los colores dominantes de la imagen
            </p>
          </DropZone>
        ) : (
          <div className="grid min-h-0 flex-1 grid-cols-2 gap-4">
            <ImagePreview
              source={image.preview}
              label={image.label}
              detail={imageDetail(image)}
            />

            <div className="flex min-h-0 flex-col">
              <h3 className="px-1 pb-2 text-[11px] uppercase tracking-wider text-mist-500">
                Paleta
              </h3>
              <ul className="min-h-0 flex-1 space-y-2 overflow-y-auto">
                {colors.map((color) => (
                  <Swatch key={color.hex} color={color} />
                ))}
              </ul>
            </div>
          </div>
        )}
      </div>
    </ToolShell>
  );
}

function Swatch({ color }: { color: PaletteColor }) {
  const rgb = parseHex(color.hex);
  // Se elige el texto que más contrasta con el propio color, para que la
  // muestra siga siendo legible sea cual sea el color extraído.
  const onWhite = rgb ? contrastRatio(rgb, { r: 255, g: 255, b: 255 }) : 1;
  const onBlack = rgb ? contrastRatio(rgb, { r: 0, g: 0, b: 0 }) : 1;
  const textColor = onBlack > onWhite ? "#000000" : "#ffffff";

  return (
    <li className="flex items-center gap-3">
      <div
        className="flex h-14 flex-1 items-center justify-between rounded-lg px-3 font-mono text-xs"
        style={{ backgroundColor: color.hex, color: textColor }}
      >
        <span className="selectable">{color.hex}</span>
        <span>{color.share.toFixed(1)} %</span>
      </div>
      <CopyButton text={color.hex} />
    </li>
  );
}
