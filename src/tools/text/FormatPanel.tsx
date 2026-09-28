import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";

import { CopyButton, SplitEditors, ToolShell } from "../../components/Panels";
import { NumberField, Toggle } from "../../components/ui";
import type { ConvertOpts, FormatId } from "../../lib/convert";
import { DEFAULT_OPTS } from "../../lib/convert";
import { ipcErrorMessage } from "../../lib/ipc";
import type { PanelProps } from "../panels";

/**
 * Un solo panel sirve para JSON, YAML y XML: formatear es convertir un formato
 * a sí mismo, así que la única diferencia entre los tres es qué formato se le
 * pasa al motor.
 */
const FORMAT_BY_TOOL: Record<string, FormatId> = {
  "json-format": "json",
  "yaml-format": "yaml",
  "xml-format": "xml",
};

const EXAMPLES: Record<FormatId, string> = {
  json: '{"servidor":{"host":"localhost","puerto":8080},"activo":true}',
  yaml: "servidor:\n  host: localhost\n  puerto: 8080\nactivo: true\n",
  xml: '<config activo="true"><servidor host="localhost" puerto="8080"/></config>',
  toml: "",
  csv: "",
  tsv: "",
  xlsx: "",
};

export function FormatPanel({ tool }: PanelProps) {
  const format = FORMAT_BY_TOOL[tool.id] ?? "json";

  const [input, setInput] = useState(() => EXAMPLES[format] ?? "");
  const [output, setOutput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [opts, setOpts] = useState<ConvertOpts>(DEFAULT_OPTS);

  useEffect(() => {
    if (!input.trim()) {
      setOutput("");
      setError(null);
      return;
    }
    let cancelled = false;
    invoke<string>("format_document", { format, input, opts })
      .then((result) => {
        if (cancelled) return;
        setOutput(result);
        setError(null);
      })
      .catch((cause) => {
        if (cancelled) return;
        setOutput("");
        setError(ipcErrorMessage(cause));
      });
    return () => {
      cancelled = true;
    };
  }, [input, format, opts]);

  // YAML no tiene modo compacto: su forma legible es la única que existe.
  const supportsCompact = format !== "yaml";

  const status = useMemo(() => {
    if (error || !input.trim()) return null;
    return `Válido · ${output.length.toLocaleString("es")} caracteres`;
  }, [error, input, output]);

  return (
    <ToolShell
      error={error}
      notice={status}
      toolbar={
        <>
          {supportsCompact && (
            <Toggle
              label="Indentar"
              checked={opts.pretty}
              onChange={(pretty) => setOpts((prev) => ({ ...prev, pretty }))}
            />
          )}
          {opts.pretty && supportsCompact && (
            <NumberField
              label="Espacios"
              value={opts.indent}
              min={1}
              max={8}
              onChange={(indent) => setOpts((prev) => ({ ...prev, indent }))}
            />
          )}
          <div className="ml-auto">
            <CopyButton text={output} />
          </div>
        </>
      }
    >
      <SplitEditors
        input={input}
        output={output}
        inputFormat={format}
        outputFormat={format}
        onInputChange={setInput}
        outputLabel={error ? "Salida · con errores" : "Salida"}
        placeholder={`Pega aquí tu ${format.toUpperCase()}`}
      />
    </ToolShell>
  );
}
