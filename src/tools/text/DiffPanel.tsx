import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

import { Editor } from "../../components/Editor";
import { Pane, ToolShell } from "../../components/Panels";
import { Select, Toggle } from "../../components/ui";
import { ipcErrorMessage } from "../../lib/ipc";

type ChangeKind = "equal" | "insert" | "delete";
type Granularity = "line" | "word" | "char";

interface DiffPiece {
  kind: ChangeKind;
  text: string;
}

interface DiffLine {
  kind: ChangeKind;
  leftNumber: number | null;
  rightNumber: number | null;
  pieces: DiffPiece[];
}

interface DiffResult {
  lines: DiffLine[];
  added: number;
  removed: number;
  identical: boolean;
}

interface DiffOptions {
  granularity: Granularity;
  ignoreCase: boolean;
  ignoreWhitespace: boolean;
}

export function DiffPanel() {
  const [left, setLeft] = useState("");
  const [right, setRight] = useState("");
  const [result, setResult] = useState<DiffResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [options, setOptions] = useState<DiffOptions>({
    granularity: "word",
    ignoreCase: false,
    ignoreWhitespace: false,
  });

  useEffect(() => {
    if (!left && !right) {
      setResult(null);
      return;
    }
    let cancelled = false;
    invoke<DiffResult>("diff_text", { left, right, options })
      .then((value) => {
        if (cancelled) return;
        setResult(value);
        setError(null);
      })
      .catch((cause) => {
        if (!cancelled) setError(ipcErrorMessage(cause));
      });
    return () => {
      cancelled = true;
    };
  }, [left, right, options]);

  const summary = result
    ? result.identical
      ? "Los dos textos son idénticos"
      : `${result.added} línea${result.added === 1 ? "" : "s"} añadida${
          result.added === 1 ? "" : "s"
        } · ${result.removed} eliminada${result.removed === 1 ? "" : "s"}`
    : null;

  return (
    <ToolShell
      error={error}
      notice={summary}
      toolbar={
        <>
          <Select
            label="Resaltar"
            value={options.granularity}
            onChange={(event) =>
              setOptions((previous) => ({
                ...previous,
                granularity: event.target.value as Granularity,
              }))
            }
          >
            <option value="word">Por palabra</option>
            <option value="char">Por carácter</option>
            <option value="line">Solo la línea</option>
          </Select>
          <Toggle
            label="Ignorar mayúsculas"
            checked={options.ignoreCase}
            onChange={(ignoreCase) =>
              setOptions((previous) => ({ ...previous, ignoreCase }))
            }
          />
          <Toggle
            label="Ignorar espacios"
            checked={options.ignoreWhitespace}
            onChange={(ignoreWhitespace) =>
              setOptions((previous) => ({ ...previous, ignoreWhitespace }))
            }
          />
        </>
      }
    >
      <div className="grid h-full grid-rows-2 gap-px bg-ink-800">
        <div className="grid min-h-0 grid-cols-2 gap-px bg-ink-800">
          <Pane label="Original">
            <Editor value={left} format="json" onChange={setLeft} placeholder="Texto original" />
          </Pane>
          <Pane label="Modificado">
            <Editor
              value={right}
              format="json"
              onChange={setRight}
              placeholder="Texto modificado"
            />
          </Pane>
        </div>

        <Pane label="Diferencias">
          {result ? (
            <DiffView lines={result.lines} />
          ) : (
            <div className="flex h-full items-center justify-center">
              <p className="text-xs text-mist-500">
                Pega los dos textos para compararlos
              </p>
            </div>
          )}
        </Pane>
      </div>
    </ToolShell>
  );
}

function DiffView({ lines }: { lines: DiffLine[] }) {
  return (
    <div className="selectable h-full overflow-auto font-mono text-xs leading-relaxed">
      {lines.map((line, index) => (
        <div
          key={index}
          className={`flex gap-0 px-2 ${
            line.kind === "insert"
              ? "bg-emerald-500/10"
              : line.kind === "delete"
                ? "bg-danger-400/10"
                : ""
          }`}
        >
          <Gutter value={line.leftNumber} />
          <Gutter value={line.rightNumber} />
          <span
            className={`w-4 shrink-0 select-none text-center ${
              line.kind === "insert"
                ? "text-emerald-400"
                : line.kind === "delete"
                  ? "text-danger-400"
                  : "text-ink-600"
            }`}
          >
            {line.kind === "insert" ? "+" : line.kind === "delete" ? "−" : " "}
          </span>
          <span className="min-w-0 flex-1 whitespace-pre-wrap break-all">
            {line.pieces.map((piece, pieceIndex) => (
              <span
                key={pieceIndex}
                className={
                  piece.kind === "insert"
                    ? "rounded bg-emerald-500/30 text-emerald-200"
                    : piece.kind === "delete"
                      ? "rounded bg-danger-400/30 text-red-200"
                      : "text-mist-400"
                }
              >
                {piece.text}
              </span>
            ))}
          </span>
        </div>
      ))}
    </div>
  );
}

function Gutter({ value }: { value: number | null }) {
  return (
    <span className="w-10 shrink-0 select-none pr-2 text-right text-ink-600">
      {value ?? ""}
    </span>
  );
}
