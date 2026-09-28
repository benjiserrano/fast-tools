import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

import { CopyButton, SplitEditors, ToolShell } from "../../components/Panels";
import { NumberField, Toggle } from "../../components/ui";
import { ipcErrorMessage } from "../../lib/ipc";

interface SqlOptions {
  indent: number;
  uppercase: boolean;
  linesBetweenQueries: number;
}

const EXAMPLE =
  "select u.id, u.nombre, count(p.id) as pedidos from usuarios u left join pedidos p on p.usuario_id = u.id where u.activo = true group by u.id, u.nombre having count(p.id) > 0 order by pedidos desc limit 10;";

export function SqlPanel() {
  const [input, setInput] = useState(EXAMPLE);
  const [output, setOutput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [options, setOptions] = useState<SqlOptions>({
    indent: 2,
    uppercase: true,
    linesBetweenQueries: 1,
  });

  useEffect(() => {
    if (!input.trim()) {
      setOutput("");
      return;
    }
    let cancelled = false;
    invoke<string>("format_sql", { input, options })
      .then((result) => {
        if (cancelled) return;
        setOutput(result);
        setError(null);
      })
      .catch((cause) => {
        if (!cancelled) setError(ipcErrorMessage(cause));
      });
    return () => {
      cancelled = true;
    };
  }, [input, options]);

  return (
    <ToolShell
      error={error}
      notice="Los marcadores de parámetro (?, $1, :nombre) se conservan tal cual."
      toolbar={
        <>
          <Toggle
            label="Palabras clave en mayúsculas"
            checked={options.uppercase}
            onChange={(uppercase) =>
              setOptions((previous) => ({ ...previous, uppercase }))
            }
          />
          <NumberField
            label="Espacios"
            value={options.indent}
            min={1}
            max={8}
            onChange={(indent) => setOptions((previous) => ({ ...previous, indent }))}
          />
          <NumberField
            label="Líneas entre consultas"
            value={options.linesBetweenQueries}
            min={0}
            max={4}
            onChange={(linesBetweenQueries) =>
              setOptions((previous) => ({ ...previous, linesBetweenQueries }))
            }
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
        placeholder="Pega aquí tu consulta"
      />
    </ToolShell>
  );
}
