import { useMemo, useState } from "react";

import { CopyButton, SplitEditors, ToolShell } from "../../components/Panels";
import { Button, Select, Toggle } from "../../components/ui";
import type { CaseStyle, EscapeTarget, UrlMode } from "../../lib/web-tools/text";
import {
  CASE_LABELS,
  convertCase,
  decodeBase64,
  decodeUrl,
  encodeBase64,
  encodeUrl,
  escapeText,
  unescapeText,
} from "../../lib/web-tools/text";
import type { PanelProps } from "../panels";

type Direction = "encode" | "decode";

/**
 * Paneles de ida y vuelta: Base64, URL y escapado comparten la misma forma
 * —entrada, un conmutador de dirección, unas opciones y salida— así que
 * comparten componente en lugar de repetirlo tres veces.
 */
export function TransformPanel({ tool }: PanelProps) {
  const [input, setInput] = useState("");
  const [direction, setDirection] = useState<Direction>("encode");

  const [urlSafe, setUrlSafe] = useState(false);
  const [urlMode, setUrlMode] = useState<UrlMode>("component");
  const [escapeTarget, setEscapeTarget] = useState<EscapeTarget>("html");

  const { output, error } = useMemo(() => {
    if (!input) return { output: "", error: null };
    try {
      return { output: run(), error: null };
    } catch (cause) {
      return {
        output: "",
        error:
          cause instanceof Error
            ? `No se pudo decodificar: ${cause.message}`
            : "No se pudo decodificar la entrada.",
      };
    }

    function run(): string {
      switch (tool.id) {
        case "base64":
          return direction === "encode"
            ? encodeBase64(input, urlSafe)
            : decodeBase64(input);
        case "url-encode":
          return direction === "encode"
            ? encodeUrl(input, urlMode)
            : decodeUrl(input, urlMode);
        case "escape":
          return direction === "encode"
            ? escapeText(input, escapeTarget)
            : unescapeText(input, escapeTarget);
        default:
          return input;
      }
    }
  }, [tool.id, input, direction, urlSafe, urlMode, escapeTarget]);

  const swap = () => {
    setDirection(direction === "encode" ? "decode" : "encode");
    if (output) setInput(output);
  };

  return (
    <ToolShell
      error={error}
      toolbar={
        <>
          <Select
            label="Dirección"
            value={direction}
            onChange={(event) => setDirection(event.target.value as Direction)}
          >
            <option value="encode">
              {tool.id === "escape" ? "Escapar" : "Codificar"}
            </option>
            <option value="decode">
              {tool.id === "escape" ? "Desescapar" : "Decodificar"}
            </option>
          </Select>

          <Button onClick={swap} disabled={!output} title="Invertir y reutilizar">
            ⇄
          </Button>

          {tool.id === "base64" && direction === "encode" && (
            <Toggle
              label="Seguro para URL"
              title="Usa «-» y «_» en lugar de «+» y «/», y quita el relleno"
              checked={urlSafe}
              onChange={setUrlSafe}
            />
          )}

          {tool.id === "url-encode" && (
            <Select
              label="Modo"
              value={urlMode}
              onChange={(event) => setUrlMode(event.target.value as UrlMode)}
            >
              <option value="component">Componente</option>
              <option value="full">URL completa</option>
              <option value="form">Formulario</option>
            </Select>
          )}

          {tool.id === "escape" && (
            <Select
              label="Destino"
              value={escapeTarget}
              onChange={(event) =>
                setEscapeTarget(event.target.value as EscapeTarget)
              }
            >
              <option value="html">HTML</option>
              <option value="json">JSON</option>
              <option value="regex">Expresión regular</option>
              <option value="shell">Shell (POSIX)</option>
            </Select>
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
        onInputChange={setInput}
        placeholder="Escribe o pega aquí"
      />
    </ToolShell>
  );
}

/** Conversor de mayúsculas: una entrada, todos los estilos a la vez. */
export function CasePanel() {
  const [input, setInput] = useState("nombre de usuario");
  const [style, setStyle] = useState<CaseStyle>("camel");

  const output = useMemo(() => convertCase(input, style), [input, style]);
  const styles = Object.keys(CASE_LABELS) as CaseStyle[];

  return (
    <ToolShell
      toolbar={
        <>
          {/* Los estilos se muestran como botones y no en un desplegable:
              son nueve y se elige entre ellos comparándolos de un vistazo. */}
          <div className="flex flex-wrap gap-1.5">
            {styles.map((candidate) => (
              <button
                key={candidate}
                type="button"
                onClick={() => setStyle(candidate)}
                className={`rounded-md px-2 py-1 font-mono text-xs transition-colors ${
                  candidate === style
                    ? "bg-bolt-500/15 text-bolt-400"
                    : "border border-ink-700 text-mist-500 hover:text-mist-200"
                }`}
              >
                {CASE_LABELS[candidate]}
              </button>
            ))}
          </div>
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
        outputLabel={`Salida · ${CASE_LABELS[style]}`}
        placeholder="Una línea por identificador"
      />
    </ToolShell>
  );
}
