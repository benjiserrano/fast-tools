import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

import { FieldRow, ToolShell } from "../../components/Panels";
import { Banner, NumberField } from "../../components/ui";
import { ipcErrorMessage } from "../../lib/ipc";

interface CronField {
  name: string;
  value: string;
}

interface CronReport {
  nextRuns: string[];
  timeToNext: string | null;
  fields: CronField[];
  note: string | null;
}

const PRESETS: Array<[string, string]> = [
  ["*/5 * * * *", "Cada 5 minutos"],
  ["0 * * * *", "Cada hora en punto"],
  ["0 9 * * 1-5", "Días laborables a las 9:00"],
  ["0 3 * * 0", "Domingos de madrugada"],
  ["0 0 1 * *", "El día 1 de cada mes"],
  ["0 0 1 1 *", "Cada Año Nuevo"],
];

export function CronPanel() {
  const [expression, setExpression] = useState("0 9 * * 1-5");
  const [count, setCount] = useState(10);
  const [report, setReport] = useState<CronReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!expression.trim()) {
      setReport(null);
      setError(null);
      return;
    }
    let cancelled = false;
    invoke<CronReport>("parse_cron", { expression, count })
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
  }, [expression, count]);

  return (
    <ToolShell
      error={error}
      notice={
        report?.timeToNext ? `La próxima ejecución es dentro de ${report.timeToNext}.` : null
      }
      toolbar={
        <NumberField
          label="Ejecuciones"
          value={count}
          min={1}
          max={50}
          onChange={setCount}
        />
      }
    >
      <div className="grid h-full grid-cols-2 gap-4 p-4">
        <div className="flex min-h-0 flex-col gap-4">
          <label className="flex flex-col gap-2">
            <span className="text-[11px] uppercase tracking-wider text-mist-500">
              Expresión
            </span>
            <input
              value={expression}
              onChange={(event) => setExpression(event.target.value)}
              spellCheck={false}
              className="selectable rounded-md border border-ink-700 bg-ink-850 px-3 py-2 font-mono text-lg text-mist-100 outline-none focus:border-ink-600"
            />
          </label>

          {report?.note && <Banner tone="warn">{report.note}</Banner>}

          {report && report.fields.length > 0 && (
            <div className="rounded-lg border border-ink-800">
              {report.fields.map((field) => (
                <FieldRow key={field.name} label={field.name} value={field.value} />
              ))}
            </div>
          )}

          <div>
            <h3 className="mb-2 text-[11px] uppercase tracking-wider text-mist-500">
              Ejemplos
            </h3>
            <ul className="space-y-1">
              {PRESETS.map(([value, label]) => (
                <li key={value}>
                  <button
                    type="button"
                    onClick={() => setExpression(value)}
                    className="flex w-full items-baseline gap-3 rounded-md px-2 py-1.5 text-left text-xs transition-colors hover:bg-ink-900"
                  >
                    <span className="w-28 shrink-0 font-mono text-bolt-400">
                      {value}
                    </span>
                    <span className="text-mist-400">{label}</span>
                  </button>
                </li>
              ))}
            </ul>
          </div>

          <Banner tone="info">
            Se admiten cinco campos, o seis si el primero son los segundos. Las
            horas se muestran en la zona horaria de esta máquina, que no tiene
            por qué ser la del servidor donde se vaya a programar.
          </Banner>
        </div>

        <div className="flex min-h-0 flex-col">
          <h3 className="px-1 pb-2 text-[11px] uppercase tracking-wider text-mist-500">
            Próximas ejecuciones
          </h3>
          <ol className="min-h-0 flex-1 overflow-y-auto rounded-lg border border-ink-800">
            {report?.nextRuns.map((run, index) => (
              <li
                key={run + index}
                className="flex items-baseline gap-3 border-b border-ink-800 px-4 py-2 font-mono text-xs last:border-b-0"
              >
                <span className="w-6 shrink-0 text-right text-mist-500">
                  {index + 1}
                </span>
                <span className="selectable text-mist-200">{run}</span>
              </li>
            ))}
            {(!report || report.nextRuns.length === 0) && (
              <li className="p-4 text-xs text-mist-500">
                Escribe una expresión válida para ver cuándo se ejecuta.
              </li>
            )}
          </ol>
        </div>
      </div>
    </ToolShell>
  );
}
