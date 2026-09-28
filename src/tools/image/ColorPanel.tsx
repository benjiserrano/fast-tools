import { useMemo, useState } from "react";

import { CopyButton, FieldRow, ToolShell } from "../../components/Panels";
import { Banner, TextField } from "../../components/ui";
import {
  describe as describeColor,
  judgeContrast,
  parseColor,
} from "../../lib/web-tools/color";

export function ColorPanel() {
  const [input, setInput] = useState("#3366cc");
  const [against, setAgainst] = useState("#ffffff");

  const color = useMemo(() => parseColor(input), [input]);
  const backdrop = useMemo(() => parseColor(against), [against]);

  const notation = color ? describeColor(color) : null;
  const contrast = color && backdrop ? judgeContrast(color, backdrop) : null;

  return (
    <ToolShell
      error={
        input && !color
          ? "No se reconoce ese color. Prueba con #rrggbb, rgb(), hsl() u oklch()."
          : null
      }
      toolbar={
        <>
          <TextField label="Color" width="w-52" value={input} onChange={setInput} />
          <TextField
            label="Sobre fondo"
            width="w-52"
            title="Para calcular el contraste según la norma WCAG"
            value={against}
            onChange={setAgainst}
          />
        </>
      }
    >
      <div className="grid h-full grid-cols-2 gap-4 p-4">
        <div className="flex min-h-0 flex-col gap-4">
          <div
            className="flex h-40 items-center justify-center rounded-lg border border-ink-800"
            style={{ backgroundColor: notation?.hex ?? "transparent" }}
          >
            {!notation && (
              <span className="text-xs text-mist-500">Sin color válido</span>
            )}
          </div>

          {notation && (
            <div className="rounded-lg border border-ink-800">
              {(["hex", "rgb", "hsl", "oklch"] as const).map((key) => (
                <FieldRow
                  key={key}
                  label={key.toUpperCase()}
                  value={
                    <span className="flex items-center gap-2">
                      <span className="min-w-0 flex-1">{notation[key]}</span>
                      <CopyButton text={notation[key]} />
                    </span>
                  }
                />
              ))}
            </div>
          )}

          <Banner tone="info">
            OKLCH es perceptualmente uniforme: cambiar su luminosidad no vira el
            tono, cosa que en HSL sí pasa. Es lo que conviene usar para generar
            escalas de color.
          </Banner>
        </div>

        <div className="flex min-h-0 flex-col gap-4">
          {contrast && notation && backdrop && (
            <>
              <div
                className="flex h-40 flex-col items-center justify-center gap-2 rounded-lg border border-ink-800"
                style={{ backgroundColor: against }}
              >
                <span className="text-2xl font-semibold" style={{ color: input }}>
                  Texto de ejemplo
                </span>
                <span className="text-sm" style={{ color: input }}>
                  Texto pequeño, 14 píxeles
                </span>
              </div>

              <div className="rounded-lg border border-ink-800">
                <FieldRow
                  label="Contraste"
                  value={`${contrast.ratio.toFixed(2)}:1`}
                  tone={contrast.aaNormal ? "good" : "bad"}
                />
                <FieldRow
                  label="AA texto normal (4,5:1)"
                  value={contrast.aaNormal ? "cumple" : "no cumple"}
                  tone={contrast.aaNormal ? "good" : "bad"}
                />
                <FieldRow
                  label="AA texto grande (3:1)"
                  value={contrast.aaLarge ? "cumple" : "no cumple"}
                  tone={contrast.aaLarge ? "good" : "bad"}
                />
                <FieldRow
                  label="AAA texto normal (7:1)"
                  value={contrast.aaaNormal ? "cumple" : "no cumple"}
                  tone={contrast.aaaNormal ? "good" : "muted"}
                />
              </div>

              <Banner tone={contrast.aaNormal ? "info" : "warn"}>
                {contrast.summary}
              </Banner>
            </>
          )}
        </div>
      </div>
    </ToolShell>
  );
}
