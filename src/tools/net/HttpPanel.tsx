import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

import { Editor } from "../../components/Editor";
import { CopyButton, FieldRow, Pane, ToolShell } from "../../components/Panels";
import { Banner, Button, NumberField, Select, Toggle } from "../../components/ui";
import { ipcErrorMessage } from "../../lib/ipc";
import { statusFamily } from "../../lib/web-tools/net";

interface HttpResponse {
  status: number;
  statusText: string;
  headers: Array<[string, string]>;
  body: string;
  bodyIsBinary: boolean;
  bytes: number;
  elapsedMs: number;
  contentType: string | null;
  redirectTo: string | null;
  truncated: boolean;
}

type Header = { name: string; value: string };

const BODY_METHODS = ["POST", "PUT", "PATCH", "DELETE"];

export function HttpPanel() {
  const [methods, setMethods] = useState<string[]>([]);
  const [method, setMethod] = useState("GET");
  const [url, setUrl] = useState("https://api.github.com/repos/jgm/pandoc");
  const [headers, setHeaders] = useState<Header[]>([{ name: "", value: "" }]);
  const [body, setBody] = useState("");
  const [timeoutSeconds, setTimeoutSeconds] = useState(30);
  const [followRedirects, setFollowRedirects] = useState(false);
  const [response, setResponse] = useState<HttpResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [view, setView] = useState<"body" | "headers">("body");

  useEffect(() => {
    invoke<string[]>("http_methods")
      .then(setMethods)
      .catch(() => setMethods(["GET", "POST"]));
  }, []);

  const send = useCallback(async () => {
    setRunning(true);
    setError(null);
    try {
      setResponse(
        await invoke<HttpResponse>("send_http", {
          request: {
            method,
            url,
            headers: headers
              .filter((header) => header.name.trim())
              .map((header) => [header.name, header.value] as [string, string]),
            body: BODY_METHODS.includes(method) && body ? body : null,
            timeoutSeconds,
            followRedirects,
          },
        }),
      );
    } catch (cause) {
      setResponse(null);
      setError(ipcErrorMessage(cause));
    } finally {
      setRunning(false);
    }
  }, [method, url, headers, body, timeoutSeconds, followRedirects]);

  const setHeader = (index: number, patch: Partial<Header>) =>
    setHeaders((previous) =>
      previous.map((header, position) =>
        position === index ? { ...header, ...patch } : header,
      ),
    );

  const family = response ? statusFamily(response.status) : null;
  // El cuerpo se resalta como JSON solo cuando el servidor dice que lo es.
  const bodyFormat = response?.contentType?.includes("json") ? "json" : "xml";

  return (
    <ToolShell
      error={error}
      notice={
        response
          ? `${response.status} ${response.statusText} · ${response.elapsedMs} ms · ${response.bytes.toLocaleString("es")} bytes`
          : null
      }
      toolbar={
        <>
          <Select
            label="Método"
            value={method}
            onChange={(event) => setMethod(event.target.value)}
          >
            {methods.map((candidate) => (
              <option key={candidate} value={candidate}>
                {candidate}
              </option>
            ))}
          </Select>
          <NumberField
            label="Plazo (s)"
            value={timeoutSeconds}
            min={1}
            max={300}
            onChange={setTimeoutSeconds}
          />
          <Toggle
            label="Seguir redirecciones"
            title="Sin esto, una redirección se muestra en vez de seguirse, y las cabeceras no viajan a un destino que no escribiste"
            checked={followRedirects}
            onChange={setFollowRedirects}
          />
          <div className="ml-auto flex items-center gap-2">
            {response && <CopyButton text={response.body} label="Copiar cuerpo" />}
            <Button variant="primary" onClick={send} disabled={running || !url.trim()}>
              {running ? "Enviando…" : "Enviar"}
            </Button>
          </div>
        </>
      }
    >
      <div className="grid h-full grid-cols-2 gap-px bg-ink-800">
        <section className="flex min-h-0 flex-col overflow-y-auto bg-ink-950 p-4">
          <Banner tone="warn">
            Esta es la única herramienta que envía datos fuera de tu máquina, y
            solo cuando pulsas Enviar.
          </Banner>

          <label className="mt-4 flex flex-col gap-2">
            <span className="text-[11px] uppercase tracking-wider text-mist-500">
              Dirección
            </span>
            <input
              value={url}
              onChange={(event) => setUrl(event.target.value)}
              spellCheck={false}
              onKeyDown={(event) => {
                if (event.key === "Enter") void send();
              }}
              className="selectable rounded-md border border-ink-700 bg-ink-850 px-3 py-2 font-mono text-sm text-mist-100 outline-none focus:border-ink-600"
            />
          </label>

          <div className="mt-4">
            <div className="mb-2 flex items-center gap-2">
              <h3 className="text-[11px] uppercase tracking-wider text-mist-500">
                Cabeceras
              </h3>
              <Button
                onClick={() => setHeaders((previous) => [...previous, { name: "", value: "" }])}
              >
                Añadir
              </Button>
            </div>
            <ul className="space-y-1.5">
              {headers.map((header, index) => (
                <li key={index} className="flex gap-2">
                  <input
                    value={header.name}
                    placeholder="Nombre"
                    onChange={(event) => setHeader(index, { name: event.target.value })}
                    className="w-40 rounded-md border border-ink-700 bg-ink-850 px-2 py-1 font-mono text-xs text-mist-200 outline-none focus:border-ink-600"
                  />
                  <input
                    value={header.value}
                    placeholder="Valor"
                    onChange={(event) => setHeader(index, { value: event.target.value })}
                    className="min-w-0 flex-1 rounded-md border border-ink-700 bg-ink-850 px-2 py-1 font-mono text-xs text-mist-200 outline-none focus:border-ink-600"
                  />
                  {headers.length > 1 && (
                    <button
                      type="button"
                      onClick={() =>
                        setHeaders((previous) =>
                          previous.filter((_, position) => position !== index),
                        )
                      }
                      className="px-1 text-mist-500 hover:text-danger-400"
                    >
                      ✕
                    </button>
                  )}
                </li>
              ))}
            </ul>
          </div>

          {BODY_METHODS.includes(method) && (
            <label className="mt-4 flex min-h-[10rem] flex-col gap-2">
              <span className="text-[11px] uppercase tracking-wider text-mist-500">
                Cuerpo
              </span>
              <textarea
                value={body}
                onChange={(event) => setBody(event.target.value)}
                className="selectable min-h-[9rem] flex-1 resize-y rounded-md border border-ink-700 bg-ink-850 p-3 font-mono text-xs text-mist-200 outline-none focus:border-ink-600"
              />
            </label>
          )}
        </section>

        <Pane
          label="Respuesta"
          actions={
            response && (
              <div className="flex gap-1">
                {(["body", "headers"] as const).map((candidate) => (
                  <button
                    key={candidate}
                    type="button"
                    onClick={() => setView(candidate)}
                    className={`rounded px-2 py-0.5 text-[11px] transition-colors ${
                      view === candidate
                        ? "bg-ink-800 text-mist-200"
                        : "text-mist-500 hover:text-mist-400"
                    }`}
                  >
                    {candidate === "body" ? "Cuerpo" : `Cabeceras (${response.headers.length})`}
                  </button>
                ))}
              </div>
            )
          }
        >
          {!response ? (
            <div className="flex h-full items-center justify-center">
              <p className="text-xs text-mist-500">
                Pulsa Enviar para hacer la petición.
              </p>
            </div>
          ) : view === "headers" ? (
            <div className="h-full overflow-y-auto">
              {response.headers.map(([name, value], index) => (
                <FieldRow key={`${name}-${index}`} label={name} value={value} />
              ))}
            </div>
          ) : (
            <div className="flex h-full flex-col">
              {family && (
                <div className="px-4 pb-2">
                  <Banner tone={family.tone === "good" ? "info" : family.tone}>
                    {family.label} · {response.status} {response.statusText}
                    {response.redirectTo && (
                      <span className="selectable block break-all">
                        Redirige a {response.redirectTo}
                      </span>
                    )}
                    {response.truncated && (
                      <span className="block">
                        El cuerpo se ha recortado a 16 MB para no bloquear la ventana.
                      </span>
                    )}
                  </Banner>
                </div>
              )}
              <div className="min-h-0 flex-1">
                <Editor value={response.body} format={bodyFormat} readOnly />
              </div>
            </div>
          )}
        </Pane>
      </div>
    </ToolShell>
  );
}
