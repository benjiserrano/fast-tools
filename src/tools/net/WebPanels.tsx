import { useEffect, useMemo, useState } from "react";

import { CopyButton, FieldRow, ToolShell } from "../../components/Panels";
import { Banner, Button, TextField } from "../../components/ui";
import {
  analyzeUrl,
  describeTimestamp,
  HTTP_STATUS,
  parseTimestamp,
  statusFamily,
} from "../../lib/web-tools/net";

// ── Inspector de URL ──────────────────────────────────────────────────────

export function UrlPanel() {
  const [input, setInput] = useState(
    "https://api.ejemplo.test/v1/pedidos?estado=abierto&pagina=2#detalle",
  );
  const analysis = useMemo(() => analyzeUrl(input), [input]);
  const parts = analysis.parts;

  return (
    <ToolShell
      error={analysis.error}
      toolbar={
        <span className="text-[11px] text-mist-500">
          Se descompone al escribir. Los valores se muestran ya decodificados.
        </span>
      }
    >
      <div className="flex h-full flex-col gap-4 overflow-y-auto p-4">
        <label className="flex flex-col gap-2">
          <span className="text-[11px] uppercase tracking-wider text-mist-500">
            Dirección
          </span>
          <input
            value={input}
            onChange={(event) => setInput(event.target.value)}
            spellCheck={false}
            className="selectable rounded-md border border-ink-700 bg-ink-850 px-3 py-2 font-mono text-sm text-mist-100 outline-none focus:border-ink-600"
          />
        </label>

        {analysis.warnings.map((warning) => (
          <Banner key={warning} tone="warn">
            {warning}
          </Banner>
        ))}

        {parts && (
          <>
            <div className="rounded-lg border border-ink-800">
              <FieldRow label="Esquema" value={parts.protocol} />
              <FieldRow label="Origen" value={parts.origin} />
              <FieldRow label="Anfitrión" value={parts.host} />
              {parts.port && <FieldRow label="Puerto" value={parts.port} />}
              {parts.username && (
                <FieldRow label="Usuario" value={parts.username} tone="bad" />
              )}
              {parts.password && (
                <FieldRow label="Contraseña" value="••••••••" tone="bad" />
              )}
              <FieldRow label="Ruta" value={parts.path} />
              {parts.hash && <FieldRow label="Fragmento" value={parts.hash} />}
            </div>

            <div>
              <h3 className="mb-2 text-[11px] uppercase tracking-wider text-mist-500">
                Parámetros ({parts.query.length})
              </h3>
              {parts.query.length === 0 ? (
                <p className="text-xs text-mist-500">La dirección no lleva parámetros.</p>
              ) : (
                <div className="rounded-lg border border-ink-800">
                  {parts.query.map(([key, value], index) => (
                    <FieldRow
                      key={`${key}-${index}`}
                      label={key}
                      value={
                        <span className="flex items-center gap-2">
                          <span className="min-w-0 flex-1">{value || "(vacío)"}</span>
                          <CopyButton text={value} />
                        </span>
                      }
                    />
                  ))}
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </ToolShell>
  );
}

// ── Marcas de tiempo ──────────────────────────────────────────────────────

export function TimestampPanel() {
  const [input, setInput] = useState(() => String(Math.floor(Date.now() / 1000)));
  const [now, setNow] = useState(Date.now());

  // El reloj de arriba avanza solo; si no, el «hace 3 minutos» se queda
  // congelado en el momento en que se abrió la herramienta.
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  const date = useMemo(() => parseTimestamp(input), [input]);
  const view = useMemo(() => (date ? describeTimestamp(date) : null), [date]);

  return (
    <ToolShell
      error={
        input.trim() && !date
          ? "No se reconoce ese instante. Prueba con un número Unix o una fecha ISO 8601."
          : null
      }
      toolbar={
        <>
          <Button onClick={() => setInput(String(Math.floor(Date.now() / 1000)))}>
            Ahora
          </Button>
          <span className="font-mono text-xs text-mist-500">
            {Math.floor(now / 1000)}
          </span>
        </>
      }
    >
      <div className="flex h-full flex-col gap-4 overflow-y-auto p-4">
        <label className="flex flex-col gap-2">
          <span className="text-[11px] uppercase tracking-wider text-mist-500">
            Instante
          </span>
          <input
            value={input}
            onChange={(event) => setInput(event.target.value)}
            spellCheck={false}
            placeholder="1700000000, 1700000000000 o 2026-03-14T15:09:26Z"
            className="selectable rounded-md border border-ink-700 bg-ink-850 px-3 py-2 font-mono text-lg text-mist-100 outline-none focus:border-ink-600"
          />
        </label>

        {view && (
          <div className="rounded-lg border border-ink-800">
            {(
              [
                ["Unix (segundos)", String(view.unixSeconds)],
                ["Unix (milisegundos)", String(view.unixMillis)],
                ["ISO 8601", view.iso],
                ["RFC 1123 (UTC)", view.utc],
                [`Local (${view.timeZone})`, view.local],
                ["Día de la semana", view.weekday],
                ["Relativo", view.relative],
              ] as const
            ).map(([label, value]) => (
              <FieldRow
                key={label}
                label={label}
                value={
                  <span className="flex items-center gap-2">
                    <span className="min-w-0 flex-1">{value}</span>
                    <CopyButton text={value} />
                  </span>
                }
              />
            ))}
          </div>
        )}

        <Banner tone="info">
          Un número de diez dígitos se interpreta como segundos y uno de trece
          como milisegundos. Es la convención que usan casi todas las API.
        </Banner>
      </div>
    </ToolShell>
  );
}

// ── Códigos de estado ─────────────────────────────────────────────────────

export function HttpStatusPanel() {
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return HTTP_STATUS;
    return HTTP_STATUS.filter(
      (status) =>
        String(status.code).includes(needle) ||
        status.name.toLowerCase().includes(needle) ||
        status.meaning.toLowerCase().includes(needle),
    );
  }, [query]);

  return (
    <ToolShell
      toolbar={
        <>
          <TextField label="Buscar" width="w-64" value={query} onChange={setQuery} />
          <span className="text-[11px] text-mist-500">
            {filtered.length} de {HTTP_STATUS.length}
          </span>
        </>
      }
    >
      <ul className="h-full overflow-y-auto p-4">
        {filtered.map((status) => {
          const family = statusFamily(status.code);
          const tone = {
            good: "text-emerald-400",
            info: "text-bolt-400",
            warn: "text-warn-400",
            error: "text-danger-400",
          }[family.tone];

          return (
            <li
              key={status.code}
              className="flex gap-4 border-b border-ink-800 px-2 py-3 last:border-b-0"
            >
              <span className={`w-12 shrink-0 font-mono text-lg ${tone}`}>
                {status.code}
              </span>
              <div className="min-w-0 flex-1">
                <p className="font-medium text-mist-200">{status.name}</p>
                <p className="mt-0.5 text-xs text-mist-400">{status.meaning}</p>
                <p className="mt-0.5 text-[11px] text-mist-500">{family.label}</p>
              </div>
            </li>
          );
        })}
        {filtered.length === 0 && (
          <li className="p-4 text-sm text-mist-500">
            Ningún código coincide con «{query}».
          </li>
        )}
      </ul>
    </ToolShell>
  );
}
