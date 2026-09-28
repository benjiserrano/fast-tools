import { useMemo, useState } from "react";

import { Editor } from "../../components/Editor";
import { Pane, ToolShell } from "../../components/Panels";
import { computeStats } from "../../lib/web-tools/text";

export function StatsPanel() {
  const [input, setInput] = useState("");
  const stats = useMemo(() => computeStats(input), [input]);

  const cards: Array<{ label: string; value: string; note?: string }> = [
    { label: "Palabras", value: format(stats.words) },
    { label: "Líneas", value: format(stats.lines), note: `${stats.nonEmptyLines} con contenido` },
    { label: "Párrafos", value: format(stats.paragraphs) },
    {
      label: "Caracteres",
      value: format(stats.graphemes),
      note: `${format(stats.charactersNoSpaces)} sin espacios`,
    },
    {
      label: "Bytes (UTF-8)",
      value: format(stats.bytes),
      // Cuando difieren hay caracteres multibyte, y eso importa para límites
      // de columna de base de datos o de campo de API.
      note:
        stats.bytes === stats.characters
          ? "todo ASCII"
          : `${format(stats.characters)} unidades UTF-16`,
    },
    {
      label: "Lectura",
      value: `${format(stats.readingMinutes)} min`,
      note: "a 200 palabras por minuto",
    },
  ];

  return (
    <ToolShell toolbar={<span className="text-xs text-mist-500">Se actualiza al escribir</span>}>
      <div className="grid h-full grid-cols-2 gap-px bg-ink-800">
        <Pane label="Texto">
          <Editor
            value={input}
            format="json"
            onChange={setInput}
            placeholder="Pega aquí el texto que quieras medir"
          />
        </Pane>

        <Pane label="Recuento">
          <div className="grid grid-cols-2 gap-3 overflow-y-auto p-4">
            {cards.map((card) => (
              <div
                key={card.label}
                className="rounded-lg border border-ink-800 bg-ink-900 px-3 py-3"
              >
                <p className="font-mono text-xl text-mist-50">{card.value}</p>
                <p className="mt-1 text-xs text-mist-400">{card.label}</p>
                {card.note && (
                  <p className="mt-0.5 text-[11px] text-mist-500">{card.note}</p>
                )}
              </div>
            ))}
          </div>
        </Pane>
      </div>
    </ToolShell>
  );
}

function format(value: number): string {
  return value.toLocaleString("es");
}
