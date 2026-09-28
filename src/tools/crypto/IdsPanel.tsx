import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

import { Editor } from "../../components/Editor";
import { CopyButton, Pane, ToolShell } from "../../components/Panels";
import { Banner, Button, NumberField, Select, Toggle } from "../../components/ui";
import { ipcErrorMessage } from "../../lib/ipc";

interface IdKindInfo {
  id: string;
  label: string;
  note: string;
}

export function IdsPanel() {
  const [kinds, setKinds] = useState<IdKindInfo[]>([]);
  const [kind, setKind] = useState("uuidv7");
  const [count, setCount] = useState(10);
  const [uppercase, setUppercase] = useState(false);
  const [ids, setIds] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<IdKindInfo[]>("list_id_kinds")
      .then(setKinds)
      .catch(() => setKinds([]));
  }, []);

  const generate = useCallback(() => {
    invoke<string[]>("generate_ids", { kind, count, uppercase })
      .then((value) => {
        setIds(value);
        setError(null);
      })
      .catch((cause) => setError(ipcErrorMessage(cause)));
  }, [kind, count, uppercase]);

  useEffect(generate, [generate]);

  const selected = kinds.find((candidate) => candidate.id === kind);
  const text = ids.join("\n");

  return (
    <ToolShell
      error={error}
      notice={selected?.note ?? null}
      toolbar={
        <>
          <Select
            label="Tipo"
            value={kind}
            onChange={(event) => setKind(event.target.value)}
          >
            {kinds.map((candidate) => (
              <option key={candidate.id} value={candidate.id}>
                {candidate.label}
              </option>
            ))}
          </Select>
          <NumberField
            label="Cantidad"
            value={count}
            min={1}
            max={10000}
            onChange={setCount}
          />
          <Toggle label="MAYÚSCULAS" checked={uppercase} onChange={setUppercase} />
          <div className="ml-auto flex items-center gap-2">
            <Button onClick={generate}>Generar otra tanda</Button>
            <CopyButton text={text} label={`Copiar ${ids.length}`} />
          </div>
        </>
      }
    >
      <Pane label={`Identificadores · ${ids.length}`}>
        <Editor value={text} format="json" readOnly />
      </Pane>
    </ToolShell>
  );
}

export function PasswordGenPanel() {
  const [options, setOptions] = useState({
    length: 20,
    count: 5,
    lowercase: true,
    uppercase: true,
    digits: true,
    symbols: true,
    avoidAmbiguous: false,
  });
  const [result, setResult] = useState<{
    passwords: string[];
    entropyBits: number;
    alphabetSize: number;
    strength: string;
    note: string;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const generate = useCallback(() => {
    invoke<typeof result>("generate_passwords", { options })
      .then((value) => {
        setResult(value);
        setError(null);
      })
      .catch((cause) => {
        setError(ipcErrorMessage(cause));
        setResult(null);
      });
  }, [options]);

  useEffect(generate, [generate]);

  const set = <K extends keyof typeof options>(
    key: K,
    value: (typeof options)[K],
  ) => setOptions((previous) => ({ ...previous, [key]: value }));

  const toneForStrength =
    result?.strength === "débil" ? "error" : ("info" as const);

  return (
    <ToolShell
      error={error}
      toolbar={
        <>
          <NumberField
            label="Longitud"
            value={options.length}
            min={1}
            max={256}
            onChange={(length) => set("length", length)}
          />
          <NumberField
            label="Cantidad"
            value={options.count}
            min={1}
            max={500}
            onChange={(count) => set("count", count)}
          />
          <Toggle
            label="a-z"
            checked={options.lowercase}
            onChange={(value) => set("lowercase", value)}
          />
          <Toggle
            label="A-Z"
            checked={options.uppercase}
            onChange={(value) => set("uppercase", value)}
          />
          <Toggle
            label="0-9"
            checked={options.digits}
            onChange={(value) => set("digits", value)}
          />
          <Toggle
            label="!@#"
            checked={options.symbols}
            onChange={(value) => set("symbols", value)}
          />
          <Toggle
            label="Sin ambiguos"
            title="Excluye I, l, 1, O, 0 y similares, que se confunden al leer o dictar"
            checked={options.avoidAmbiguous}
            onChange={(value) => set("avoidAmbiguous", value)}
          />
          <div className="ml-auto flex items-center gap-2">
            <Button onClick={generate}>Generar otras</Button>
            <CopyButton
              text={result?.passwords.join("\n") ?? ""}
              label="Copiar todas"
            />
          </div>
        </>
      }
    >
      <div className="flex h-full flex-col gap-3 p-4">
        {result && (
          <Banner tone={toneForStrength}>
            <strong>{Math.round(result.entropyBits)} bits</strong> de entropía ·
            alfabeto de {result.alphabetSize} caracteres · {result.strength}.{" "}
            {result.note}
          </Banner>
        )}

        <ul className="min-h-0 flex-1 overflow-y-auto rounded-lg border border-ink-800">
          {result?.passwords.map((password, index) => (
            <li
              key={index}
              className="flex items-center gap-3 border-b border-ink-800 px-3 py-2 last:border-b-0 hover:bg-ink-900"
            >
              <span className="selectable min-w-0 flex-1 break-all font-mono text-sm text-mist-200">
                {password}
              </span>
              <CopyButton text={password} />
            </li>
          ))}
        </ul>
      </div>
    </ToolShell>
  );
}
