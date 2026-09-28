import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

import { Editor } from "../../components/Editor";
import { CopyButton, Pane, ToolShell } from "../../components/Panels";
import { Banner, TextField, Toggle } from "../../components/ui";
import { ipcErrorMessage } from "../../lib/ipc";

interface CaptureGroup {
  index: number;
  name: string | null;
  value: string | null;
  start: number | null;
  end: number | null;
}

interface RegexMatch {
  start: number;
  end: number;
  value: string;
  groups: CaptureGroup[];
}

interface RegexReport {
  matches: RegexMatch[];
  total: number;
  truncated: boolean;
  groupNames: Array<string | null>;
  note: string | null;
}

interface RegexOptions {
  caseInsensitive: boolean;
  multiLine: boolean;
  dotMatchesNewline: boolean;
  extended: boolean;
}

const EXAMPLE_TEXT = `Contacto: ana@ejemplo.test
Soporte: soporte@fast-tools.test
Sin correo en esta línea.
Ventas: ventas@ejemplo.test`;

export function RegexPanel() {
  const [pattern, setPattern] = useState(String.raw`(?<usuario>[\w.-]+)@(?<dominio>[\w.-]+)`);
  const [haystack, setHaystack] = useState(EXAMPLE_TEXT);
  const [replacement, setReplacement] = useState("");
  const [replaced, setReplaced] = useState("");
  const [report, setReport] = useState<RegexReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [options, setOptions] = useState<RegexOptions>({
    caseInsensitive: false,
    multiLine: true,
    dotMatchesNewline: false,
    extended: false,
  });

  useEffect(() => {
    if (!pattern) {
      setReport(null);
      setError(null);
      return;
    }
    let cancelled = false;
    invoke<RegexReport>("test_regex", { pattern, haystack, options })
      .then((value) => {
        if (cancelled) return;
        setReport(value);
        setError(null);
      })
      .catch((cause) => {
        if (cancelled) return;
        setReport(null);
        setError(ipcErrorMessage(cause));
      });
    return () => {
      cancelled = true;
    };
  }, [pattern, haystack, options]);

  useEffect(() => {
    if (!pattern || !replacement) {
      setReplaced("");
      return;
    }
    let cancelled = false;
    invoke<string>("replace_regex", { pattern, haystack, replacement, options })
      .then((value) => {
        if (!cancelled) setReplaced(value);
      })
      .catch(() => {
        if (!cancelled) setReplaced("");
      });
    return () => {
      cancelled = true;
    };
  }, [pattern, haystack, replacement, options]);

  const set = <K extends keyof RegexOptions>(key: K, value: RegexOptions[K]) =>
    setOptions((previous) => ({ ...previous, [key]: value }));

  return (
    <ToolShell
      error={error}
      notice={
        report
          ? `${report.total} coincidencia${report.total === 1 ? "" : "s"}` +
            (report.truncated ? ` · se muestran las primeras ${report.matches.length}` : "")
          : null
      }
      toolbar={
        <>
          <Toggle
            label="Aa"
            title="Ignorar mayúsculas y minúsculas"
            checked={options.caseInsensitive}
            onChange={(value) => set("caseInsensitive", value)}
          />
          <Toggle
            label="^$ por línea"
            title="Los anclajes encajan al principio y al final de cada línea"
            checked={options.multiLine}
            onChange={(value) => set("multiLine", value)}
          />
          <Toggle
            label=". incluye salto"
            checked={options.dotMatchesNewline}
            onChange={(value) => set("dotMatchesNewline", value)}
          />
          <Toggle
            label="Modo extendido"
            title="Ignora los espacios del patrón y admite comentarios con #"
            checked={options.extended}
            onChange={(value) => set("extended", value)}
          />
          <TextField
            label="Reemplazo"
            width="w-52"
            title="Admite $1, $2 y $nombre para insertar los grupos capturados"
            value={replacement}
            onChange={setReplacement}
          />
          {replaced && <CopyButton text={replaced} label="Copiar reemplazo" />}
        </>
      }
    >
      <div className="flex h-full flex-col">
        <div className="border-b border-ink-800 px-4 py-2">
          <label className="flex items-center gap-2">
            <span className="text-[11px] uppercase tracking-wider text-mist-500">
              Patrón
            </span>
            <input
              value={pattern}
              onChange={(event) => setPattern(event.target.value)}
              spellCheck={false}
              className="selectable min-w-0 flex-1 rounded-md border border-ink-700 bg-ink-850 px-3 py-1.5 font-mono text-sm text-mist-100 outline-none focus:border-ink-600"
            />
          </label>
          <p className="mt-1 text-[11px] text-mist-500">
            Sintaxis del crate <span className="font-mono">regex</span> de Rust:
            sin retroceso, así que no hay patrones que se cuelguen. No admite
            miradas hacia atrás ni referencias a grupos anteriores.
          </p>
        </div>

        <div className="grid min-h-0 flex-1 grid-cols-2 gap-px bg-ink-800">
          <Pane label="Texto">
            <Editor value={haystack} format="json" onChange={setHaystack} />
          </Pane>

          <Pane label="Coincidencias">
            <div className="space-y-3 overflow-y-auto p-4">
              {report?.note && <Banner tone="warn">{report.note}</Banner>}

              {report && report.matches.length === 0 && (
                <p className="text-xs text-mist-500">
                  El patrón es válido pero no encaja con nada.
                </p>
              )}

              {report?.matches.map((match, index) => (
                <div
                  key={index}
                  className="rounded-lg border border-ink-800 bg-ink-900 p-3"
                >
                  <div className="flex items-baseline gap-2">
                    <span className="font-mono text-[11px] text-mist-500">
                      {match.start}–{match.end}
                    </span>
                    <span className="selectable min-w-0 flex-1 break-all font-mono text-sm text-bolt-400">
                      {match.value}
                    </span>
                  </div>

                  {match.groups.length > 0 && (
                    <ul className="mt-2 space-y-1 border-t border-ink-800 pt-2">
                      {match.groups.map((group) => (
                        <li
                          key={group.index}
                          className="flex gap-2 font-mono text-[11px]"
                        >
                          <span className="w-24 shrink-0 text-mist-500">
                            {group.name ?? `$${group.index}`}
                          </span>
                          <span
                            className={`selectable min-w-0 flex-1 break-all ${
                              group.value === null ? "text-mist-500" : "text-mist-200"
                            }`}
                          >
                            {group.value ?? "sin coincidencia"}
                          </span>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              ))}

              {replaced && (
                <div>
                  <h3 className="mb-1 text-[11px] uppercase tracking-wider text-mist-500">
                    Con el reemplazo aplicado
                  </h3>
                  <pre className="selectable whitespace-pre-wrap rounded-lg border border-ink-800 bg-ink-900 p-3 font-mono text-xs text-mist-200">
                    {replaced}
                  </pre>
                </div>
              )}
            </div>
          </Pane>
        </div>
      </div>
    </ToolShell>
  );
}
