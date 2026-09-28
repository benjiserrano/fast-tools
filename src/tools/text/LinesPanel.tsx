import { useMemo, useState } from "react";

import { CopyButton, SplitEditors, ToolShell } from "../../components/Panels";
import { Select, TextField, Toggle } from "../../components/ui";
import type { LineOptions } from "../../lib/web-tools/text";
import { DEFAULT_LINE_OPTIONS, transformLines } from "../../lib/web-tools/text";

export function LinesPanel() {
  const [input, setInput] = useState("");
  const [options, setOptions] = useState<LineOptions>(DEFAULT_LINE_OPTIONS);

  const output = useMemo(() => transformLines(input, options), [input, options]);

  const set = <K extends keyof LineOptions>(key: K, value: LineOptions[K]) =>
    setOptions((previous) => ({ ...previous, [key]: value }));

  const inputLines = input === "" ? 0 : input.split("\n").length;
  const outputLines = output === "" ? 0 : output.split("\n").length;
  const removed = inputLines - outputLines;

  return (
    <ToolShell
      notice={
        input && removed > 0
          ? `${removed} línea${removed === 1 ? "" : "s"} menos: ${inputLines} → ${outputLines}`
          : null
      }
      toolbar={
        <>
          <Toggle
            label="Recortar"
            title="Quita los espacios del principio y del final de cada línea"
            checked={options.trim}
            onChange={(value) => set("trim", value)}
          />
          <Toggle
            label="Sin vacías"
            checked={options.removeEmpty}
            onChange={(value) => set("removeEmpty", value)}
          />
          <Toggle
            label="Sin duplicados"
            title="Conserva la primera aparición de cada línea"
            checked={options.deduplicate}
            onChange={(value) => set("deduplicate", value)}
          />
          <Select
            label="Orden"
            value={options.sort}
            onChange={(event) =>
              set("sort", event.target.value as LineOptions["sort"])
            }
          >
            <option value="none">Sin ordenar</option>
            <option value="asc">A → Z</option>
            <option value="desc">Z → A</option>
            <option value="length">Por longitud</option>
          </Select>
          <Toggle
            label="Invertir"
            checked={options.reverse}
            onChange={(value) => set("reverse", value)}
          />
          <Toggle
            label="Numerar"
            checked={options.number}
            onChange={(value) => set("number", value)}
          />

          <div className="h-4 w-px bg-ink-800" />

          <TextField
            label="Filtrar"
            width="w-28"
            value={options.filter}
            onChange={(value) => set("filter", value)}
          />
          <Toggle
            label="Excluir"
            title="Deja las líneas que NO contienen el filtro"
            checked={options.invertFilter}
            onChange={(value) => set("invertFilter", value)}
          />
          <Toggle
            label="Aa"
            title="Distinguir mayúsculas de minúsculas"
            checked={options.caseSensitive}
            onChange={(value) => set("caseSensitive", value)}
          />

          <div className="ml-auto">
            <CopyButton text={output} />
          </div>
        </>
      }
    >
      <SplitEditors
        input={input}
        output={output}
        onInputChange={setInput}
        inputLabel={`Entrada · ${inputLines} líneas`}
        outputLabel={`Salida · ${outputLines} líneas`}
        placeholder="Una línea por elemento"
      />
    </ToolShell>
  );
}
