import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useState } from "react";

import { Editor } from "../../components/Editor";
import { CopyButton, FieldRow, Pane, ToolShell } from "../../components/Panels";
import { Banner, Button, Select, TextField } from "../../components/ui";
import { baseName } from "../../lib/convert";
import { ipcErrorMessage } from "../../lib/ipc";

interface AlgorithmInfo {
  id: string;
  label: string;
  insecureNote: string | null;
}

interface HashResult {
  algorithm: string;
  label: string;
  digest: string;
}

interface FileHashResult {
  path: string;
  bytes: number;
  hashes: HashResult[];
}

const DEFAULT_SELECTION = ["sha256"];

function useAlgorithms() {
  const [algorithms, setAlgorithms] = useState<AlgorithmInfo[]>([]);
  useEffect(() => {
    invoke<AlgorithmInfo[]>("list_hash_algorithms")
      .then(setAlgorithms)
      .catch(() => setAlgorithms([]));
  }, []);
  return algorithms;
}

function AlgorithmPicker({
  algorithms,
  selected,
  onToggle,
}: {
  algorithms: AlgorithmInfo[];
  selected: string[];
  onToggle: (id: string) => void;
}) {
  return (
    <div className="flex flex-wrap gap-1.5">
      {algorithms.map((algorithm) => (
        <button
          key={algorithm.id}
          type="button"
          onClick={() => onToggle(algorithm.id)}
          title={algorithm.insecureNote ?? undefined}
          className={`rounded-md px-2 py-1 font-mono text-xs transition-colors ${
            selected.includes(algorithm.id)
              ? "bg-bolt-500/15 text-bolt-400"
              : "border border-ink-700 text-mist-500 hover:text-mist-200"
          }`}
        >
          {algorithm.label}
          {algorithm.insecureNote && <span className="ml-1 text-warn-400">!</span>}
        </button>
      ))}
    </div>
  );
}

/** Avisos de los algoritmos rotos que el usuario tenga seleccionados. */
function InsecureWarnings({
  algorithms,
  selected,
}: {
  algorithms: AlgorithmInfo[];
  selected: string[];
}) {
  const warnings = algorithms.filter(
    (algorithm) => selected.includes(algorithm.id) && algorithm.insecureNote,
  );
  if (warnings.length === 0) return null;

  return (
    <div className="space-y-2 px-4 pt-3">
      {warnings.map((algorithm) => (
        <Banner key={algorithm.id} tone="warn">
          <strong>{algorithm.label}</strong> — {algorithm.insecureNote}
        </Banner>
      ))}
    </div>
  );
}

export function HashTextPanel() {
  const algorithms = useAlgorithms();
  const [selected, setSelected] = useState<string[]>(DEFAULT_SELECTION);
  const [input, setInput] = useState("");
  const [results, setResults] = useState<HashResult[]>([]);
  const [error, setError] = useState<string | null>(null);

  const toggle = (id: string) =>
    setSelected((previous) =>
      previous.includes(id)
        ? previous.filter((item) => item !== id)
        : [...previous, id],
    );

  useEffect(() => {
    if (selected.length === 0) {
      setResults([]);
      return;
    }
    let cancelled = false;
    invoke<HashResult[]>("hash_text", { input, algorithms: selected })
      .then((value) => {
        if (cancelled) return;
        setResults(value);
        setError(null);
      })
      .catch((cause) => {
        if (!cancelled) setError(ipcErrorMessage(cause));
      });
    return () => {
      cancelled = true;
    };
  }, [input, selected]);

  return (
    <ToolShell
      error={error}
      toolbar={
        <AlgorithmPicker
          algorithms={algorithms}
          selected={selected}
          onToggle={toggle}
        />
      }
    >
      <InsecureWarnings algorithms={algorithms} selected={selected} />
      <div className="grid h-full grid-cols-2 gap-px bg-ink-800">
        <Pane label="Texto">
          <Editor
            value={input}
            format="json"
            onChange={setInput}
            placeholder="El resumen se calcula sobre los bytes UTF-8 de este texto"
          />
        </Pane>
        <Pane label="Resúmenes">
          <div className="overflow-y-auto">
            {results.map((result) => (
              <FieldRow
                key={result.algorithm}
                label={result.label}
                value={
                  <span className="flex items-center gap-2">
                    <span className="min-w-0 flex-1">{result.digest}</span>
                    <CopyButton text={result.digest} />
                  </span>
                }
              />
            ))}
            {selected.length === 0 && (
              <p className="p-4 text-xs text-mist-500">
                Elige al menos un algoritmo arriba.
              </p>
            )}
          </div>
        </Pane>
      </div>
    </ToolShell>
  );
}

export function HashFilePanel() {
  const algorithms = useAlgorithms();
  const [selected, setSelected] = useState<string[]>(DEFAULT_SELECTION);
  const [result, setResult] = useState<FileHashResult | null>(null);
  const [expected, setExpected] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);

  const toggle = (id: string) =>
    setSelected((previous) =>
      previous.includes(id)
        ? previous.filter((item) => item !== id)
        : [...previous, id],
    );

  const pickFile = useCallback(async () => {
    const path = await open({ multiple: false });
    if (typeof path !== "string") return;

    setRunning(true);
    setError(null);
    try {
      setResult(await invoke<FileHashResult>("hash_file", { path, algorithms: selected }));
    } catch (cause) {
      setError(ipcErrorMessage(cause));
      setResult(null);
    } finally {
      setRunning(false);
    }
  }, [selected]);

  // La comparación se hace aquí y no en Rust porque los dos valores ya están
  // en la interfaz; no hace falta volver a leer el archivo para compararlos.
  const comparison = (() => {
    const wanted = expected.trim().toLowerCase();
    if (!wanted || !result) return null;
    const hit = result.hashes.find((hash) => hash.digest === wanted);
    if (hit) return { ok: true, message: `Coincide con el ${hit.label} calculado.` };
    return {
      ok: false,
      message:
        "No coincide con ninguno de los resúmenes calculados. Comprueba que has " +
        "marcado el algoritmo correcto y que la descarga está completa.",
    };
  })();

  return (
    <ToolShell
      error={error}
      toolbar={
        <>
          <AlgorithmPicker
            algorithms={algorithms}
            selected={selected}
            onToggle={toggle}
          />
          <div className="ml-auto">
            <Button variant="primary" onClick={pickFile} disabled={running}>
              {running ? "Calculando…" : "Elegir archivo"}
            </Button>
          </div>
        </>
      }
    >
      <InsecureWarnings algorithms={algorithms} selected={selected} />

      <div className="space-y-4 overflow-y-auto p-4">
        {!result && (
          <p className="text-sm text-mist-500">
            Elige un archivo para calcular sus sumas de comprobación. Los archivos
            se leen por bloques, así que el tamaño no importa.
          </p>
        )}

        {result && (
          <>
            <div className="rounded-lg border border-ink-800">
              <FieldRow label="Archivo" value={baseName(result.path)} />
              <FieldRow
                label="Tamaño"
                value={`${result.bytes.toLocaleString("es")} bytes`}
                tone="muted"
              />
              {result.hashes.map((hash) => (
                <FieldRow
                  key={hash.algorithm}
                  label={hash.label}
                  value={
                    <span className="flex items-center gap-2">
                      <span className="min-w-0 flex-1">{hash.digest}</span>
                      <CopyButton text={hash.digest} />
                    </span>
                  }
                />
              ))}
            </div>

            <div className="flex items-center gap-3">
              <TextField
                label="Suma publicada"
                width="w-[34rem]"
                title="Pega aquí la suma que aparece en la página de descarga"
                value={expected}
                onChange={setExpected}
              />
            </div>

            {comparison && (
              <Banner tone={comparison.ok ? "info" : "error"}>
                {comparison.message}
              </Banner>
            )}
          </>
        )}
      </div>
    </ToolShell>
  );
}

export function HmacPanel() {
  const [input, setInput] = useState("");
  const [key, setKey] = useState("");
  const [algorithm, setAlgorithm] = useState("sha256");
  const [output, setOutput] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!input && !key) {
      setOutput("");
      return;
    }
    let cancelled = false;
    invoke<string>("hmac_text", { input, key, algorithm })
      .then((value) => {
        if (cancelled) return;
        setOutput(value);
        setError(null);
      })
      .catch((cause) => {
        if (!cancelled) setError(ipcErrorMessage(cause));
      });
    return () => {
      cancelled = true;
    };
  }, [input, key, algorithm]);

  return (
    <ToolShell
      error={error}
      notice="Para comparar dos HMAC, usa siempre una comparación en tiempo constante; hacerlo con «==» filtra la firma byte a byte."
      toolbar={
        <>
          <Select
            label="Algoritmo"
            value={algorithm}
            onChange={(event) => setAlgorithm(event.target.value)}
          >
            <option value="sha256">HMAC-SHA-256</option>
            <option value="sha384">HMAC-SHA-384</option>
            <option value="sha512">HMAC-SHA-512</option>
          </Select>
          <TextField label="Clave" width="w-64" value={key} onChange={setKey} />
          <div className="ml-auto">
            <CopyButton text={output} />
          </div>
        </>
      }
    >
      <div className="grid h-full grid-cols-2 gap-px bg-ink-800">
        <Pane label="Mensaje">
          <Editor value={input} format="json" onChange={setInput} placeholder="Mensaje a firmar" />
        </Pane>
        <Pane label="HMAC">
          <div className="selectable break-all p-4 font-mono text-sm text-bolt-400">
            {output || (
              <span className="text-mist-500">
                Escribe un mensaje y una clave.
              </span>
            )}
          </div>
        </Pane>
      </div>
    </ToolShell>
  );
}
