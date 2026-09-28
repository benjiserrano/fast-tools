import { useMemo, useState } from "react";

import { Editor } from "../../components/Editor";
import { CopyButton, Pane, ToolShell } from "../../components/Panels";
import { diffLists, type ListDiffResult } from "../../lib/web-tools/listDiff";

const EMPTY_RESULT: ListDiffResult = { aOnly: [], bOnly: [], both: [], all: [] };

export function ListDiffPanel() {
  const [a, setA] = useState("");
  const [b, setB] = useState("");
  const result = useMemo(
    () => (a || b ? diffLists(a, b) : EMPTY_RESULT),
    [a, b],
  );

  return (
    <ToolShell
      notice={
        a || b
          ? `${result.all.length} elementos distintos · ${result.both.length} en ambas listas`
          : null
      }
      toolbar={null}
    >
      <div className="grid h-full grid-rows-[minmax(0,1fr)_minmax(0,1fr)] gap-px bg-ink-800">
        <div className="grid min-h-0 grid-cols-2 gap-px bg-ink-800">
          <Pane label="A">
            <Editor value={a} format="csv" onChange={setA} placeholder="Un elemento por línea" />
          </Pane>
          <Pane label="B">
            <Editor value={b} format="csv" onChange={setB} placeholder="Un elemento por línea" />
          </Pane>
        </div>

        <div className="grid min-h-0 grid-cols-2 gap-px bg-ink-800">
          <ResultPane label="A only" items={result.aOnly} tone="text-danger-400" />
          <ResultPane label="B only" items={result.bOnly} tone="text-warn-400" />
          <ResultPane label="A - B (In both)" items={result.both} tone="text-emerald-400" />
          <ResultPane label="A - B (All items)" items={result.all} tone="text-bolt-400" />
        </div>
      </div>
    </ToolShell>
  );
}

function ResultPane({
  label,
  items,
  tone,
}: {
  label: string;
  items: string[];
  tone: string;
}) {
  const output = items.join("\n");

  return (
    <Pane label={`${label} · ${items.length}`} actions={<CopyButton text={output} />}>
      <div className={`selectable h-full overflow-auto whitespace-pre-wrap break-all p-4 font-mono text-xs leading-relaxed ${tone}`}>
        {output || <span className="text-mist-600">Sin elementos</span>}
      </div>
    </Pane>
  );
}
